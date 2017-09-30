extern crate api;
extern crate block_utils;
extern crate bytes;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate protobuf;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate simplelog;
extern crate zmq;

mod backend;

use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use api::service::{Disk, Disks, DiskType, Op, OpResult_ResultType, OpResult};
use backend::BackendType;
use block_utils::{Device, MediaType};
use clap::{Arg, App};
use protobuf::Message as ProtobufMsg;
use protobuf::RepeatedField;
use protobuf::core::parse_from_bytes;
use simplelog::{Config, SimpleLogger};
use zmq::{Message, Socket};
use zmq::Result as ZmqResult;

fn convert_media_to_disk_type(m: MediaType) -> DiskType {
    match m {
        MediaType::Loopback => DiskType::LOOPBACK,
        MediaType::LVM => DiskType::LVM,
        MediaType::NVME => DiskType::NVME,
        MediaType::Ram => DiskType::RAM,
        MediaType::Rotational => DiskType::ROTATIONAL,
        MediaType::SolidState => DiskType::SOLID_STATE,
        MediaType::Unknown => DiskType::UNKNOWN,
        MediaType::Virtual => DiskType::VIRTUAL,
    }
}

fn get_disks() -> Result<Vec<Disk>> {
    let mut disks: Vec<Disk> = Vec::new();
    debug!("Searching for block devices");
    let devices = block_utils::get_block_devices().map_err(|e| {
        Error::new(ErrorKind::Other, e)
    })?;

    debug!("Gathering udev info on block devices");
    // Gather info on all devices and skip Loopback devices
    let device_info: Vec<Device> = block_utils::get_all_device_info(devices.as_slice())
        .map_err(|e| Error::new(ErrorKind::Other, e))?
        .into_iter()
        .collect();
    debug!("Device info found: {:?}", device_info);

    for device in device_info {
        let mut d = Disk::new();
        //Translate block_utils MediaType -> Protobuf DiskType
        d.set_field_type(convert_media_to_disk_type(device.media_type));
        d.set_dev_path(format!("/dev/{}", device.name));
        if let Some(serial) = device.serial_number {
            d.set_serial_number(serial);
        }
        disks.push(d);
    }

    Ok(disks)
}

/*
 Server that manages disks
 */
fn listen(backend_type: backend::BackendType, config_dir: &Path) -> ZmqResult<()> {
    debug!("Starting zmq listener with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let mut responder = context.socket(zmq::REP)?;

    assert!(responder.bind("tcp://*:5555").is_ok());

    loop {
        let msg = responder.recv_bytes(0)?;
        debug!("Got msg len: {}", msg.len());
        trace!("Parsing msg {:?} as hex", msg);
        let operation = match parse_from_bytes::<api::service::Operation>(&msg) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to parse_from_bytes {:?}.  Ignoring request", e);
                continue;
            }
        };
        debug!("Operation requested: {:?}", operation.get_Op_type());
        match operation.get_Op_type() {
            Op::Add => {
                let id = if operation.has_id() {
                    Some(operation.get_id())
                } else {
                    None
                };
                if !operation.has_disk() {
                    error!("Add operation must include disk field.  Ignoring request");
                    continue;
                }
                match add_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    id,
                    config_dir,
                ) {
                    Ok(_) => {
                        info!("Add disk successful");
                    }
                    Err(e) => {
                        error!("Add disk error: {:?}", e);
                    }
                };
            }
            Op::List => {
                match list_disks(&mut responder) {
                    Ok(_) => {
                        info!("List disks successful");
                    }
                    Err(e) => {
                        error!("List disks error: {:?}", e);
                    }
                };
            }
            Op::Remove => {
                if !operation.has_disk() {
                    error!("Remove operation must include disk field.  Ignoring request");
                    continue;
                }
                match remove_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    config_dir,
                ) {
                    Ok(_) => {
                        info!("Remove disk successful");
                    }
                    Err(e) => {
                        error!("Remove disk error: {:?}", e);
                    }
                };
            }
        };
        thread::sleep(Duration::from_millis(10));
    }
}

fn add_disk(
    s: &mut Socket,
    d: &str,
    backend: &BackendType,
    id: Option<u64>,
    config_dir: &Path,
) -> Result<()> {
    let backend = backend::load_backend(backend, Some(config_dir)).map_err(
        |e| {
            Error::new(ErrorKind::Other, e)
        },
    )?;
    let mut result = OpResult::new();

    //Send back OpResult
    match backend.add_disk(&Path::new(d), id, false) {
        Ok(_) => {
            result.set_result(OpResult_ResultType::OK);
        }
        Err(e) => {
            result.set_result(OpResult_ResultType::ERR);
            result.set_error_msg(e.to_string());
        }
    }
    let encoded = result.write_to_bytes().map_err(
        |e| Error::new(ErrorKind::Other, e),
    )?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;

    Ok(())
}

fn list_disks(s: &mut Socket) -> Result<()> {
    let disk_list: Vec<Disk> = get_disks().map_err(|e| Error::new(ErrorKind::Other, e))?;

    let mut disks = Disks::new();
    disks.set_disk(RepeatedField::from_vec(disk_list));
    debug!("Encoding disk list");
    let encoded = disks.write_to_bytes().map_err(
        |e| Error::new(ErrorKind::Other, e),
    )?;

    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}

fn remove_disk(s: &mut Socket, d: &str, backend: &BackendType, config_dir: &Path) -> Result<()> {
    //Returns OpResult
    let backend = backend::load_backend(backend, Some(config_dir)).map_err(
        |e| {
            Error::new(ErrorKind::Other, e)
        },
    )?;
    let mut result = OpResult::new();
    match backend.remove_disk(&Path::new(d), false) {
        Ok(_) => {
            result.set_result(OpResult_ResultType::OK);
        }
        Err(e) => {
            result.set_result(OpResult_ResultType::ERR);
            result.set_error_msg(e.to_string());
        }
    };
    let encoded = result.write_to_bytes().map_err(
        |e| Error::new(ErrorKind::Other, e),
    )?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}


fn main() {
    let matches = App::new("Disk Manager")
        .version(crate_version!())
        .author(crate_authors!())
        .about(
            "Detect dead hard drives, create a support ticket and watch for resolution",
        )
        .arg(
            Arg::with_name("backend")
                .default_value("ceph")
                .help("Backend cluster type to manage disks for")
                .long("backend")
                // TODO: Insert other backend values here as they become available
                .possible_values(&["ceph"])
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("configdir")
                .default_value("/etc/ceph_dead_disk")
                .help("The directory where all config files can be found")
                .long("configdir")
                .takes_value(true)
                .required(false),
        )
        .arg(Arg::with_name("v").short("v").multiple(true).help(
            "Sets the level of verbosity",
        ))
        .get_matches();
    let level = match matches.occurrences_of("v") {
        0 => log::LogLevelFilter::Info, //default
        1 => log::LogLevelFilter::Debug,
        _ => log::LogLevelFilter::Trace,
    };
    let config_dir = Path::new(matches.value_of("configdir").unwrap());
    let backend = BackendType::from_str(matches.value_of("backend").unwrap()).unwrap();
    let _ = SimpleLogger::init(level, Config::default());
    match listen(backend, config_dir) {
        Ok(_) => {
            println!("Finished");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    };
}
