extern crate block_utils;
extern crate ceph_rust;
extern crate ceph_safe_disk;
extern crate fstab;
extern crate init_daemon;
extern crate libc;
extern crate mktemp;
extern crate serde_json;
extern crate uuid;

use std::env::home_dir;
use std::fs::{create_dir, File};
use std::io::{Error, ErrorKind, Read, Write};
use std::io::Result as IOResult;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use backend::Backend;

use self::ceph_rust::ceph::{connect_to_ceph, ceph_mon_command_without_data, disconnect_from_ceph};
use self::ceph_rust::rados::rados_t;
use self::fstab::FsTab;
use self::init_daemon::{detect_daemon, Daemon};
use self::mktemp::Temp;
use self::uuid::Uuid;
use super::super::host_information::Host;

/// Ceph cluster
pub struct CephBackend {
    cluster_handle: rados_t,
}

#[derive(Deserialize, Debug)]
struct CephConfig {
    /// The location of the ceph.conf file
    config_file: String,
    /// The cephx user to connect to the Ceph service with
    user_id: String,
}

fn choose_ceph_config(config_dir: Option<&Path>) -> IOResult<PathBuf> {
    match config_dir {
        Some(config) => {
            let mut json_path = config.to_path_buf();
            json_path.push("ceph.json");
            if !json_path.exists() {
                let err_msg = format!("{} does not exist.  Please create", json_path.display());
                error!("{}", err_msg);
                return Err(Error::new(ErrorKind::NotFound, err_msg));
            }
            debug!(
                "Loading ceph config from: {}",
                json_path.display(),
            );
            Ok(json_path)
        }
        None => {
            let home = home_dir().expect("HOME env variable not defined");
            let mut json_path = PathBuf::from(home);
            json_path.push(".config");
            json_path.push("ceph.json");
            if !json_path.exists() {
                let err_msg = format!("{} does not exist.  Please create", json_path.display());
                error!("{}", err_msg);
                return Err(Error::new(ErrorKind::NotFound, err_msg));
            }
            info!(
                "Reading ceph config file: {}",
                json_path.display(),
            );
            Ok(json_path)
        }
    }
}

impl CephBackend {
    pub fn new(config_dir: Option<&Path>) -> IOResult<CephBackend> {
        let ceph_config = choose_ceph_config(config_dir)?;
        let mut f = File::open(ceph_config)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        let deserialized: CephConfig = serde_json::from_str(&s)?;

        info!("Connecting to Ceph");
        let cluster_handle = connect_to_ceph(&deserialized.user_id, &deserialized.config_file)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        info!("Connected to ceph");
        Ok(CephBackend { cluster_handle: cluster_handle })
    }

