use crate::error::{BynarError, BynarResult};
/// Gather information about the current host
use hostname::get_hostname;
use log::debug;
use pnet::datalink::{self, NetworkInterface};
use uname::uname;

use std::fmt::{Display, Formatter, Result as fResult};
use std::fs::{read_to_string, File};
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

/// All the host information we could gather
#[derive(Debug)]
pub struct Host {
    pub hostname: String,
    pub ip: IpAddr,
    pub region: String,
    pub kernel: String,
    pub server_type: String,
    pub serial_number: String,
    pub machine_architecture: String,
    pub scsi_info: Vec<block_utils::ScsiInfo>,
    pub storage_type: StorageTypeEnum,
    pub array_name: Option<String>,
    pub pool_name: Option<String>,
}

impl Host {
    pub fn new() -> BynarResult<Self> {
        //
        debug!("Loading host information");
        let uname_info = uname()?;
        debug!("{:#?}", uname_info);
        let hostname = get_hostname().ok_or_else(|| BynarError::from("hostname not found"))?;
        debug!("{:#?}", hostname);
        let ip = get_ip()?;
        let region = get_region_from_hostname(&hostname)?;
        let storage_type = get_storage_type()?;

        debug!("ip {}, region {}, storage_type {}", ip, region, storage_type);
        let server_type = server_type()?;
        let serial_number = server_serial()?;
        debug!("Gathering raid info");
        let scsi_info = block_utils::get_scsi_info()?;

        Ok(Host {
            ip,
            hostname,
            kernel: uname_info.release,
            region,
            storage_type,
            machine_architecture: uname_info.machine,
            server_type,
            serial_number,
            scsi_info,
            array_name: None,
            pool_name: None,
        })
    }
}
#[derive(Debug)]
pub enum StorageTypeEnum {
    Ceph,
    Scaleio,
    Gluster,
    Hitachi,
}

impl Display for StorageTypeEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fResult {
        let message = match *self {
            StorageTypeEnum::Ceph => "ceph",
            StorageTypeEnum::Scaleio => "scaleio",
            StorageTypeEnum::Hitachi => "hitachi",
            StorageTypeEnum::Gluster => "gluster",
        };
        write!(f, "{}", message)
    }
}
/// Get the default interface
fn get_default_iface() -> BynarResult<String> {
    let p = Path::new("/proc/net/route");
    let proc_route = File::open(p)?;
    let reader = BufReader::new(proc_route);
    for line in reader.lines() {
        let l = line?;
        let parts: Vec<&str> = l.split_whitespace().collect();
        if parts.len() > 2 && parts[1] == "00000000" {
            //Default gateway found
            return Ok(parts[0].to_string());
        }
    }

    Err(BynarError::from("No default interface found"))
}

/// Find the IP on default interface
fn get_ip() -> BynarResult<IpAddr> {
    let mut all_interfaces = datalink::interfaces();
    let default_iface = get_default_iface()?;
    if all_interfaces.is_empty() {
        return Err(BynarError::from("No network interface found"));
    }
    all_interfaces.retain(|iface: &NetworkInterface| iface.name == default_iface);
    if all_interfaces.is_empty() {
        return Err(BynarError::from("No network interface found"));
    }
    if all_interfaces.len() > 1 {
        debug!("More than one default network interface found");
    }
    match all_interfaces.get(0) {
        Some(iface) => {
            let mut my_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
            let mut found: bool = false;
            for ip in &iface.ips {
                if ip.is_ipv4() {
                    my_ip = ip.ip();
                    found = true;
                    break;
                }
            }
            if found {
                Ok(my_ip)
            } else {
                Err(BynarError::from("IPv4 Address not found"))
            }
        }
        None => Err(BynarError::from("Default network interface does not exist")),
    }
}

/// Get region from hostname
fn get_region_from_hostname(hostname: &str) -> BynarResult<String> {
    // Production hostnames are usually <name>-<region_part-1>-<region_part2><*>
    if hostname.contains('-') {
        let splitter: Vec<&str> = hostname.split('-').collect();
        let mut region = String::new();
        for (index, v) in splitter.iter().enumerate() {
            // skip the first sub string
            if index == 1 {
                region.push_str(v);
                region.push_str("-");
            }
            if index == 2 {
                region.push_str(&v[0..1]);
            }
        }
        Ok(region)
    } else {
        Ok("test-region".to_string())
    }
}

/// Get storage type used on this system
fn get_storage_type() -> BynarResult<StorageTypeEnum> {
    // TODO: Change this later for other types
    Ok(StorageTypeEnum::Ceph)
}

/// Find the server type
fn server_type() -> BynarResult<String> {
    debug!("Gathering server type");
    let path = Path::new("/sys/class/dmi/id/product_name");
    if Path::exists(path) {
        let buff = read_to_string(path)?;
        return Ok(buff.trim().into());
    }
    Err(BynarError::from("/sys/class/dmi/id/product_name does not exist"))
}

fn server_serial() -> BynarResult<String> {
    debug!("Gathering server serial");
    // Try the easy way first
    debug!("Checking for serial in /sys/class/dmi/id/product_serial");
    let path_1 = Path::new("/sys/class/dmi/id/product_serial");
    if Path::exists(path_1) {
        let buff = read_to_string(path_1)?;
        return Ok(buff.trim().into());
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
    Err(BynarError::from("Unable to discover system serial"))
}

//TODO: smp-utils has a lot of use information about how to interface with sas enclosures
// http://sg.danny.cz/sg/smp_utils.html#mozTocId356346
/*
/// Given a disk find out what chassis position this disk is located at
fn disk_position(dev: &Path, raid_type: RaidType) -> Result<String> {
    //
    Ok("".into())
}
*/
