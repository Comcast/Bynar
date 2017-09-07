/// Gather information about the current host
extern crate block_utils;
extern crate dmi;

use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result};
use std::path::Path;

use self::block_utils::RaidType;

pub struct Host {}

/// Find the server hostname
pub fn hostname() -> Result<String> {
    let mut buff = String::new();
    let mut f = File::open("/etc/hostname")?;
    f.read_to_string(&mut buff)?;
    Ok(buff.trim().into())
}

/// Find the server type
pub fn server_type() -> Result<String> {
    let path = Path::new("/sys/class/dmi/id/product_name");
    if Path::exists(path) {
        let mut f = File::open(path)?;
        let mut buff = String::new();
        f.read_to_string(&mut buff)?;
        return Ok(buff);
    }
    Err(Error::new(
        ErrorKind::Other,
        "/sys/class/dmi/id/product_name does not exist",
    ))
}

pub fn server_serial() -> Result<String> {
    // Try the easy way first
    debug!("Checking for serial in /sys/class/dmi/id/product_serial");
    let path_1 = Path::new("/sys/class/dmi/id/product_serial");
    if Path::exists(path_1) {
        let mut f = File::open(path_1)?;
        let mut buff = String::new();
        f.read_to_string(&mut buff)?;
        return Ok(buff);
    }

    // /sys/firmware/dmi/tables/DMI
    /*
    debug!("Checking for serial in /sys/firmware/dmi/tables/DMI");
    let path_2 = Path::new("/sys/firmware/dmi/tables/DMI");
    if Path::exists(path_2) {
        let dmi_tables = dmi::get_tables(path_2)?;
        for table in dmi_tables {
            if table.header.kind == 1 {
                let s = dmi::SystemInfo::from_bytes(&table.data).expect(
                    "SystemInfo DMI parsing failed",
                );
                debug!("SystemInfo: {:?}", s);
            }
        }
    }
    // /sys/firmware/dmi/tables/DMI
    debug!("Checking for serial in /sys/firmware/efi/systab");
    let path_3 = Path::new("/sys/firmware/efi/systab");
    if Path::exists(path_3) {
        let dmi_tables = dmi::get_tables(path_3)?;
        for table in dmi_tables {
            if table.header.kind == 1 {
                let s = dmi::SystemInfo::from_bytes(&table.data).expect(
                    "SystemInfo DMI parsing failed",
                );
                debug!("SystemInfo: {:?}", s);
            }
        }
    }
    */
    // /sys/firmware/efi/systab
    // /proc/efi/systab
    Err(Error::new(
        ErrorKind::Other,
        "Unable to discover system serial",
    ))
}

//TODO: smp-utils has a lot of use information about how to interface with sas enclosures
// http://sg.danny.cz/sg/smp_utils.html#mozTocId356346
fn raid_info(dev: &Path) -> Result<()> {
    let info = block_utils::get_raid_info().unwrap();
    println!("raid info: {:?}", info);
    Ok(())
}

/// Given a disk find out what chassis position this disk is located at
fn disk_position(dev: &Path, raid_type: RaidType) -> Result<String> {
    //
    Ok("".into())
}
