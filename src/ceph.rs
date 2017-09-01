extern crate blkid;
extern crate ceph_rust;

use std::fs::create_dir;
use std::path::Path;
use std::process::Command;

use self::blkid::BlkId;
use self::ceph_rust::ceph::*;
use self::ceph_rust::rados::rados_t;

fn run_ceph_command(cluster_handle: rados_t, name: &str, args: &Vec<&str>) -> Result<(), String> {
    ceph_command(cluster_handle, "prefix", name, CephCommandTypes::Mon, args)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn osd_out(cluster_handle: rados_t, osd_id: u64) -> Result<(), String> {
    let id = osd_id.to_string();
    let mut args: Vec<&str> = Vec::new();
    args.push(&id);
    return Ok(run_ceph_command(cluster_handle, "osd out", &args)?);
}

fn osd_crush_remove(cluster_handle: rados_t, osd_id: u64) -> Result<(), String> {
    let id = osd_id.to_string();
    let mut args: Vec<&str> = Vec::new();
    args.push(&id);
    return Ok(run_ceph_command(cluster_handle, "osd crush remove", &args)?);
}

fn auth_del(cluster_handle: rados_t, osd_id: u64) -> Result<(), String> {
    let id = osd_id.to_string();
    let mut args: Vec<&str> = Vec::new();
    args.push(&id);
    return Ok(run_ceph_command(cluster_handle, "auth del", &args)?);
}

fn osd_rm(cluster_handle: rados_t, osd_id: u64) -> Result<(), String> {
    let id = osd_id.to_string();
    let mut args: Vec<&str> = Vec::new();
    args.push(&id);
    return Ok(run_ceph_command(cluster_handle, "osd rm", &args)?);
}

fn osd_create(cluster_handle: rados_t, osd_id: u64) -> Result<(), String> {
    let id = osd_id.to_string();
    let mut args: Vec<&str> = Vec::new();
    args.push(&id);
    return Ok(run_ceph_command(cluster_handle, "osd create", &args)?);
}

fn auth_add(cluster_handle: rados_t, osd_id: u64) -> Result<(), String> {
    let id = osd_id.to_string();
    let mut args: Vec<&str> = Vec::new();
    args.push(&id);
    return Ok(run_ceph_command(cluster_handle, "auth add", &args)?);
}

fn osd_crush_add(cluster_handle: rados_t, osd_id: u64) -> Result<(), String> {
    let id = osd_id.to_string();
    let mut args: Vec<&str> = Vec::new();
    args.push(&id);
    return Ok(run_ceph_command(cluster_handle, "osd crush add", &args)?);
}

pub fn remove_osd(path: &Path) -> Result<(), String> {
    //If the OSD is still running we can query its version.  If not then we
    //should ask either another OSD or a monitor.
    let version = ceph_version("/var/lib/ceph/osd/xx");
    let cluster_handle = connect_to_ceph("admin", "/etc/ceph/ceph.conf").map_err(
        |e| {
            e.to_string()
        },
    )?;
    osd_out(cluster_handle, 0)?;
    osd_crush_remove(cluster_handle, 1)?; //ceph osd crush remove osd.i
    auth_del(cluster_handle, 0)?; //ceph auth del osd.i
    osd_rm(cluster_handle, 0)?; //ceph osd rm i
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
    osd_create(cluster_handle, 0)?;
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
    osd_crush_add(cluster_handle, 0)?;
    // ceph osd crush add {id-or-name} {weight}  [{bucket-type}={bucket-name} ...]

    // This step depends on whether it's systemctl, upstart, etc
    // sudo start ceph-osd id={osd-num}
    Ok(())
}
