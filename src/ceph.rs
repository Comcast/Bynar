extern crate blkid;
extern crate block_utils;
extern crate ceph_rust;
extern crate fstab;
extern crate libc;
extern crate serde_json;
extern crate uuid;

use std::ffi::CString;
use std::fs::{create_dir, File};
use std::io::{Read, Write};
use std::io::Result as IOResult;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use self::blkid::BlkId;
use self::ceph_rust::ceph::{connect_to_ceph, ceph_mon_command_without_data};
use self::ceph_rust::rados::rados_t;
use self::fstab::FsTab;
use self::libc::c_char;
use self::serde_json::Value;
use super::host_information::hostname;

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

pub fn remove_osd(path: &Path) -> Result<(), String> {
    //If the OSD is still running we can query its version.  If not then we
    //should ask either another OSD or a monitor.

    //let version = ceph_version("/var/lib/ceph/osd/xx");
    //println!("ceph_version: {:?}", version);
    let cluster_handle = connect_to_ceph("admin", "/etc/ceph/ceph.conf").map_err(
        |e| {
            e.to_string()
        },
    )?;
    let osd_id = get_osd_id(path)?;
    debug!("Setting osd {} out", osd_id);
    osd_out(cluster_handle, osd_id)?;
    debug!("Removing osd {} from crush", osd_id);
    osd_crush_remove(cluster_handle, osd_id)?; //ceph osd crush remove osd.i
    debug!("Deleting osd {} auth key", osd_id);
    auth_del(cluster_handle, osd_id)?; //ceph auth del osd.i
    debug!("Removing osd {}", osd_id);
    osd_rm(cluster_handle, osd_id)?; //ceph osd rm i

    // Wipe the disk if it's still mounted
    debug!("Erasing the disk {}", osd_id);
    let mount_point = block_utils::get_mount_device(&path).map_err(
        |e| e.to_string(),
    )?;
    match mount_point {
        Some(disk) => {
            debug!("Disk mounted for {:?}.", disk);
            block_utils::erase_block_device(&disk)?;
        }
        None => {
            // No disk mounted.  Nothing to do
            debug!("No disk mounted for {:?}.  Nothing to do", path);
        }
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

/// Add a new /dev/ path as an osd.
pub fn add_osd(dev_path: &Path) -> Result<(), String> {
    //Connect to ceph
    let cluster_handle = connect_to_ceph("admin", "/etc/ceph/ceph.conf").map_err(
        |e| {
            e.to_string()
        },
    )?;
    //Format the drive
    let xfs_options = block_utils::Filesystem::Xfs {
        stripe_size: None,
        stripe_width: None,
        block_size: None,
        inode_size: Some(2048),
        force: true,
    };
    debug!("Formatting {:?} with XFS options: {:?}", dev_path, xfs_options);
    block_utils::format_block_device(dev_path, &xfs_options)?;

    // Probe the drive
    debug!("udev Probing device {:?}", dev_path);
    let info = block_utils::get_device_info(dev_path)?;
    debug!("udev info {:?}", info);
    if info.id.is_none() {
        return Err(
            format!("Formatted device {:?} doesn't have a filesystem UUID.  Please investigate",
            dev_path),
        );
    }

    // Create a new osd id
    let new_osd_id = osd_create(cluster_handle, info.id.unwrap())?;
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
    auth_add(cluster_handle, new_osd_id)?;
    let auth_key = auth_get_key(cluster_handle, new_osd_id)?;
    debug!("Saving ceph keyring");
    save_keyring(new_osd_id, &auth_key).map_err(
        |e| e.to_string(),
    )?;
    let hostname = hostname().map_err(|e| e.to_string())?;
    debug!("Adding OSD {} to crushmap under host {}", new_osd_id, hostname);
    osd_crush_add(cluster_handle, new_osd_id, 0.00_f64, &hostname)?;
    add_osd_to_fstab(&info, new_osd_id)?;
    // This step depends on whether it's systemctl, upstart, etc
    // sudo start ceph-osd id={osd-num}
    Ok(())
}
