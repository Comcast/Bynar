//! Functions that are needed across most of the workspace.
//!
extern crate api;
extern crate hashicorp_vault;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate zmq;

use std::io::{Error, ErrorKind, Read};
use std::io::Result as IOResult;
use std::fs::File;
use std::path::Path;

use api::service::{Disk, Op, OpBoolResult, Operation, ResultType};
use hashicorp_vault::client::VaultClient;
use protobuf::Message as ProtobufMsg;
use protobuf::parse_from_bytes;
use serde::de::DeserializeOwned;
use zmq::{Message, Socket};
use zmq::Result as ZmqResult;

pub mod host_information;

pub fn load_config<T>(config_dir: &str, name: &str) -> IOResult<T>
where
    T: DeserializeOwned,
{
    let mut f = File::open(format!("{}/{}", config_dir, name))?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    let deserialized: T =
        serde_json::from_str(&s).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    Ok(deserialized)
}

pub fn connect(host: &str, port: &str, server_publickey: &str) -> ZmqResult<Socket> {
    debug!("Starting zmq sender with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ)?;
    let client_keypair = zmq::CurveKeyPair::new()?;

    requester.set_curve_serverkey(server_publickey)?;
    requester.set_curve_publickey(&client_keypair.public_key)?;
    requester.set_curve_secretkey(&client_keypair.secret_key)?;
    debug!("Connecting to tcp://{}:{}", host, port);
    assert!(
        requester
            .connect(&format!("tcp://{}:{}", host, port))
            .is_ok()
    );
    debug!("Client mechanism: {:?}", requester.get_mechanism());

    Ok(requester)
}

pub fn get_vault_token(
    endpoint: &str,
    token: &str,
    hostname: &str,
) -> Result<String, ::hashicorp_vault::client::error::Error> {
    let client = VaultClient::new(endpoint, token)?;
    let res = client.get_secret(&format!("/{}", hostname))?;
    Ok(res)
}

pub fn add_disk_request(
    s: &mut Socket,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> Result<(), String> {
    let mut o = Operation::new();
    debug!("Creating add disk operation request");
    o.set_Op_type(Op::Add);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if id.is_some() {
        o.set_osd_id(id.unwrap());
    }

    let encoded = o.write_to_bytes().unwrap();
    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    let add_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", add_response.len());
    let op_result =
        parse_from_bytes::<api::service::OpResult>(&add_response).map_err(|e| e.to_string())?;
    match op_result.get_result() {
        ResultType::OK => {
            debug!("Add disk successful");
            Ok(())
        }
        ResultType::ERR => {
            if op_result.has_error_msg() {
                let msg = op_result.get_error_msg();
                error!("Add disk failed: {}", msg);
                Err(op_result.get_error_msg().into())
            } else {
                error!("Add disk failed but error_msg not set");
                Err("Add disk failed but error_msg not set".to_string())
            }
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
    let disk_list =
        parse_from_bytes::<api::service::Disks>(&disks_response).map_err(|e| e.to_string())?;

    let mut d: Vec<Disk> = Vec::new();
    for disk in disk_list.get_disk() {
        d.push(disk.clone());
    }

    Ok(d)
}

pub fn safe_to_remove_request(s: &mut Socket, path: &Path) -> Result<bool, String> {
    let mut o = Operation::new();
    debug!("Creating safe to remove operation request");
    o.set_Op_type(Op::SafeToRemove);
    o.set_disk(format!("{}", path.display()));
    let encoded = o.write_to_bytes().map_err(|e| e.to_string())?;
    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    let safe_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", safe_response.len());
    let op_result = parse_from_bytes::<OpBoolResult>(&safe_response).map_err(|e| e.to_string())?;
    match op_result.get_result() {
        ResultType::OK => Ok(op_result.get_value()),
        ResultType::ERR => Err(op_result.get_error_msg().into()),
    }
}

pub fn remove_disk_request(
    s: &mut Socket,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> Result<(), String> {
    let mut o = Operation::new();
    debug!("Creating remove operation request");
    o.set_Op_type(Op::Remove);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if id.is_some() {
        o.set_osd_id(id.unwrap());
    }

    let encoded = o.write_to_bytes().map_err(|e| e.to_string())?;
    let msg = Message::from_slice(&encoded).map_err(|e| e.to_string())?;
    debug!("Sending message");
    s.send_msg(msg, 0).map_err(|e| e.to_string())?;

    debug!("Waiting for response");
    let remove_response = s.recv_bytes(0).map_err(|e| e.to_string())?;
    debug!("Decoding msg len: {}", remove_response.len());
    let op_result =
        parse_from_bytes::<api::service::OpResult>(&remove_response).map_err(|e| e.to_string())?;
    match op_result.get_result() {
        ResultType::OK => {
            debug!("Add disk successful");
            Ok(())
        }
        ResultType::ERR => {
            if op_result.has_error_msg() {
                let msg = op_result.get_error_msg();
                error!("Remove disk failed: {}", msg);
                Err(op_result.get_error_msg().into())
            } else {
                error!("Remove disk failed but error_msg not set");
                Err("Remove disk failed but error_msg not set".to_string())
            }
        }
    }
}
