extern crate block_utils;
extern crate ceph;
extern crate ceph_safe_disk;
extern crate dirs;
extern crate fstab;
extern crate helpers;
extern crate init_daemon;
extern crate libc;
extern crate serde_json;
extern crate tempdir;
extern crate uuid;

use std::fs::{create_dir, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use backend::Backend;

use self::ceph::ceph::{connect_to_ceph, Rados};
use self::ceph::cmd::*;
use self::ceph_safe_disk::diag::{DiagMap, Format, Status};
use self::dirs::home_dir;
use self::fstab::FsTab;
use self::helpers::{error::*, host_information::Host};
use self::init_daemon::{detect_daemon, Daemon};
use self::tempdir::TempDir;

/// Ceph cluster
pub struct CephBackend {
    cluster_handle: Rados,
}

#[derive(Deserialize, Debug)]
struct CephConfig {
    /// The location of the ceph.conf file
    config_file: String,
    /// The cephx user to connect to the Ceph service with
    user_id: String,
}

fn choose_ceph_config(config_dir: Option<&Path>) -> BynarResult<PathBuf> {
    match config_dir {
        Some(config) => {
            let json_path = config.join("ceph.json");
            if !json_path.exists() {
                let err_msg = format!("{} does not exist.  Please create", json_path.display());
                error!("{}", err_msg);
                return Err(BynarError::new(err_msg));
            }
            debug!("Loading ceph config from: {}", json_path.display(),);
            Ok(json_path)
        }
        None => {
            let home = home_dir().expect("HOME env variable not defined");
            let json_path = PathBuf::from(home).join(".config").join("ceph.json");
            if !json_path.exists() {
                let err_msg = format!("{} does not exist.  Please create", json_path.display());
                error!("{}", err_msg);
                return Err(BynarError::new(err_msg));
            }
            info!("Reading ceph config file: {}", json_path.display(),);
            Ok(json_path)
        }
    }
}

impl CephBackend {
    pub fn new(config_dir: Option<&Path>) -> BynarResult<CephBackend> {
        let ceph_config = choose_ceph_config(config_dir)?;
        if !ceph_config.exists() {
            error!("ceph config {} does not exist", ceph_config.display());
        }
        let s = read_to_string(ceph_config)?;
        let deserialized: CephConfig = serde_json::from_str(&s)?;

        info!("Connecting to Ceph");
        let cluster_handle = connect_to_ceph(&deserialized.user_id, &deserialized.config_file)?;
        info!("Connected to Ceph");
        Ok(CephBackend {
            cluster_handle: cluster_handle,
        })
    }

    /// Add a new /dev/ path as an osd.
    fn add_osd(&self, dev_path: &Path, id: Option<u64>, simulate: bool) -> BynarResult<()> {
        //Format the drive
        let xfs_options = block_utils::Filesystem::Xfs {
            stripe_size: None,
            stripe_width: None,
            block_size: None,
            agcount: Some(32),
            inode_size: Some(2048),
            force: true,
        };
        debug!(
            "Formatting {:?} with XFS options: {:?}",
            dev_path, xfs_options
        );
        if !simulate {
            block_utils::format_block_device(dev_path, &xfs_options)?;
            let _ = settle_udev();
        }

        // Probe the drive
        debug!("udev Probing device {:?}", dev_path);
        let info = block_utils::get_device_info(dev_path)?;
        debug!("udev info {:?}", info);
        if info.id.is_none() {
            return Err(BynarError::new(format!(
                "Formatted device {:?} doesn't have a filesystem UUID.  Please investigate",
                dev_path
            )));
        }

        // Create a new osd id
        let new_osd_id = osd_create(&self.cluster_handle, id, simulate)?;
        debug!("New osd id created: {:?}", new_osd_id);

        // Mount the drive
        let mount_point = format!("/var/lib/ceph/osd/ceph-{}", new_osd_id);
        if !simulate {
            if !Path::new(&mount_point).exists() {
                debug!("Mount point {} doesn't exist.  Creating.", mount_point);
                create_dir(&mount_point)?;
            }
            block_utils::mount_device(&info, &mount_point)?;
        }

        // Format the osd with the osd filesystem
        ceph_mkfs(new_osd_id, None, simulate)?;
        debug!("Creating ceph authorization entry");
        osd_auth_add(&self.cluster_handle, new_osd_id, simulate)?;
        let auth_key = auth_get_key(&self.cluster_handle, "osd", &new_osd_id.to_string())?;
        debug!("Saving ceph keyring");
        save_keyring(new_osd_id, &auth_key, simulate)?;
        let host_info = Host::new()?;
        let gb_capacity = info.capacity / 1073741824;
        let osd_weight = gb_capacity as f64 * 0.001_f64;
        debug!(
            "Adding OSD {} to crushmap under host {} with weight: {}",
            new_osd_id, host_info.hostname, osd_weight
        );
        osd_crush_add(
            &self.cluster_handle,
            new_osd_id,
            osd_weight,
            &host_info.hostname,
            simulate,
        )?;
        add_osd_to_fstab(&info, new_osd_id, simulate)?;
        // This step depends on whether it's systemctl, upstart, etc
        setup_osd_init(new_osd_id, simulate)?;
        // sudo start ceph-osd id={osd-num}
        Ok(())
    }

    fn remove_osd(&self, dev_path: &Path, simulate: bool) -> BynarResult<()> {
        //If the OSD is still running we can query its version.  If not then we
        //should ask either another OSD or a monitor.
        let mount_point = match block_utils::get_mountpoint(&dev_path)? {
            Some(osd_path) => osd_path,
            None => {
                let temp_dir = TempDir::new("osd")?;
                temp_dir.into_path()
            }
        };
        debug!("OSD mounted at: {:?}", mount_point);

        let osd_id = match get_osd_id(&mount_point, simulate) {
            Ok(osd_id) => osd_id,
            Err(e) => {
                error!(
                    "Failed to discover osd id: {:?}.  Falling back on path name",
                    e
                );
                get_osd_id_from_path(&mount_point)?
            }
        };
        debug!("Setting osd {} out", osd_id);
        osd_out(&self.cluster_handle, osd_id, simulate)?;
        debug!("Removing osd {} from crush", osd_id);
        osd_crush_remove(&self.cluster_handle, osd_id, simulate)?;
        debug!("Deleting osd {} auth key", osd_id);
        auth_del(&self.cluster_handle, osd_id, simulate)?;
        debug!("Removing osd {}", osd_id);
        osd_rm(&self.cluster_handle, osd_id, simulate)?;

        // Wipe the disk
        debug!("Erasing disk {}", dev_path.display());
        if !simulate {
            match block_utils::erase_block_device(&dev_path) {
                Ok(_) => {
                    debug!("{} erased", dev_path.display());
                }
                Err(e) => {
                    // At this point the disk is about to be replaced anyways
                    // so this doesn't really matter
                    error!("{} failed to erase: {:?}", dev_path.display(), e);
                }
            };
        }

        Ok(())
    }
}

impl Backend for CephBackend {
    fn add_disk(
        &self,
        device: &Path,
        id: Option<u64>,
        journal: Option<&str>,
        journal_partition: Option<u32>,
        simulate: bool,
    ) -> BynarResult<()> {
        self.add_osd(device, id, simulate)?;
        Ok(())
    }
    fn remove_disk(&self, device: &Path, simulate: bool) -> BynarResult<()> {
        self.remove_osd(device, simulate)?;
        Ok(())
    }

    fn safe_to_remove(&self, device: &Path, simulate: bool) -> BynarResult<bool> {
        let diag_map = DiagMap::new().map_err(|e| BynarError::new(e.to_string()))?;
        debug!("Checking if a disk is safe to remove from ceph");
        match diag_map.exhaustive_diag(Format::Json) {
            Status::Safe => return Ok(true),
            Status::NonSafe => return Ok(false),
            Status::Unknown => return Ok(false),
        };
    }
}

// A fallback function to get the osd id from the mount path.  This isn't
// 100% accurate but it should be good enough for most cases unless the disk
// is mounted in the wrong location or is missing an osd id in the path name
fn get_osd_id_from_path(path: &Path) -> BynarResult<u64> {
    match path.file_name() {
        Some(name) => {
            let name_string = name.to_string_lossy().into_owned();
            let parts: Vec<&str> = name_string.split("-").collect();
            let id = u64::from_str(parts[1])?;
            Ok(id)
        }
        None => Err(BynarError::new(format!("Unable to get filename from {}", path.display()))),
    }
}

// Get an osd ID from the whoami file in the osd mount directory
fn get_osd_id(path: &Path, simulate: bool) -> BynarResult<u64> {
    if simulate {
        return Ok(0);
    }
    let whoami_path = path.join("whoami");
    debug!("Discovering osd id number from: {}", whoami_path.display());
    let buff = read_to_string(&whoami_path)?;
    u64::from_str(buff.trim()).map_err(|e| BynarError::ParseIntError(e))
}

fn save_keyring(osd_id: u64, key: &str, simulate: bool) -> BynarResult<()> {
    let base_dir = format!("/var/lib/ceph/osd/ceph-{}", osd_id);
    if !Path::new(&base_dir).exists() {
        return Err(BynarError::new(
            format!("{} directory doesn't exist", base_dir),
        ));
    }
    debug!("Creating {}/keyring", base_dir);
    if !simulate {
        let mut f = File::create(format!("{}/keyring", base_dir))?;
        f.write_all(format!("[osd.{}]\n\tkey = {}\n", osd_id, key).as_bytes())?;
    }
    Ok(())
}

fn add_osd_to_fstab(
    device_info: &block_utils::Device,
    osd_id: u64,
    simulate: bool,
) -> BynarResult<()> {
    let fstab = FsTab::default();
    let fstab_entry = fstab::FsEntry {
        fs_spec: format!(
            "UUID={}",
            device_info.id.unwrap().to_hyphenated().to_string()
        ),
        mountpoint: PathBuf::from(&format!("/var/lib/ceph/osd/ceph-{}", osd_id)),
        vfs_type: device_info.fs_type.to_string(),
        mount_options: vec![
            "noatime".into(),
            "inode64".into(),
            "attr2".into(),
            "logbsize=256k".into(),
            "noquota".into(),
        ],
        dump: false,
        fsck_order: 2,
    };
    debug!("Saving Fstab entry {:?}", fstab_entry);
    if !simulate {
        let result = fstab.add_entry(fstab_entry)?;
        match result {
            true => debug!("Fstab entry saved"),
            false => debug!("Fstab entry was updated"),
        };
    }
    Ok(())
}

fn setup_osd_init(osd_id: u64, simulate: bool) -> BynarResult<()> {
    debug!("Detecting init system");
    let init_daemon = detect_daemon()?;
    match init_daemon {
        Daemon::Systemd => {
            debug!("Systemd detected.  Starting OSD");
            let mut cmd = Command::new("systemctl");
            cmd.arg("start");
            cmd.arg(format!("ceph-osd@{}", osd_id));
            debug!("cmd: {:?}", cmd);
            if !simulate {
                let output = cmd.output()?;
                if !output.status.success() {
                    return Err(BynarError::new(String::from_utf8_lossy(&output.stderr).into_owned()));
                }
            }
            return Ok(());
        }
        Daemon::Upstart => {
            debug!("Upstart detected.  Starting OSD");
            let mut cmd = Command::new("start");
            cmd.arg("ceph-osd");
            cmd.arg(format!("id={}", osd_id));
            debug!("cmd: {:?}", cmd);
            if !simulate {
                let output = cmd.output()?;
                if !output.status.success() {
                    return Err(BynarError::new(String::from_utf8_lossy(&output.stderr).into_owned()));
                }
            }
            return Ok(());
        }
        Daemon::Unknown => {
            return Err(BynarError::new(
                "Unknown init system.  Cannot start osd service".to_string(),
            ));
        }
    };
}

fn settle_udev() -> BynarResult<()> {
    let output = Command::new("udevadm").arg("settle").output()?;
    if !output.status.success() {
        return Err(BynarError::new(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(())
}

// Run ceph-osd --mkfs and return the osd UUID
fn ceph_mkfs(osd_id: u64, journal: Option<&Path>, simulate: bool) -> BynarResult<()> {
    debug!("Running ceph-osd --mkfs");
    let journal_str: String;
    let osd_id_str = osd_id.to_string();

    let mut args: Vec<&str> = vec!["--cluster", "ceph", "-i", &osd_id_str, "--mkfs"];
    if let Some(journal_path) = journal {
        journal_str = journal_path.to_string_lossy().into_owned();
        args.push("--journal");
        args.push(&journal_str);
    }
    debug!("cmd: ceph-osd {:?}", args);
    if simulate {
        return Ok(());
    }
    Command::new("ceph-osd").args(&args).output()?;
    Ok(())
}

// Add osds with xfs
// Jewel or earlier
fn add_filestore_osd(dev_path: &Path) -> BynarResult<()> {
    //
    Ok(())
}

// Add osds with bluestore
// Luminous or later
// TODO: Research bluestore creation
// SO far it looks like ceph-disk prepare makes a 100MB xfs partition
// and makes a symlink to the block devices by partition_type
// block -> /dev/disk/by-partuuid/71b11da1-70b8-4ab5-ba97-036062e6f061
// cat /var/lib/ceph/tmp/mnt.sYq7No/block_uuid
// 71b11da1-70b8-4ab5-ba97-036062e6f061
// cat /var/lib/ceph/tmp/mnt.sYq7No/ceph_fsid
// c8bb8cb4-6dda-4a8e-9f14-3e5a8d451cf4
// root@server:~# cat /var/lib/ceph/tmp/mnt.sYq7No/magic
// ceph osd volume v026
// cat /var/lib/ceph/tmp/mnt.sYq7No/type
// bluestore
// cat /var/lib/ceph/tmp/mnt.sYq7No/fsid
// a848a7ba-e1c1-4df2-aef5-58895d77895a
fn add_bluestore_osd(dev_path: &Path) -> BynarResult<()> {
    //
    Ok(())
}
