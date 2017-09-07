extern crate blkid;
extern crate ceph_rust;
extern crate serde_json;
extern crate libc;
extern crate uuid;

use std::ffi::CString;
use std::fs::create_dir;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

use self::libc::c_char;
use self::blkid::BlkId;
use self::ceph_rust::ceph::*;
use self::ceph_rust::rados::rados_t;
use self::serde_json::Value;
use self::uuid::Uuid;

fn run_ceph_command(
    cluster_handle: rados_t,
    name: &str,
    json_data: serde_json::Value,
) -> Result<(Option<String>, Option<String>), String> {
    let mut data: Vec<*mut c_char> = Vec::with_capacity(1);
    let data_str = CString::new(json_data.to_string()).map_err(
        |e| e.to_string(),
    )?;
    data.push(data_str.as_ptr() as *mut i8);

    Ok(ceph_mon_command_with_data(
        cluster_handle,
        "prefix",
        name,
        None,
        data,
    ).map_err(|e| e.to_string())?)
}

fn osd_out(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let id_data = json!({
        "ids": [osd_id.to_string()]
    });

    Ok(run_ceph_command(cluster_handle, "osd out", id_data)?)
}

fn osd_crush_remove(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let id_data = json!({
        "name": osd_id.to_string()
    });
    Ok(run_ceph_command(
        cluster_handle,
        "osd crush remove",
        id_data,
    )?)
}

fn auth_del(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let id_data = json!({
        "entity": format!("osd.{}", osd_id)
    });

    Ok(run_ceph_command(cluster_handle, "auth del", id_data)?)
}

fn osd_rm(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let id_data = json!({
        "ids": [osd_id.to_string()]
    });

    Ok(run_ceph_command(cluster_handle, "osd rm", id_data)?)

}

fn osd_create(cluster_handle: rados_t, uuid: Uuid) -> Result<u64, String> {
    let id_data = json!({
            "uuid": uuid.hyphenated().to_string()
        });

    let result = run_ceph_command(cluster_handle, "osd create", id_data)?;
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
// {"prefix":"auth add", "entity":"client.admin",
// "caps":["mon rwx", "osd *"]}
fn auth_add(
    cluster_handle: rados_t,
    osd_id: u64,
) -> Result<(Option<String>, Option<String>), String> {
    let id_data = json!({
        "entity": format!("osd.{}", osd_id),
        caps: ["mon profile osd", "osd *"]
    });

    Ok(run_ceph_command(cluster_handle, "auth add", id_data)?)
}

//ceph osd crush add {id-or-name} {weight}  [{bucket-type}={bucket-name} ...]
/// add or update crushmap position and weight for an osd
fn osd_crush_add(
    cluster_handle: rados_t,
    osd_id: u64,
    weight: f64,
    host: &str,
) -> Result<(Option<String>, Option<String>), String> {
    let id_data = json!({
        "id": osd_id,
        "weight": weight,
        "args": [format!("host={}", host)]
    });
    println!("data: {}", id_data.to_string());

    Ok(run_ceph_command(cluster_handle, "osd crush add", id_data)?)
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