    /// Add a new /dev/ path as an osd.
    fn add_osd(&self, dev_path: &Path, simulate: bool) -> Result<(), String> {
        //Format the drive
        let xfs_options = block_utils::Filesystem::Xfs {
            stripe_size: None,
            stripe_width: None,
            block_size: None,
            inode_size: Some(2048),
            force: true,
        };
        debug!(
            "Formatting {:?} with XFS options: {:?}",
            dev_path,
            xfs_options
        );
        if !simulate {
            block_utils::format_block_device(dev_path, &xfs_options)?;
            settle_udev();
        }

        // Probe the drive
        debug!("udev Probing device {:?}", dev_path);
        let info = block_utils::get_device_info(dev_path)?;
        debug!("udev info {:?}", info);
        if info.id.is_none() {
            return Err(format!(
                "Formatted device {:?} doesn't have a filesystem UUID.  Please investigate",
                dev_path
            ));
        }

        // Create a new osd id
        let new_osd_id = osd_create(self.cluster_handle, simulate)?;
        debug!("New osd id created: {:?}", new_osd_id);

        // Mount the drive
        let mount_point = format!("/var/lib/ceph/osd/ceph-{}", new_osd_id);
        if !simulate {
            if !Path::new(&mount_point).exists() {
                debug!("Mount point {} doesn't exist.  Creating.", mount_point);
                create_dir(&mount_point).map_err(|e| e.to_string())?;
            }
            block_utils::mount_device(&info, &mount_point)?;
        }

        // Format the osd with the osd filesystem
        ceph_mkfs(new_osd_id, None, simulate)?;
        debug!("Creating ceph authorization entry");
        auth_add(self.cluster_handle, new_osd_id, simulate)?;
        let auth_key = auth_get_key(self.cluster_handle, new_osd_id, simulate)?;
        debug!("Saving ceph keyring");
        save_keyring(new_osd_id, &auth_key, simulate).map_err(|e| {
            e.to_string()
        })?;
        let host_info = Host::new().map_err(|e| e.to_string())?;
        let gb_capacity = info.capacity / 1073741824;
        let osd_weight = gb_capacity as f64 * 0.001_f64;
        debug!(
            "Adding OSD {} to crushmap under host {} with weight: {}",
            new_osd_id,
            host_info.hostname,
            osd_weight
        );
        osd_crush_add(
            self.cluster_handle,
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

    fn remove_osd(&self, dev_path: &Path, simulate: bool) -> Result<(), String> {
        //If the OSD is still running we can query its version.  If not then we
        //should ask either another OSD or a monitor.
        let mount_point = match block_utils::get_mountpoint(&dev_path).map_err(
            |e| e.to_string(),
        )? {
            Some(osd_path) => osd_path,
            None => {
                let temp_dir = Temp::new_dir().map_err(|e| e.to_string())?;
                temp_dir.to_path_buf()
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
        osd_out(self.cluster_handle, osd_id, simulate)?;
        debug!("Removing osd {} from crush", osd_id);
        osd_crush_remove(self.cluster_handle, osd_id, simulate)?;
        debug!("Deleting osd {} auth key", osd_id);
        auth_del(self.cluster_handle, osd_id, simulate)?;
        debug!("Removing osd {}", osd_id);
        osd_rm(self.cluster_handle, osd_id, simulate)?;

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
impl Drop for CephBackend {
    fn drop(&mut self) {
        disconnect_from_ceph(self.cluster_handle);
    }
}

impl Backend for CephBackend {
    fn add_disk(&self, device: &Path, simulate: bool) -> IOResult<()> {
        self.add_osd(device, simulate).map_err(|e| {
            Error::new(ErrorKind::Other, e)
        })?;
        Ok(())
    }
    fn remove_disk(&self, device: &Path, simulate: bool) -> IOResult<()> {
        self.remove_osd(device, simulate).map_err(|e| {
            Error::new(ErrorKind::Other, e)
        })?;
        Ok(())
    }
}

fn osd_out(cluster_handle: rados_t, osd_id: u64, simulate: bool) -> Result<(), String> {
    let cmd = json!({
        "prefix": "osd out",
        "ids": [osd_id.to_string()]
    });
    debug!("osd out: {:?}", cmd.to_string());

    if !simulate {
        ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn osd_crush_remove(cluster_handle: rados_t, osd_id: u64, simulate: bool) -> Result<(), String> {
    let cmd = json!({
        "prefix": "osd crush remove",
        "name": format!("osd.{}", osd_id),
    });
    debug!("osd crush remove: {:?}", cmd.to_string());
    if !simulate {
        ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn auth_del(cluster_handle: rados_t, osd_id: u64, simulate: bool) -> Result<(), String> {
    let cmd = json!({
        "prefix": "auth del",
        "entity": format!("osd.{}", osd_id)
    });
    debug!("auth del: {:?}", cmd.to_string());

    if !simulate {
        ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn osd_rm(cluster_handle: rados_t, osd_id: u64, simulate: bool) -> Result<(), String> {
    let cmd = json!({
        "prefix": "osd rm",
        "ids": [osd_id.to_string()]
    });
    debug!("osd rm: {:?}", cmd.to_string());

    if !simulate {
        ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())

}

fn osd_create(cluster_handle: rados_t, simulate: bool) -> Result<u64, String> {
    let cmd = json!({
            "prefix": "osd create",
        });
    debug!("osd create: {:?}", cmd.to_string());
    if simulate {
        return Ok(0);
    }

    let result = ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
        .map_err(|e| e.to_string())?;
    if result.0.is_some() {
        let return_data = result.0.unwrap();
        let mut l = return_data.lines();
        match l.next() {
            Some(num) => return Ok(u64::from_str(num).map_err(|e| e.to_string())?),
            None => {
                return Err(format!(
                "Unable to parse osd create output: {:?}",
                return_data,
            ))
            }
        }
    }
    Err(format!("Unable to parse osd create output: {:?}", result))
}

fn auth_add(cluster_handle: rados_t, osd_id: u64, simulate: bool) -> Result<(), String> {
    let cmd = json!({
        "prefix": "auth add",
        "entity": format!("osd.{}", osd_id),
        "caps": ["mon", "allow rwx", "osd", "allow *"],
    });
    debug!("auth_add: {:?}", cmd.to_string());

    if !simulate {
        ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn auth_get_key(cluster_handle: rados_t, osd_id: u64, simulate: bool) -> Result<String, String> {
    let cmd = json!({
        "prefix": "auth get-key",
        "entity": format!("osd.{}", osd_id),
    });
    debug!("auth_get_key: {:?}", cmd.to_string());

    let result = ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
        .map_err(|e| e.to_string())?;
    if result.0.is_some() {
        let return_data = result.0.unwrap();
        let mut l = return_data.lines();
        match l.next() {
            Some(key) => return Ok(key.into()),
            None => {
                return Err(format!(
                "Unable to parse auth get-key: {:?}",
                return_data,
            ))
            }
        }
    }
    Err(format!("Unable to parse auth get-key output: {:?}", result))
}

//ceph osd crush add {id-or-name} {weight}  [{bucket-type}={bucket-name} ...]
/// add or update crushmap position and weight for an osd
fn osd_crush_add(
    cluster_handle: rados_t,
    osd_id: u64,
    weight: f64,
    host: &str,
    simulate: bool,
) -> Result<(), String> {
    let cmd = json!({
        "prefix": "osd crush add",
        "id": osd_id,
        "weight": weight,
        "args": [format!("host={}", host)]
    });
    debug!("osd crush add: {:?}", cmd.to_string());

    if !simulate {
        ceph_mon_command_without_data(cluster_handle, &cmd.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// A fallback function to get the osd id from the mount path.  This isn't
// 100% accurate but it should be good enough for most cases unless the disk
// is mounted in the wrong location or is missing an osd id in the path name
fn get_osd_id_from_path(path: &Path) -> Result<u64, String> {
    match path.file_name() {
        Some(name) => {
            let name_string = name.to_string_lossy().into_owned();
            let parts: Vec<&str> = name_string.split("-").collect();
            let id = u64::from_str(parts[1]).map_err(|e| e.to_string())?;
            Ok(id)
        }
        None => Err(format!("Unable to get filename from {}", path.display())),
    }
}

// Get an osd ID from the whoami file in the osd mount directory
fn get_osd_id(path: &Path, simulate: bool) -> Result<u64, String> {
    let mut whoami_path = PathBuf::from(path);
    whoami_path.push("whoami");
    debug!("Discovering osd id number from: {:?}", whoami_path);
    let mut f = File::open(&whoami_path).map_err(|e| e.to_string())?;
    let mut buff = String::new();
    f.read_to_string(&mut buff).map_err(|e| e.to_string())?;
    u64::from_str(buff.trim()).map_err(|e| e.to_string())
}

fn save_keyring(osd_id: u64, key: &str, simulate: bool) -> IOResult<()> {
    let base_dir = format!("/var/lib/ceph/osd/ceph-{}", osd_id);
    if !Path::new(&base_dir).exists() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            format!("{} directory doesn't exist", base_dir),
        ));
    }
    debug!("Creating {}/keyring", base_dir);
    if !simulate {
        let mut f = File::create(format!("{}/keyring", base_dir))?;
        f.write_all(
            format!("[osd.{}]\n\tkey = {}\n", osd_id, key).as_bytes(),
        )?;
    }
    Ok(())
}

fn add_osd_to_fstab(
    device_info: &block_utils::Device,
    osd_id: u64,
    simulate: bool,
) -> Result<(), String> {
    let fstab = FsTab::default();
    let fstab_entry = fstab::FsEntry {
        fs_spec: format!("UUID={}",
                     device_info.id
                         .unwrap()
                         .hyphenated()
                         .to_string()),
        mountpoint: PathBuf::from(&format!("/var/lib/ceph/osd/ceph-{}", osd_id)),
        vfs_type: device_info.fs_type.to_string(),
        mount_options: vec!["noatime".into(), "inode64".into(), "attr2".into(),
                            "logbsize=256k".into(),"noquota".into()],
        dump: false,
        fsck_order: 2,
    };
    debug!("Saving Fstab entry {:?}", fstab_entry);
    if !simulate {
        let result = fstab.add_entry(fstab_entry).map_err(|e| e.to_string())?;
        match result {
            true => debug!("Fstab entry saved"),
            false => debug!("Fstab entry was updated"),
        };
    }
    Ok(())
}

fn setup_osd_init(osd_id: u64, simulate: bool) -> Result<(), String> {
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
                let output = cmd.output().map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).into_owned());
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
                let output = cmd.output().map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).into_owned());
                }
            }
            return Ok(());
        }
        Daemon::Unknown => {
            return Err("Unknown init system.  Cannot start osd service".to_string());
        }
    };

    Ok(())
}

fn settle_udev() -> IOResult<()> {
    let output = Command::new("udevadm").arg("settle").output()?;
    if !output.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(())
}

// Run ceph-osd --mkfs and return the osd UUID
fn ceph_mkfs(osd_id: u64, journal: Option<&Path>, simulate: bool) -> Result<(), String> {
    debug!("Running ceph-osd --mkfs");
    let fsid_str = format!("/var/lib/ceph/osd/ceph-{}/fsid", osd_id);
    let fsid_path = Path::new(&fsid_str);
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
    Command::new("ceph-osd").args(&args).output().map_err(|e| {
        e.to_string()
    })?;
    Ok(())
}

// Add osds with xfs
// Jewel or earlier
fn add_filestore_osd(dev_path: &Path) -> Result<(), String> {
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
// root@pistore-ho-b17:~# cat /var/lib/ceph/tmp/mnt.sYq7No/magic
// ceph osd volume v026
// cat /var/lib/ceph/tmp/mnt.sYq7No/type
// bluestore
// cat /var/lib/ceph/tmp/mnt.sYq7No/fsid
// a848a7ba-e1c1-4df2-aef5-58895d77895a
fn add_bluestore_osd(dev_path: &Path) -> Result<(), String> {
    //
    Ok(())
}
