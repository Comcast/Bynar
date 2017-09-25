extern crate api;
extern crate block_utils;
extern crate bytes;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate simplelog;
extern crate zmq;

use std::io::{Error, ErrorKind, Result};
use std::ops::Deref;
use std::thread;
use std::time::Duration;

use api::service::{Disk, Disks, DiskType, Operation, Op, Osd, RepairResponse, OpResult};
use block_utils::{Device, MediaType};
use clap::{Arg, App};
use protobuf::Message as ProtobufMsg;
use protobuf::RepeatedField;
use protobuf::core::parse_from_bytes;
use protobuf::hex::{decode_hex, encode_hex};
use simplelog::{Config, SimpleLogger};
use zmq::{Message, Socket};
use zmq::Result as ZmqResult;

fn convert_media_to_disk_type(m: MediaType) -> DiskType {
    match m {
        MediaType::SolidState => DiskType::SOLID_STATE,
        MediaType::Rotational => DiskType::ROTATIONAL,
        MediaType::Loopback => DiskType::LOOPBACK,
        MediaType::LVM => DiskType::LVM,
        MediaType::Ram => DiskType::RAM,
        MediaType::Virtual => DiskType::VIRTUAL,
        MediaType::Unknown => DiskType::UNKNOWN,
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
fn listen() -> ZmqResult<()> {
    debug!("Starting zmq listener with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let mut responder = context.socket(zmq::REP)?;

    assert!(responder.bind("tcp://*:5555").is_ok());

    loop {
        let msg = responder.recv_bytes(0)?;
        debug!("Got msg len: {}", msg.len());
        debug!("Decoding msg {:?} as hex", msg);
        //let op_bytes = decode_hex(&String::from_utf8(msg).unwrap());
        debug!("Parsing msg");
        let operation = match parse_from_bytes::<api::service::Operation>(&msg) {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("Failed to parse_from_bytes {:?}", e);
                return Err(zmq::Error::EPROTO);
            }
        };
        debug!("Operation requested: {:?}", operation.get_Op_type());
        match operation.get_Op_type() {
            Op::Add => add_disk(&mut responder),
            Op::Check => check_disk(&mut responder),
            Op::List => list_disks(&mut responder),
            Op::Remove => remove_disk(&mut responder),
        };

        //println!("Received {}", msg.as_str().unwrap_or(""));
        thread::sleep(Duration::from_millis(10));
        //responder.send("hello".as_bytes(), 0)?;
    }
}

fn add_disk(s: &mut Socket) -> ZmqResult<()> {
    Ok(())
}

fn check_disk(s: &mut Socket) -> ZmqResult<()> {
    //
    Ok(())
}

fn list_disks(s: &mut Socket) -> ZmqResult<()> {
    let mut disk_list: Vec<Disk> = get_disks().map_err(|e| zmq::Error::EPROTO)?;

    let mut disks = Disks::new();
    disks.set_disk(RepeatedField::from_vec(disk_list));
    debug!("Encoding disk list");
    let encoded = disks.write_to_bytes().unwrap();

    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}

fn remove_disk(s: &mut Socket) -> ZmqResult<()> {
    //
    Ok(())
}


fn main() {
    let matches = App::new("Ceph Disk Manager")
        .version(crate_version!())
        .author(crate_authors!())
        .about(
            "Detect dead hard drives, create a support ticket and watch for resolution",
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
    let _ = SimpleLogger::init(level, Config::default());
    listen();
}
