extern crate api;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate zmq;

use std::path::Path;

use api::service::{Disk, Operation, Op, OpResult_ResultType};
use protobuf::Message as ProtobufMsg;
use protobuf::core::parse_from_bytes;
use zmq::{Message, Socket};
use zmq::Result as ZmqResult;

pub mod host_information;

pub fn connect(host: &str, port: &str) -> ZmqResult<Socket> {
    debug!("Starting zmq request with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ)?;
    assert!(
        requester
            .connect(&format!("tcp://{}:{}", host, port))
            .is_ok()
    );

    Ok(requester)
}

pub fn add_disk_request(s: &mut Socket, path: &Path) -> Result<(), String> {
    let mut o = Operation::new();
    debug!("Creating add disk operation request");
    o.set_Op_type(Op::Add);
    o.set_disk(format!("{}", path.display()));

    let encoded = o.write_to_bytes().unwrap();
    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    let add_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", add_response.len());
    let op_result = parse_from_bytes::<api::service::OpResult>(&add_response)
        .map_err(|e| e.to_string())?;
    match op_result.get_result() {
        OpResult_ResultType::OK => {
            debug!("Add disk successful");
            Ok(())
        }
        OpResult_ResultType::ERR => {
            let msg = op_result.get_error_msg();
            error!("Add disk failed: {}", msg);
            Err(op_result.get_error_msg().into())
        }
    }
}

/*
pub fn check_disk_request(s: &mut Socket) -> Result<RepairResponse, String> {
    let mut o = Operation::new();
    debug!("Creating check disk operation request");
    o.set_Op_type(Op::Check);

    let encoded = o.write_to_bytes().map_err(|e| e.to_string())?;
    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    let check_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", check_response.len());
    let op_result = parse_from_bytes::<api::service::RepairResponse>(&check_response)
        .map_err(|e| e.to_string())?;

    Ok(op_result)
}
*/

pub fn list_disks_request(s: &mut Socket) -> Result<Vec<Disk>, String> {
    let mut o = Operation::new();
    debug!("Creating list operation request");
    o.set_Op_type(Op::List);

    debug!("Encoding as hex");
    let encoded = o.write_to_bytes().map_err(|e| e.to_string())?;
    debug!("{:?}", encoded);

    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    let disks_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", disks_response.len());
    let disk_list = parse_from_bytes::<api::service::Disks>(&disks_response)
        .map_err(|e| e.to_string())?;

    let mut d: Vec<Disk> = Vec::new();
    for disk in disk_list.get_disk() {
        d.push(disk.clone());
    }

    Ok(d)
}

pub fn remove_disk_request(s: &mut Socket, path: &Path) -> Result<(), String> {
    let mut o = Operation::new();
    debug!("Creating remove operation request");
    o.set_Op_type(Op::Remove);
    o.set_disk(format!("{}", path.display()));

    let encoded = o.write_to_bytes().map_err(|e| e.to_string())?;
    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    debug!("Waiting for response");
    let remove_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", remove_response.len());
    let op_result = parse_from_bytes::<api::service::OpResult>(&remove_response)
        .map_err(|e| e.to_string())?;
    match op_result.get_result() {
        OpResult_ResultType::OK => {
            debug!("Add disk successful");
            Ok(())
        }
        OpResult_ResultType::ERR => {
            let msg = op_result.get_error_msg();
            error!("Remove disk failed: {}", msg);
            Err(msg.into())
        }
    }
}
