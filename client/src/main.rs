extern crate api;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate simplelog;
extern crate zmq;

use api::service::{Disk, Disks, DiskType, Operation, Op, Osd, RepairResponse, OpResult};
use clap::{Arg, App};
use protobuf::Message as ProtobufMsg;
use protobuf::RepeatedField;
use protobuf::core::parse_from_bytes;
use protobuf::hex::{decode_hex, encode_hex};
use simplelog::{Config, SimpleLogger};
use zmq::{Message, Socket};
use zmq::Result as ZmqResult;
/*
    CLI client to call functions over RPC
*/

fn connect() -> ZmqResult<Socket> {
    debug!("Starting zmq sender with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ)?;
    assert!(requester.connect("tcp://localhost:5555").is_ok());

    Ok(requester)
}

fn add_disk(s: Socket) -> ZmqResult<()> {
    //
    Ok(())
}

fn check_disk(s: Socket) -> ZmqResult<()> {
    //
    Ok(())
}

fn list_disks(s: Socket) -> ZmqResult<()> {
    let mut o = Operation::new();
    debug!("Creating list operation request");
    o.set_Op_type(Op::List);

    debug!("Encoding as hex");
    let encoded = o.write_to_bytes().unwrap();
    debug!("{:?}", encoded);

    let msg = Message::from_slice(&encoded)?;
    debug!("Sending message");
    s.send_msg(msg, 0)?;

    debug!("Waiting for response");
    let disks_response = s.recv_bytes(0)?;
    debug!("Decoding msg len: {}", disks_response.len());
    //let rep_bytes = decode_hex(disks_response.as_str().unwrap());
    let disk_list = parse_from_bytes::<api::service::Disks>(&disks_response).unwrap();
    println!("disk list: {:?}", disk_list);

    Ok(())
}

fn remove_disk(s: Socket) -> ZmqResult<()> {
    //
    Ok(())
}


fn main() {
    let matches = App::new("Ceph Disk Manager Client")
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
    info!("Starting up");

    let s = match connect() {
        Ok(s) => s,
        Err(e) => {
            error!("Error connecting to socket: {:?}", e);
            return;
        }
    };
    list_disks(s);
}
