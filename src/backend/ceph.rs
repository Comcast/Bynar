extern crate blkid;
extern crate block_utils;
extern crate ceph_rust;
extern crate ceph_safe_disk;
extern crate fstab;
extern crate libc;
extern crate mktemp;
extern crate serde_json;
extern crate uuid;

use std::env::home_dir;
use std::ffi::CString;
use std::fs::{create_dir, File};
use std::io::{Error, ErrorKind, Read, Write};
use std::io::Result as IOResult;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use backend::Backend;

use self::blkid::BlkId;
use self::ceph_rust::ceph::{connect_to_ceph, ceph_mon_command_without_data, disconnect_from_ceph};
use self::ceph_rust::rados::rados_t;
use self::fstab::FsTab;
use self::libc::c_char;
use self::mktemp::Temp;
use self::serde_json::Value;
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

impl CephBackend {
    pub fn new(config_dir: Option<&Path>) -> IOResult<CephBackend> {
        let ceph_config: CephConfig = match config_dir {
            Some(config) => {
                info!(
                    "Reading ceph config file: {}/{}",
                    config.display(),
                    "ceph.json"
                );
                let mut f = File::open(config.join("ceph.json"))?;
                let mut s = String::new();
                f.read_to_string(&mut s)?;

                let deserialized: CephConfig = serde_json::from_str(&s)?;
                deserialized
            }
            None => {
                info!(
                    "Reading ceph config file: {}/{}",
                    home_dir().unwrap().to_string_lossy(),
                    ".config/ceph.json"
                );
                let mut f = File::open(format!(
                    "{}/{}",
                    home_dir().unwrap().to_string_lossy(),
                    ".config/ceph.json"
                ))?;
                let mut s = String::new();
                f.read_to_string(&mut s)?;

                let deserialized: CephConfig = serde_json::from_str(&s)?;
                deserialized
            }
        };
        info!("Connecting to Ceph");
        let cluster_handle = connect_to_ceph(&ceph_config.user_id, &ceph_config.config_file)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        info!("Connected to ceph");
        Ok(CephBackend { cluster_handle: cluster_handle })
    }

    /// Add a new /dev/ path as an osd.
    fn add_osd(&self, dev_path: &Path) -> Result<(), String> {
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
        block_utils::format_block_device(dev_path, &xfs_options)?;

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
        let new_osd_id = osd_create(self.cluster_handle, info.id.unwrap())?;
        debug!("New osd id created: {}", new_osd_id);

        // Mount the drive
        let mount_point = format!("/var/lib/ceph/osd/ceph-{}", new_osd_id);
        create_dir(&mount_point).map_err(|e| e.to_string())?;
        block_utils::mount_device(&info, &mount_point)?;

        // Format the osd with the osd filesystem
        debug!("Running ceph-osd --mkfs");
        Command::new("ceph-osd")
            .args(&["-i", &new_osd_id.to_string(), "--mkfs", "mkkey"])
            .output()
            .expect("Failed to run ceph-osd mkfs");
        debug!("Creating ceph authorization entry");
        auth_add(self.cluster_handle, new_osd_id)?;
        let auth_key = auth_get_key(self.cluster_handle, new_osd_id)?;
        debug!("Saving ceph keyring");
        save_keyring(new_osd_id, &auth_key).map_err(
            |e| e.to_string(),
        )?;
        let host_info = Host::new().map_err(|e| e.to_string())?;
        debug!(
            "Adding OSD {} to crushmap under host {}",
            new_osd_id,
            host_info.hostname
        );
        osd_crush_add(
            self.cluster_handle,
            new_osd_id,
            0.00_f64,
            &host_info.hostname,
        )?;
        add_osd_to_fstab(&info, new_osd_id)?;
        // This step depends on whether it's systemctl, upstart, etc
        // sudo start ceph-osd id={osd-num}
        Ok(())
    }

    fn remove_osd(&self, dev_path: &Path) -> Result<(), String> {
        //If the OSD is still running we can query its version.  If not then we
        //should ask either another OSD or a monitor.
        let mount_point = match block_utils::get_mount_device(&dev_path).map_err(
            |e| e.to_string(),
        )? {
            Some(osd_path) => osd_path,
            None => {
                let temp_dir = Temp::new_dir().map_err(|e| e.to_string())?;
                temp_dir.to_path_buf()
            }
        };
        debug!("OSD mounted at: {:?}", mount_point);

        let osd_id = get_osd_id(&mount_point)?;
        debug!("Setting osd {} out", osd_id);
        osd_out(self.cluster_handle, osd_id)?;
        debug!("Removing osd {} from crush", osd_id);
        osd_crush_remove(self.cluster_handle, osd_id)?;
        debug!("Deleting osd {} auth key", osd_id);
        auth_del(self.cluster_handle, osd_id)?;
        debug!("Removing osd {}", osd_id);
        osd_rm(self.cluster_handle, osd_id)?;

        // Wipe the disk
        debug!("Erasing disk {:?}", dev_path);
        block_utils::erase_block_device(&dev_path)?;

        Ok(())
    }
}
impl Drop for CephBackend {
    fn drop(&mut self) {
        disconnect_from_ceph(self.cluster_handle);
    }
}

