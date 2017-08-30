/// Gather information about the current host
extern crate block_utils;

use std::fs::File;
use std::io::{Read, Result};
use std::path::Path;

use self::block_utils::RaidType;

/// Find the server hostname
pub fn hostname() -> Result<String> {
    let mut buff = String::new();
    let mut f = File::open("/etc/hostname")?;
    f.read_to_string(&mut buff)?;
    Ok(buff.trim().into())
}

/// Find the server manufacturer
pub fn server_type() -> Result<String> {
    //
    Ok("".into())
}

pub fn raid_info(dev: &Path) -> Result<()> {
    let info = block_utils::get_raid_info().unwrap();
    println!("raid info: {:?}", info);
    Ok(())
}

/// Given a disk find out what chassis position this disk is located at
pub fn disk_position(dev: &Path, raid_type: RaidType) -> Result<String> {
    //
    Ok("".into())
}
