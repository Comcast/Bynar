//! Functions that are needed across most of the workspace.
//!
use serde_derive::*;
use std::fs::read_to_string;
use std::path::Path;

use crate::error::{BynarError, BynarResult};
use api::service::{
    Disk, JiraInfo, Op, OpJiraTicketsResult, OpOutcome, OpOutcomeResult, Operation, ResultType,
};
use hashicorp_vault::client::VaultClient;
use log::{debug, error};
use protobuf::parse_from_bytes;
use protobuf::Message as ProtobufMsg;
use serde::de::DeserializeOwned;
use zmq::Socket;

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

pub fn connect(host: &str, port: &str, server_publickey: &[u8]) -> BynarResult<Socket> {
    debug!("Starting zmq sender with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let requester = context.socket(zmq::STREAM)?;
    let client_keypair = zmq::CurveKeyPair::new()?;
    debug!("Created new keypair");
    requester.set_curve_serverkey(server_publickey)?;
    requester.set_curve_publickey(&client_keypair.public_key)?;
    requester.set_curve_secretkey(&client_keypair.secret_key)?;
    debug!("Connecting to tcp://{}:{}", host, port);
    assert!(requester.connect(&format!("tcp://{}:{}", host, port)).is_ok());
    debug!("Client mechanism: {:?}", requester.get_mechanism());
    Ok(requester)
}

pub fn get_vault_token(endpoint: &str, token: &str, hostname: &str) -> BynarResult<String> {
    let client = VaultClient::new(endpoint, token)?;
    let res = client.get_secret(&format!("/{}", hostname))?;
    Ok(res)
}

/// send an operation request to the disk-manager
pub fn request(s: &Socket, op: Operation, client_id: Vec<u8>) -> BynarResult<()> {
    //send the id first
    s.send(&client_id, zmq::SNDMORE)?;
    let encoded = op.write_to_bytes()?;
    debug!("Sending message");
    s.send(&encoded, 0)?;
    Ok(())
}

/// send an add_disk request to the disk-manager
pub fn add_disk_request(
    s: &Socket,
    path: &Path,
    id: Option<u64>,
    client_id: Vec<u8>,
    simulate: bool,
) -> BynarResult<()> {
    debug!("Creating add disk operation request");
    let mut o = Operation::new();
    o.set_Op_type(Op::Add);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if let Some(id) = id {
        o.set_osd_id(id);
    }
    debug!("Sending message in add_disk_request");
    request(s, o, client_id)?;
    Ok(())
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

/// send a list disk request to the disk-manager
pub fn list_disks_request(s: &Socket, client_id: Vec<u8>) -> BynarResult<()> {
    //BynarResult<Vec<Disk>> {
    debug!("Printing ID {:?}", client_id);
    let mut o = Operation::new();
    debug!("Creating list operation request");
    o.set_Op_type(Op::List);
    debug!("Sending message in list_disks_request");
    request(s, o, client_id)?;
    Ok(())
}

/// send safe-to-remove request to disk-manager
pub fn safe_to_remove_request(s: &Socket, path: &Path, client_id: Vec<u8>) -> BynarResult<()> {
    let mut o = Operation::new();
    debug!("Creating safe to remove operation request");
    o.set_Op_type(Op::SafeToRemove);
    o.set_disk(format!("{}", path.display()));
    debug!("Sending message in safe_to_remove_request");
    request(s, o, client_id)?;
    Ok(())
}

/// Send a remove disk request to the disk_manager
pub fn remove_disk_request(
    s: &Socket,
    path: &Path,
    id: Option<u64>,
    client_id: Vec<u8>,
    simulate: bool,
) -> BynarResult<()> {
    let mut o = Operation::new();
    debug!("Creating remove operation request");
    o.set_Op_type(Op::Remove);
    o.set_disk(format!("{}", path.display()));
    o.set_simulate(simulate);
    if let Some(osd_id) = id {
        o.set_osd_id(osd_id);
    }
    debug!("Sending message in remove_disk_request");
    request(s, o, client_id)?;
    Ok(())
}

// default filename for daemon_output
fn default_out() -> String {
    "bynar_daemon.out".to_string()
}
// default filename for daemon_err
fn default_err() -> String {
    "bynar_daemon.err".to_string()
}
//default filename for daemon_pid
fn default_pid() -> String {
    "bynar_daemon.pid".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigSettings {
    pub manager_host: String,
    pub manager_port: u16,
    /// Redfish Ip address or dns name ( Usually iLo where redfish is listening)
    pub redfish_ip: Option<String>,
    /// Redfish credentials
    pub redfish_username: Option<String>,
    /// Redfish credentials
    pub redfish_password: Option<String>,
    /// The port redfish is listening on
    pub redfish_port: Option<u16>,
    pub slack_webhook: Option<String>,
    pub slack_channel: Option<String>,
    pub slack_botname: Option<String>,
    pub vault_endpoint: Option<String>,
    pub vault_token: Option<String>,
    pub jira_user: String,
    pub jira_password: String,
    pub jira_host: String,
    pub jira_issue_type: String,
    pub jira_priority: String,
    pub jira_project_id: String,
    pub jira_ticket_assignee: String,
    /// Name of the Daemon Output file
    #[serde(default = "default_out")]
    pub daemon_output: String,
    /// Name of the Daemon Error file
    #[serde(default = "default_err")]
    pub daemon_error: String,
    /// Name of the Daemon pid file
    #[serde(default = "default_pid")]
    pub daemon_pid: String,
    pub proxy: Option<String>,
    pub database: DBConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DBConfig {
    pub username: String,
    pub password: Option<String>,
    pub port: u16,
    pub endpoint: String,
    pub dbname: String,
}

/// get message(s) from the socket
pub fn get_messages(s: &Socket) -> BynarResult<Vec<u8>> {
    let _id = s.recv_bytes(0)?;
    if s.get_rcvmore()? {
        return Ok(s.recv_bytes(0)?);
    }
    Ok(vec![])
}

#[macro_export]
/// Create a new Operation
macro_rules! make_op {
    ($op_type: ident) => {{
        let mut o = Operation::new();
        o.set_Op_type(Op::$op_type);
        o
    }};
    ($op_type:ident, $disk_path:expr) => {{
        let mut o = Operation::new();
        o.set_Op_type(Op::$op_type);
        o.set_disk($disk_path);
        o
    }};
    ($op_type:ident, $disk_path:expr, $simulate:expr) => {{
        let mut o = Operation::new();
        o.set_Op_type(Op::$op_type);
        o.set_disk($disk_path);
        o.set_simulate($simulate);
        o
    }};
    ($op_type:ident, $disk_path:expr, $simulate:expr, $id:expr) => {{
        let mut o = Operation::new();
        o.set_Op_type(Op::$op_type);
        o.set_disk($disk_path);
        o.set_simulate($simulate);
        if let Some(osd_id) = $id {
            o.set_osd_id(osd_id);
        }
        o
    }};
}

#[macro_export]
/// get the first instance of a message type
macro_rules! get_first_instance {
    ($message:expr, $mess_type:ty) => {{
        let mut copy = $message.clone();
        if copy.is_empty() {
            return None;
        }
        while !copy.is_empty() {
            if let Ok(mess) = parse_from_bytes::<$mess_type>(&copy) {
                let bytes = mess.write_to_bytes().unwrap();
                let size = bytes.len();
                //println!("compare {:?} with {:?}", bytes, copy);
                if $message.starts_with(&bytes) {
                    $message.drain(0..size);
                    return Some(mess);
                }
                // we can't error out early since
                // the tag/wire bits are at the end and we can't tell
                // how long a message might be or what kind(s) are in the vec
            }
            // parse from bytes grabs from the end of the byte array
            //so, remove half the length of bytes from the end of the message and try again
            copy.drain((copy.len() - 1)..copy.len());
        }
        None
    }};
}

pub fn get_first_op_result(message: &mut Vec<u8>) -> Option<OpOutcomeResult> {
    get_first_instance!(message, OpOutcomeResult)
}

/// get the list of JIRA tickets from disk-manager
pub fn get_jira_tickets(s: &Socket, client_id: Vec<u8>) -> BynarResult<()> {
    debug!("Printing ID {:?}", client_id);
    let mut o = Operation::new();
    debug!("calling get_jira_tickets ");
    o.set_Op_type(Op::GetCreatedTickets);
    debug!("Sending message in get_jira_tickets");
    request(s, o, client_id)?;
    Ok(())
}
