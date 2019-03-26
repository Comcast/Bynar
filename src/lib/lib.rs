//! Functions that are needed across most of the workspace.
//!
use std::fs::read_to_string;
use std::path::Path;

use crate::error::{BynarError, BynarResult};
use api::service::{Disk, Op, OpBoolResult, Operation, ResultType};
use hashicorp_vault::client::VaultClient;
use log::{debug, error};
use protobuf::parse_from_bytes;
use protobuf::Message as ProtobufMsg;
use serde::de::DeserializeOwned;
use websocket::client::builder::ClientBuilder;
use websocket::client::sync::Client;
use websocket::message::{Message, OwnedMessage};
use websocket::stream::sync::{TcpStream, TlsStream};

pub mod error;
pub mod host_information;

pub fn load_config<T>(config_dir: &Path, name: &str) -> BynarResult<T>
where
    T: DeserializeOwned,
{
    let p = config_dir.join(name);
    if !p.exists() {
        error!("{} config file does not exist", p.display());
    }
    let s = read_to_string(p)?;
    let deserialized: T = serde_json::from_str(&s)?;
    Ok(deserialized)
}

pub fn connect(
    host: &str,
    port: &str,
    server_publickey: &str,
) -> BynarResult<Client<TlsStream<TcpStream>>> {
    debug!("Connecting to ws://{}:{}", host, port);
    let mut client = ClientBuilder::new(&format!("ws://{}:{}", host, port))?;
    Ok(client.connect_secure(None)?)
}

pub fn get_vault_token(endpoint: &str, token: &str, hostname: &str) -> BynarResult<String> {
    let client = VaultClient::new(endpoint, token)?;
    let res = client.get_secret(&format!("/{}", hostname))?;
    Ok(res)
}

pub fn add_disk_request(
    s: &mut Client<TlsStream<TcpStream>>,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> BynarResult<()> {
    let mut o = Operation::new();
    debug!("Creating add disk operation request");
    o.set_Op_type(Op::Add);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if let Some(id) = id {
        o.set_osd_id(id);
    }

    let encoded = o.write_to_bytes().unwrap();
    let msg = Message::binary(encoded);
    s.send_message(&msg)?;
    debug!("Sending message");

    let response = s.recv_message()?;
    match response {
        OwnedMessage::Binary(buf) => {
            debug!("Decoding msg len: {}", buf.len());
            let op_result = parse_from_bytes::<api::service::OpResult>(&buf)?;
            match op_result.get_result() {
                ResultType::OK => {
                    debug!("Add disk successful");
                    Ok(())
                }
                ResultType::ERR => {
                    if op_result.has_error_msg() {
                        let msg = op_result.get_error_msg();
                        error!("Add disk failed: {}", msg);
                        Err(BynarError::from(op_result.get_error_msg()))
                    } else {
                        error!("Add disk failed but error_msg not set");
                        Err(BynarError::from("Add disk failed but error_msg not set"))
                    }
                }
            }
        }
        _ => {
            // Not a message of bytes... ignore???
            Ok(())
        }
    }
}

pub fn list_disks_request(s: &mut Client<TlsStream<TcpStream>>) -> BynarResult<Vec<Disk>> {
    let mut o = Operation::new();
    debug!("Creating list operation request");
    o.set_Op_type(Op::List);

    debug!("Encoding as hex");
    let encoded = o.write_to_bytes()?;
    debug!("{:?}", encoded);

    let msg = Message::binary(encoded);
    debug!("Sending message");
    s.send_message(&msg)?;

    debug!("Waiting for response");
    let disks_response = s.recv_message()?;
    match disks_response {
        OwnedMessage::Binary(buf) => {
            debug!("Decoding msg len: {}", buf.len());
            let disk_list = parse_from_bytes::<api::service::Disks>(&buf)?;

            let mut d: Vec<Disk> = Vec::new();
            for disk in disk_list.get_disk() {
                d.push(disk.clone());
            }

            Ok(d)
        }
        _ => {
            //If other type of message...ignore???
            Ok(Vec::new())
        }
    }
}

pub fn safe_to_remove_request(
    s: &mut Client<TlsStream<TcpStream>>,
    path: &Path,
) -> BynarResult<bool> {
    let mut o = Operation::new();
    debug!("Creating safe to remove operation request");
    o.set_Op_type(Op::SafeToRemove);
    o.set_disk(format!("{}", path.display()));
    let encoded = o.write_to_bytes()?;
    let msg = Message::binary(encoded);
    debug!("Sending message");
    s.send_message(&msg)?;

    debug!("Waiting for response");
    let safe_response = s.recv_message()?;
    match safe_response {
        OwnedMessage::Binary(buf) => {
            debug!("Decoding msg len: {}", buf.len());
            let op_result = parse_from_bytes::<OpBoolResult>(&buf)?;
            match op_result.get_result() {
                ResultType::OK => Ok(op_result.get_value()),
                ResultType::ERR => Err(BynarError::from(op_result.get_error_msg())),
            }
        }
        _ => {
            //...I have no clue what sort of value might be default...
            Ok(true)
        }
    }
}

pub fn remove_disk_request(
    s: &mut Client<TlsStream<TcpStream>>,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> BynarResult<()> {
    let mut o = Operation::new();
    debug!("Creating remove operation request");
    o.set_Op_type(Op::Remove);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if id.is_some() {
        o.set_osd_id(id.unwrap());
    }

    let encoded = o.write_to_bytes()?;
    let msg = Message::binary(encoded);
    debug!("Sending message");
    s.send_message(&msg)?;

    debug!("Waiting for response");
    let remove_response = s.recv_message()?;
    match remove_response {
        OwnedMessage::Binary(buf) => {
            debug!("Decoding msg len: {}", buf.len());
            let op_result = parse_from_bytes::<api::service::OpResult>(&buf)?;
            match op_result.get_result() {
                ResultType::OK => {
                    debug!("Add disk successful");
                    Ok(())
                }
                ResultType::ERR => {
                    if op_result.has_error_msg() {
                        let msg = op_result.get_error_msg();
                        error!("Remove disk failed: {}", msg);
                        Err(BynarError::from(op_result.get_error_msg()))
                    } else {
                        error!("Remove disk failed but error_msg not set");
                        Err(BynarError::from("Remove disk failed but error_msg not set"))
                    }
                }
            }
        }
        _ => {
            // who knows?
            Ok(())
        }
    }
}
