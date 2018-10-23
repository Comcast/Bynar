/// Gather information about the current host
extern crate block_utils;
extern crate dmi;
extern crate hostname;
extern crate uname;
extern crate pnet;

use std::net::{IpAddr, Ipv4Addr};
use std::fs::read_to_string;
use std::io::{Error, ErrorKind, Result};
use std::fs::File;
use std::fmt::{Display, Formatter, Result as fResult};
use std::path::Path;
use self::pnet::datalink::{self, NetworkInterface};
//use self::block_utils::RaidType;
use self::hostname::get_hostname;
use self::uname::uname;
use std::io::{BufRead, BufReader};

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
    pub raid_info: Vec<block_utils::ScsiInfo>,
    pub storage_type: StorageTypeEnum,
    pub array_name: Option<String>,
    pub pool_name: Option<String>,
}

impl Host {
    pub fn new() -> Result<Self> {
        //
        debug!("Loading host information");
        let uname_info = uname()?;
        debug!("{:#?}", uname_info);
        let hostname =
            get_hostname().ok_or_else(|| Error::new(ErrorKind::Other, "hostname not found"))?;
        debug!("{:#?}", hostname);
        let ip = get_ip()?;
        let region = get_region_from_hostname(&hostname)?;
        let storage_type = get_storage_type()?;
       
        debug!("ip {}, region {}, storage_type {}", ip, region, storage_type);
        let server_type = server_type()?;
        let serial_number = server_serial()?;
        println!("Gathering raid info");
        let raid_info =
            block_utils::get_raid_info().map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(Host {
            ip,
            hostname,
            kernel: uname_info.release,
            region,
            storage_type,
            machine_architecture: uname_info.machine,
            server_type,
            serial_number,
            raid_info,
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
    fn fmt(&self, f: &mut Formatter) -> fResult {
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
fn get_default_iface() -> Result<String> {
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

    Err(Error::new(ErrorKind::Other, "No default interface found"))
}

/// Find the IP on default interface
fn get_ip() -> Result<IpAddr> {
    let mut all_interfaces = datalink::interfaces();
    let default_iface = get_default_iface()?;
    if all_interfaces.is_empty() {
        Err(Error::new(ErrorKind::Other, "No network interface found"))
    } else {
        all_interfaces.retain(| iface: &NetworkInterface| iface.name == default_iface);
        if all_interfaces.len() != 1 {
            Err(Error::new(ErrorKind::Other, "More than one default network interface found"))
        } else {
            match all_interfaces.get(0) {
                Some(iface) => {
                let mut my_ip = IpAddr::V4(Ipv4Addr::new(0,0,0,0));
                let mut found: bool = false;
                let ips= &iface.ips;
                for ip in ips {
                    if ip.is_ipv4() {
                        my_ip = ip.ip();
                        found = true;
                        break;
                    } 
                }
                if found {
                    Ok(my_ip)
                } else {
                    Err(Error::new(ErrorKind::Other, "IPv4 Address not found"))
                }
                }
                None => {
                    Err(Error::new(ErrorKind::Other, "Default network interface does not exist"))

                }
            }
        }
    }
}

/// Get region from hostname
fn get_region_from_hostname(hostname: &str) -> Result<String> {
    // Production hostnames are usually <name>-<region><*>
    if hostname.contains('-') {
        let splitter: Vec<&str> = hostname.splitn(2, '-').collect();
        match splitter.get(1) {
            Some(region) => {
                Ok(region[0..4].to_owned())
            }
            None => {
                Err(Error::new(ErrorKind::Other, "Cannot deduce region from hostname"))
            }
        }
    } else {
        Ok("test-region".to_string())
    }
}

/// Get storage type used on this system
fn get_storage_type() -> Result<StorageTypeEnum> {
    // TODO: Change this later for other types
    Ok(StorageTypeEnum::Ceph)
}

/// Find the server type
fn server_type() -> Result<String> {
    debug!("Gathering server type");
    let path = Path::new("/sys/class/dmi/id/product_name");
    if Path::exists(path) {
        let buff = read_to_string(path)?;
        return Ok(buff.trim().into());
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
    Err(Error::new(
        ErrorKind::Other,
        "Unable to discover system serial",
    ))
}

//TODO: smp-utils has a lot of use information about how to interface with sas enclosures
// http://sg.danny.cz/sg/smp_utils.html#mozTocId356346
/*
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
*/
