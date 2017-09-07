extern crate blkid;
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
use self::ceph_rust::ceph::*;
use self::ceph_rust::rados::rados_t;
use self::serde_json::Value;
use self::uuid::Uuid;

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

fn osd_create(cluster_handle: rados_t, uuid: Uuid) -> Result<u64, String> {
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
        "caps": ["mon rwx", "osd *"]
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
    let osd_id: u64 = {
        let mut whoami_path = PathBuf::from(path);
        whoami_path.push("whoami");
        debug!("Discovering osd id number from: {:?}", whoami_path);
        let mut f = File::open(&whoami_path).map_err(|e| e.to_string())?;
        let mut buff = String::new();
        f.read_to_string(&mut buff).map_err(|e| e.to_string())?;
        u64::from_str(buff.trim()).map_err(|e| e.to_string())?
    };
    osd_out(cluster_handle, osd_id)?;
    osd_crush_remove(cluster_handle, osd_id)?; //ceph osd crush remove osd.i
    auth_del(cluster_handle, osd_id)?; //ceph auth del osd.i
    osd_rm(cluster_handle, osd_id)?; //ceph osd rm i
    Ok(())
}

fn get_device_uuid(path: &Path) -> Result<String, String> {
    debug!("Probing device with blkid");
    let probe = BlkId::new(&path).map_err(|e| e.to_string())?;
    probe.do_probe().map_err(|e| e.to_string())?;
    let uuid = probe.lookup_value("UUID").map_err(|e| e.to_string())?;
    Ok(uuid.into())
}

pub fn add_osd() -> Result<(), String> {
    let cluster_handle = connect_to_ceph("admin", "/etc/ceph/ceph.conf").map_err(
        |e| {
            e.to_string()
        },
    )?;
    //ceph osd create [{uuid} [{id}]]
    // TODO: Get uuid of the osd from blkid
    //osd_create(cluster_handle, 0)?;
    create_dir(format!("/var/lib/ceph/osd/ceph-{}", 0))
        .map_err(|e| e.to_string())?;
    //mkfs -t {fstype} /dev/{drive}
    //mount -o user_xattr /dev/{hdd} /var/lib/ceph/osd/ceph-{osd-number}
    Command::new("ceph-osd")
        .args(&["-i", "0", "--mkfs", "mkkey"])
        .output()
        .expect("Failed to run ceph-osd mkfs");
    auth_add(cluster_handle, 0)?;
    //ceph auth add osd.{osd-num} osd 'allow *' mon 'allow rwx' -i
    // /var/lib/ceph/osd/ceph-{osd-num}/keyring
    //osd_crush_add(cluster_handle, 0)?;
    // ceph osd crush add {id-or-name} {weight}  [{bucket-type}={bucket-name} ...]

    // This step depends on whether it's systemctl, upstart, etc
    // sudo start ceph-osd id={osd-num}
    Ok(())
}
