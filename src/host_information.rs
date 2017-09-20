/// Gather information about the current host
extern crate block_utils;
extern crate dmi;
extern crate uname;

use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result};
use std::path::Path;

use self::block_utils::{RaidType, ScsiInfo};
use self::uname::uname;

/// All the host information we could gather
#[derive(Debug)]
pub struct Host {
    pub hostname: String,
    pub kernel: String,
    pub server_type: String,
    pub serial_number: String,
    pub machine_architecture: String,
    pub raid_info: Vec<block_utils::ScsiInfo>,
}

impl Host {
    pub fn new() -> Result<Self> {
        //
        debug!("Loading host information");
        debug!("Gathering uname info");
        let uname_info = uname()?;
        let hostname = hostname()?;
        let server_type = server_type()?;
        let serial_number = server_serial()?;
        debug!("Gathering raid info");
        let raid_info = block_utils::get_raid_info().map_err(|e| {
            Error::new(ErrorKind::Other, e)
        })?;
        Ok(Host {
            hostname: hostname,
            kernel: uname_info.release,
            machine_architecture: uname_info.machine,
            server_type: server_type,
            serial_number: serial_number,
            raid_info: raid_info,
        })
    }
}

/// Find the server hostname
fn hostname() -> Result<String> {
    debug!("Gathering hostname info");
    let mut buff = String::new();
    let mut f = File::open("/etc/hostname")?;
    f.read_to_string(&mut buff)?;
    Ok(buff.trim().into())
}

/// Find the server type
fn server_type() -> Result<String> {
    debug!("Gathering server type");
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

fn server_serial() -> Result<String> {
    debug!("Gathering server serial");
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
    let info = block_utils::get_raid_info().map_err(|e| {
        Error::new(ErrorKind::Other, e)
    })?;
    println!("raid info: {:?}", info);
    Ok(())
}

/// Given a disk find out what chassis position this disk is located at
fn disk_position(dev: &Path, raid_type: RaidType) -> Result<String> {
    //
    Ok("".into())
}
