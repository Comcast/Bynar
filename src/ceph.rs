extern crate blkid;
extern crate block_utils;
extern crate ceph_rust;
extern crate serde_json;
extern crate libc;
extern crate uuid;

use std::ffi::CString;
use std::fs::{create_dir, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use self::libc::c_char;
use self::blkid::BlkId;
use self::ceph_rust::ceph::{connect_to_ceph, ceph_mon_command_without_data};
use self::ceph_rust::rados::rados_t;
use self::serde_json::Value;
//use self::uuid::Uuid;

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

/// Add a new /dev/ path as an osd.
pub fn add_osd(dev_path: &Path) -> Result<(), String> {
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
        inode_size: Some(512),
        force: true,
    };
    block_utils::format_block_device(dev_path, &xfs_options)?;
    let info = block_utils::get_device_info(dev_path)?;
    if info.id.is_none() {
        return Err(
            format!("Formatted device {:?} doesn't have a filesystem UUID.  Please investigate",
            dev_path),
        );
    }

    let new_osd_id = osd_create(cluster_handle, info.id.unwrap())?;
    create_dir(format!("/var/lib/ceph/osd/ceph-{}", new_osd_id))
        .map_err(|e| e.to_string())?;
    //mount -o user_xattr /dev/{hdd} /var/lib/ceph/osd/ceph-{osd-number}
    Command::new("ceph-osd")
        .args(&["-i", "0", "--mkfs", "mkkey"])
        .output()
        .expect("Failed to run ceph-osd mkfs");
    auth_add(cluster_handle, new_osd_id)?;
    // /var/lib/ceph/osd/ceph-{osd-num}/keyring
    osd_crush_add(cluster_handle, new_osd_id, 0.00_f64, "host")?;

    // This step depends on whether it's systemctl, upstart, etc
    // sudo start ceph-osd id={osd-num}
    Ok(())
}