impl Backend for CephBackend {
    fn add_disk(&self, device: &Path) -> IOResult<()> {
        self.add_osd(device).map_err(
            |e| Error::new(ErrorKind::Other, e),
        )?;
        Ok(())
    }
    fn remove_disk(&self, device: &Path) -> IOResult<()> {
        self.remove_osd(device).map_err(
            |e| Error::new(ErrorKind::Other, e),
        )?;
        Ok(())
    }
}

fn osd_out(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let cmd = json!({
        "prefix": "osd out",
        "ids": [osd_id.to_string()]
    });
    debug!("osd out: {:?}", cmd.to_string());

    Ok(ceph_mon_command_without_data(
        cluster_handle,
        &cmd.to_string(),
    ).map_err(|e| e.to_string())?)
}

fn osd_crush_remove(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let cmd = json!({
        "prefix": "osd crush remove",
        "name": format!("osd.{}", osd_id),
    });
    debug!("osd crush remove: {:?}", cmd.to_string());
    Ok(ceph_mon_command_without_data(
        cluster_handle,
        &cmd.to_string(),
    ).map_err(|e| e.to_string())?)
}

fn auth_del(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let cmd = json!({
        "prefix": "auth del",
        "entity": format!("osd.{}", osd_id)
    });
    debug!("auth del: {:?}", cmd.to_string());

    Ok(ceph_mon_command_without_data(
        cluster_handle,
        &cmd.to_string(),
    ).map_err(|e| e.to_string())?)
}

fn osd_rm(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let cmd = json!({
        "prefix": "osd rm",
        "ids": [osd_id.to_string()]
    });
    debug!("osd rm: {:?}", cmd.to_string());

    Ok(ceph_mon_command_without_data(
        cluster_handle,
        &cmd.to_string(),
    ).map_err(|e| e.to_string())?)

}

fn osd_create(cluster_handle: rados_t, uuid: uuid::Uuid) -> Result<u64, String> {
    let cmd = json!({
            "prefix": "osd create",
            "uuid": uuid.hyphenated().to_string()
        });
    debug!("osd create: {:?}", cmd.to_string());

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

fn auth_add(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let cmd = json!({
        "prefix": "auth add",
        "entity": format!("osd.{}", osd_id),
        "caps": ["mon", "allow rwx", "osd", "allow *"],
    });
    debug!("auth_add: {:?}", cmd.to_string());

    Ok(ceph_mon_command_without_data(
        cluster_handle,
        &cmd.to_string(),
    ).map_err(|e| e.to_string())?)
}

fn auth_get_key(cluster_handle: rados_t, osd_id: u64) -> Result<String, String> {
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
) -> Result<(Option<String>, Option<String>), String> {
    let cmd = json!({
        "prefix": "osd crush add",
        "id": osd_id,
        "weight": weight,
        "args": [format!("host={}", host)]
    });
    debug!("osd crush add: {:?}", cmd.to_string());

    Ok(ceph_mon_command_without_data(
        cluster_handle,
        &cmd.to_string(),
    ).map_err(|e| e.to_string())?)
}

// Get an osd ID from the whoami file in the osd mount directory
fn get_osd_id(path: &Path) -> Result<u64, String> {
    let mut whoami_path = PathBuf::from(path);
    whoami_path.push("whoami");
    debug!("Discovering osd id number from: {:?}", whoami_path);
    let mut f = File::open(&whoami_path).map_err(|e| e.to_string())?;
    let mut buff = String::new();
    f.read_to_string(&mut buff).map_err(|e| e.to_string())?;
    u64::from_str(buff.trim()).map_err(|e| e.to_string())
}

fn save_keyring(osd_id: u64, key: &str) -> IOResult<()> {
    let base_dir = format!("/var/lib/ceph/osd/ceph-{}", osd_id);
    if !Path::new(&base_dir).exists() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            format!("{} directory doesn't exist", base_dir),
        ));
    }
    let mut f = File::create(format!("{}/keyring", base_dir))?;
    f.write_all(
        format!("[osd.{}]\n\tkey = {}", osd_id, key).as_bytes(),
    )?;
    Ok(())
}

fn add_osd_to_fstab(device_info: &block_utils::Device, osd_id: u64) -> Result<(), String> {
    let fstab = FsTab::default();
    let entries = fstab.get_entries().map_err(|e| e.to_string())?;

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
    let result = fstab.add_entry(fstab_entry).map_err(|e| e.to_string())?;
    match result {
        true => debug!("Fstab entry saved"),
        false => debug!("Fstab entry was updated"),
    };
    Ok(())
}

fn get_device_uuid(path: &Path) -> Result<String, String> {
    debug!("Probing device with blkid");
    let probe = BlkId::new(&path).map_err(|e| e.to_string())?;
    probe.do_probe().map_err(|e| e.to_string())?;
    let uuid = probe.lookup_value("UUID").map_err(|e| e.to_string())?;
    Ok(uuid.into())
}

fn ceph_mkfs(osd_id: u64, journal: Option<&Path>) -> Result<(), String> {
    //
    debug!("Running ceph-osd --mkfs");
    let journal_str: String;
    let osd_id_str = osd_id.to_string();

    let mut args: Vec<&str> = vec!["--cluster", "ceph", "-i", &osd_id_str, "--mkfs"];
    if let Some(journal_path) = journal {
        journal_str = journal_path.to_string_lossy().into_owned();
        args.push("--journal");
        args.push(&journal_str);
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
