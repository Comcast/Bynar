use serde_derive::*;

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::fs::{create_dir, read_to_string, File};
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::process::Command;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use api::service::{
    Disk, DiskType, Disks, JiraInfo, Op, OpJiraTicketsResult, OpOutcome, OpOutcomeResult, OpResult,
    Operation, Partition, PartitionInfo, ResultType,
};
mod backend;
mod in_progress;
mod test_disk;
#[macro_use]
mod util;

use crate::backend::BackendType;
use block_utils::{Device, MediaType};
use clap::{crate_authors, crate_version, App, Arg};
use crossbeam::*;
use daemonize::Daemonize;
use gpt::{disk, header::read_header, partition::read_partitions};
use hashicorp_vault::client::VaultClient;
use helpers::{error::*, host_information::Host, ConfigSettings};
use hostname::get_hostname;
use libc::c_int;
use log::{debug, error, info, trace, warn};
use protobuf::parse_from_bytes;
use protobuf::Message as ProtobufMsg;
use protobuf::RepeatedField;
use signal_hook::iterator::Signals;
use signal_hook::*;
use simplelog::{CombinedLogger, Config, SharedLogger, TermLogger, WriteLogger};
use slack_hook::{PayloadBuilder, Slack};
use zmq::Socket;

// Create the request map for the disk-manager
fn create_req_map() -> BynarResult<HashMap<PathBuf, Option<Op>>> {
    // List out currently mounted block_devices
    let mut devices: Vec<PathBuf> = block_utils::get_block_devices()?
        .into_iter()
        .filter(|b| {
            !(if let Some(p) = b.as_path().file_name() {
                (p.to_string_lossy().starts_with("sr") || p.to_string_lossy().starts_with("loop"))
            } else {
                true
            })
        })
        .collect();
    let mut partitions = block_utils::get_block_partitions()?;
    devices.append(&mut partitions);
    // add all devices to the HashMap (initialize the Hashmap)
    let mut req_map: HashMap<PathBuf, Option<Op>> = HashMap::new();
    devices.iter().for_each(|device| {
        req_map.insert(device.to_path_buf(), None);
    });
    Ok(req_map)
}

#[macro_export]
/// Get the disk path of an Operation/OpOutcomeResult (any protobuf message with a get_disk() where disk is the diskpath) as a PathBuf
macro_rules! get_op_pathbuf {
    ($op:expr) => {{
        let disk = $op.get_disk();
        PathBuf::from(disk)
    }};
}

// check if a disk already has a request.  Return true if an op is already running (false otherwise or if
// op is List or GetCreatedTickets)
macro_rules! op_running {
    ($req_map:expr,$op:expr) => {{
        match $op.get_Op_type() {
            Op::List | Op::GetCreatedTickets => false,
            _ => {
                if let Some(o) = $req_map.get(&get_op_pathbuf!($op)) {
                    o.is_some()
                } else {
                    false
                }
            }
        }
    }};
    ($req_map:expr,$op:expr,$is_result:expr) => {{
        match $op.get_op_type() {
            Op::List | Op::GetCreatedTickets => $is_result, //handle List or GetCreatedTickets...
            _ => {
                if let Some(o) = $req_map.get(&get_op_pathbuf!($op)) {
                    o.is_some()
                } else {
                    false
                }
            }
        }
    }};
}

macro_rules! set_outcome_result {
    (err => $result:ident, $outcome:expr) => {{
        $result.set_result(ResultType::ERR);
        $result.set_error_msg($outcome);
    }};
    (ok =>$result:ident,  $outcome:expr) => {{
        $result.set_result(ResultType::OK);
        $result.set_outcome($outcome);
    }};
    (ok => $result:ident, $outcome:expr, $val:expr) => {{
        $result.set_value($val);
        set_outcome_result!(ok => $result, $outcome)
    }};
}

// Note: if the operation is List or GetCreatedTickets, skip adding it to the map
// REQUIRES: assert!(!is_op_running(req_map, op))
// ENSURES: assert!(is_op_running(req_map, op)) (if op_type != List || GetCreatedTickets)
fn op_insert(req_map: &mut HashMap<PathBuf, Option<Op>>, op: &Operation) {
    assert!(!op_running!(req_map, op));
    // if op_type is List or GetCreatedTickets, skip
    match op.get_Op_type() {
        Op::List | Op::GetCreatedTickets => return,
        _ => {
            req_map.insert(get_op_pathbuf!(op), Some(op.get_Op_type())); //no getting around the clone here unfortunately...
        }
    }
    assert!(op_running!(req_map, op));
}

// send a notification to slack channel (if config has webhook)
fn notify_slack(config: &DiskManagerConfig, msg: &str) -> BynarResult<()> {
    let c = config.clone();
    let slack = Slack::new(c.slack_webhook.expect("slack webhook option is None").as_ref())?;
    let slack_channel = c.slack_channel.unwrap_or_else(|| "".to_string());
    let bot_name = c.slack_botname.unwrap_or_else(|| "".to_string());
    let p = PayloadBuilder::new().text(msg).channel(slack_channel).username(bot_name).build()?;

    let res = slack.send(&p);
    match res {
        Ok(_) => debug!("Slack notified"),
        Err(e) => error!("Slack error: {:?}", e),
    };
    Ok(())
}

// default filename for daemon_output
fn default_out() -> String {
    "disk_manager_daemon.out".to_string()
}
// default filename for daemon_err
fn default_err() -> String {
    "disk_manager_daemon.err".to_string()
}
//default filename for daemon_pid
fn default_pid() -> String {
    "disk_manager_daemon.pid".to_string()
}

#[derive(Clone, Debug, Deserialize)]
struct DiskManagerConfig {
    backend: BackendType,
    vault_token: Option<String>,
    vault_endpoint: Option<String>,
    /// Name of the Daemon Output file
    #[serde(default = "default_out")]
    pub daemon_output: String,
    /// Name of the Daemon Error file
    #[serde(default = "default_err")]
    pub daemon_error: String,
    /// Name of the Daemon pid file
    #[serde(default = "default_pid")]
    pub daemon_pid: String,
    /// Optional Slack webhook (does not have to be the same as main client webhook)
    slack_webhook: Option<String>,
    slack_channel: Option<String>,
    slack_botname: Option<String>,
}

fn convert_media_to_disk_type(m: &MediaType) -> DiskType {
    match *m {
        MediaType::Loopback => DiskType::LOOPBACK,
        MediaType::LVM => DiskType::LVM,
        MediaType::MdRaid => DiskType::MDRAID,
        MediaType::NVME => DiskType::NVME,
        MediaType::Ram => DiskType::RAM,
        MediaType::Rotational => DiskType::ROTATIONAL,
        MediaType::SolidState => DiskType::SOLID_STATE,
        MediaType::Unknown => DiskType::UNKNOWN,
        MediaType::Virtual => DiskType::VIRTUAL,
    }
}

fn setup_curve(s: &Socket, config_dir: &Path, vault: bool) -> BynarResult<()> {
    // will raise EINVAL if not linked against libsodium
    // The ubuntu package is linked so this shouldn't fail
    s.set_curve_server(true)?;
    let keypair = zmq::CurveKeyPair::new()?;
    let hostname =
        get_hostname().ok_or_else(|| Error::new(ErrorKind::Other, "hostname not found"))?;
    let key_file = config_dir.join(format!("{}.pem", hostname));
    if vault {
        //Connect to vault
        let config: DiskManagerConfig = helpers::load_config(&config_dir, "disk-manager.json")?;
        if config.vault_token.is_none() || config.vault_endpoint.is_none() {
            error!("Vault support requested but vault_token or vault_endpoint aren't set");
            return Err(BynarError::new(
                "vault_token or vault_endpoint must be set for vault support".to_string(),
            ));
        }
        let endpoint = config.vault_endpoint.unwrap();
        let token = config.vault_token.unwrap();
        debug!("Connecting to vault to save the public key to /bynar/{}.pem", hostname);
        let client = VaultClient::new(endpoint.as_str(), token)?;
        client.set_secret(
            format!("{}/{}.pem", config_dir.display(), hostname),
            String::from_utf8_lossy(&keypair.public_key),
        )?;
        s.set_curve_secretkey(&keypair.secret_key)?;
    } else {
        debug!("Creating new curve keypair");
        s.set_curve_secretkey(&keypair.secret_key)?;
        let mut f = File::create(key_file)?;
        f.write_all(&keypair.public_key)?;
    }
    debug!("Server mechanism: {:?}", s.get_mechanism());
    debug!("Curve server: {:?}", s.is_curve_server());

    Ok(())
}

// check if operation does not have disk.  If it doesn't, return true, else false
fn op_no_disk(responder: &Socket, op: &Operation) -> bool {
    if !op.has_disk() {
        match op.get_Op_type() {
            Op::Add | Op::AddPartition | Op::Remove | Op::SafeToRemove => {
                error!("Add operation must include disk field.  Ignoring request")
            }
            _ => return false,
        }
        // We still have to respond with an error message
        let mut result = OpOutcomeResult::new();
        set_outcome_result!(err => result, "missing operation field in protocol. Ignoring request".to_string());
        let _ = respond_to_client(&result, &responder);
        return true;
    }
    false
}

/*
Server that manages disks
*/
fn listen(
    backend_type: &backend::BackendType,
    config_dir: &Path,
    listen_address: &str,
    signals: &Signals,
    daemon: bool,
    vault: bool,
) -> BynarResult<()> {
    debug!("Starting zmq listener with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let responder = context.socket(zmq::SocketType::STREAM)?;

    debug!("Listening on tcp://{}:5555", listen_address);
    // Fail to start if this fails
    setup_curve(&responder, config_dir, vault)?;
    assert!(responder.bind(&format!("tcp://{}:5555", listen_address)).is_ok());

    debug!("Building thread pool");
    //Note, for now we are using 16 threads by default
    let pool = rayon::ThreadPoolBuilder::new().num_threads(16).build()?;
    // channel to send results from backend to main thread
    let (send_res, recv_res) = crossbeam_channel::unbounded::<(Vec<u8>, OpOutcomeResult)>();
    let (send_disk, recv_disk) = crossbeam_channel::unbounded::<(Vec<u8>, Disks)>();
    let (send_ticket, recv_ticket) =
        crossbeam_channel::unbounded::<(Vec<u8>, OpJiraTicketsResult)>();

    debug!("Create request map");
    let mut req_map = create_req_map()?;
    let mut messages: VecDeque<(Operation, Vec<u8>)> = VecDeque::new();
    'outer: loop {
        let now = Instant::now();
        match responder.get_events() {
            Ok(events) => {
                // is the socket readable?
                if events.contains(zmq::PollEvents::POLLIN) {
                    //get the id first {STREAM sockets get messages with id prepended}
                    let client_id = responder.recv_bytes(0)?; //leave as Vec<u8>, not utf8 friendly
                    trace!("Client ID {:?}", client_id);
                    // get actual message
                    while responder.get_rcvmore()? {
                        let mut msg = responder.recv_bytes(0)?;
                        debug!("Got msg len: {}", msg.len());
                        trace!("Parsing msg {:?} as hex", msg);
                        if msg.is_empty() {
                            continue; // its the ID message, so skip
                        }
                        while !msg.is_empty() {
                            let operation = match parse_from_bytes::<Operation>(&msg.clone()) {
                                Ok(bytes) => bytes,
                                Err(e) => {
                                    error!("Failed to parse_from_bytes {:?}.  Ignoring request", e);
                                    continue;
                                }
                            };
                            let size = operation.write_to_bytes()?.len();
                            msg.drain((msg.len() - size)..msg.len());
                            let client_id = client_id.clone();
                            debug!("Operation requested: {:?}", operation.get_Op_type());
                            if op_no_disk(&responder, &operation) {
                                continue;
                            }
                            // check if op is currently running.  If so, skip it
                            if op_running!(&req_map, &operation) {
                                trace!(
                            "Operation {:?} cannot be run, disk is already running an operation",
                            operation
                        );
                                trace!("Operations: {:?}", req_map);
                                let mut op_res = OpOutcomeResult::new();
                                op_res.set_disk(operation.get_disk().to_string());
                                op_res.set_op_type(operation.get_Op_type());
                                set_outcome_result!(ok => op_res, OpOutcome::SkipRepeat, false);
                                let _ = send_res.send((client_id, op_res)); // this shouldn't error unless the channel breaks
                                continue;
                            }
                            op_insert(&mut req_map, &operation);
                            messages.push_back((operation, client_id));
                        }
                    }
                }
                if !messages.is_empty() {
                    for _ in 0..messages.len() {
                        let (operation, client_id) = messages.pop_front().unwrap(); //this should be safe assuming !empty
                        let client_id = client_id.clone();
                        let (send_res, send_disk, send_ticket) =
                            (send_res.clone(), send_disk.clone(), send_ticket.clone());
                        let backend_type = backend_type.clone();
                        let config_dir = config_dir.to_path_buf();
                        match operation.get_Op_type() {
                            Op::Add => {
                                let id = if operation.has_osd_id() {
                                    Some(operation.get_osd_id())
                                } else {
                                    None
                                };
                                pool.spawn(move || {
                                    let disk = operation.get_disk();
                                    match add_disk(
                                        &send_res,
                                        disk,
                                        &backend_type,
                                        id,
                                        config_dir.to_path_buf(),
                                        client_id,
                                    ) {
                                        Ok(_) => {
                                            info!("Add disk finished");
                                        }
                                        Err(e) => {
                                            error!("Add disk error: {:?}", e);
                                        }
                                    }
                                });
                            }
                            Op::AddPartition => {
                                //
                            }
                            Op::List => {
                                pool.spawn(move || {
                                    match list_disks(&send_disk, client_id) {
                                        Ok(_) => {
                                            info!("List disks finished");
                                        }
                                        Err(e) => {
                                            error!("List disks error: {:?}", e);
                                        }
                                    };
                                });
                            }
                            Op::Remove => {
                                let mut result = OpOutcomeResult::new();
                                result.set_disk(operation.get_disk().to_string());
                                result.set_op_type(Op::Remove);

                                pool.spawn(move || {
                            match safe_to_remove(
                                &Path::new(operation.get_disk()),
                                &backend_type,
                                &config_dir,
                            ) {
                                Ok((OpOutcome::Success, true)) => {
                                    match remove_disk(
                                        &send_res,
                                        operation.get_disk(),
                                        &backend_type,
                                        &config_dir,
                                        client_id,
                                    ) {
                                        Ok(_) => {
                                            info!("Remove disk finished");
                                        }
                                        Err(e) => {
                                            error!("Remove disk error: {:?}", e);
                                        }
                                    };
                                }
                                Ok((OpOutcome::Skipped, val)) => {
                                    debug!("Disk skipped");
                                    set_outcome_result!(ok => result, OpOutcome::Skipped, val);
                                    let _ = send_res.send((client_id, result));
                                }
                                Ok((OpOutcome::SkipRepeat, val)) => {
                                    debug!("Disk skipped, safe to remove already ran");
                                    result.set_outcome(OpOutcome::SkipRepeat);
                                    result.set_value(val);
                                    result.set_result(ResultType::OK);
                                    let _ = send_res.send((client_id, result));
                                }
                                Ok((_, false)) => {
                                    debug!("Disk is not safe to remove");
                                    //Response to client
                                    result.set_value(false);
                                    result.set_outcome(OpOutcome::Success);
                                    result.set_result(ResultType::ERR);
                                    result.set_error_msg("Not safe to remove disk".to_string());
                                    let _ = send_res.send((client_id, result));
                                }
                                Err(e) => {
                                    error!("safe to remove failed: {:?}", e);
                                    // Response to client
                                    result.set_value(false);
                                    result.set_result(ResultType::ERR);
                                    result.set_error_msg(e.to_string());
                                    let _ = send_res.send((client_id, result));
                                }
                            };
                        });
                            }
                            Op::SafeToRemove => {
                                pool.spawn(move || {
                                    match safe_to_remove_disk(
                                        &send_res,
                                        operation.get_disk(),
                                        &backend_type,
                                        &config_dir,
                                        client_id,
                                    ) {
                                        Ok(_) => {
                                            info!("Safe to remove disk finished");
                                        }
                                        Err(e) => {
                                            error!("Safe to remove error: {:?}", e);
                                        }
                                    };
                                });
                            }
                            Op::GetCreatedTickets => {
                                match get_jira_tickets(&send_ticket, &config_dir, client_id) {
                                    Ok(_) => {
                                        info!("Fetching jira tickets finished");
                                    }
                                    Err(e) => {
                                        error!("Fetching jira error: {:?}", e);
                                    }
                                };
                            }
                        }
                    }
                }
                if events.contains(zmq::PollEvents::POLLOUT) {
                    //check disks first, since those are faster requests than add/remove reqs
                    match recv_disk.try_recv() {
                        Ok((client_id, result)) => {
                            // send result back to client
                            let _ = responder.send(&client_id, zmq::SNDMORE);
                            let _ = respond_to_client(&result, &responder);
                        }
                        Err(_) => {
                            // check if there are tickets (also takes a while, but not as long as add/remove/safe-to-remove)
                            match recv_ticket.try_recv() {
                                Ok((client_id, result)) => {
                                    let _ = responder.send(&client_id, zmq::SNDMORE);
                                    let _ = respond_to_client(&result, &responder);
                                }
                                Err(_) => {
                                    // no disks in the queue, check if any add/remove/safe-to-remove req results
                                    if let Ok((client_id, result)) = recv_res.try_recv() {
                                        //check if result is SkipRepeat, if so, skipp the assert! and insert
                                        debug!("Send {:?}", result);
                                        if OpOutcome::SkipRepeat != result.get_outcome() {
                                            assert!(op_running!(req_map, &result, true));
                                        }
                                        // set entry in req_map to None
                                        req_map.insert(get_op_pathbuf!(&result), None);
                                        let _ = responder.send(&client_id, zmq::SNDMORE);
                                        let _ = respond_to_client(&result, &responder);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(zmq::Error::EBUSY) => {
                debug!("Socket Busy, skip");
            }
            Err(e) => {
                error!("Get Client Socket Events errored...{:?}", e);
                return Err(BynarError::from(e));
            }
        }

        if daemon {
            while now.elapsed() < Duration::from_millis(10) {
                for signal in signals.pending() {
                    match signal as c_int {
                        signal_hook::SIGHUP => {
                            // Don't actually need to reload the config, since it gets reloaded on every call to backend...
                            debug!("Requested to reload config file");
                            let config: DiskManagerConfig =
                                match helpers::load_config(&config_dir, "disk-manager.json") {
                                    Ok(p) => p,
                                    Err(e) => {
                                        error!("Failed to load config file {}", e);
                                        continue;
                                    }
                                };
                            notify_slack(
                                    &config,
                                    &"Requested to reload config, ignoring request: config changes already loaded".to_string(),
                                )
                                .expect("Unable to connect to slack");
                        }
                        signal_hook::SIGINT | signal_hook::SIGCHLD => {
                            //skip this
                            debug!("Ignore signal");
                            continue;
                        }
                        signal_hook::SIGTERM => {
                            //"gracefully" exit
                            debug!("Exit Process");
                            break 'outer;
                        }
                        _ => unreachable!(),
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        } else {
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

// send message to client
fn respond_to_client<T: protobuf::Message>(result: &T, s: &Socket) -> BynarResult<()> {
    let encoded = result.write_to_bytes()?;
    debug!("Responding to client with msg len: {}", encoded.len());
    s.send(&encoded, 0)?;
    Ok(())
}

#[test]
fn test_set_outcome_result() {
    let mut result = OpOutcomeResult::new();
    result.set_disk("/dev/sdc".to_string());
    result.set_op_type(Op::Add);
    set_outcome_result!(err => result, "Error was set".to_string());
    println!("Error Outcome: {:#?}", result);
    let mut result = OpOutcomeResult::new();
    result.set_disk("/dev/sdc".to_string());
    result.set_op_type(Op::Add);
    let outcome = OpOutcome::Success;
    set_outcome_result!(ok => result, outcome);
    println!("Success Outcome: {:#?}", result);
    let mut result = OpOutcomeResult::new();
    result.set_disk("/dev/sdc".to_string());
    result.set_op_type(Op::Add);
    set_outcome_result!(ok => result, outcome, true);
    println!("Success Outcome: {:#?}", result);
}

// add disk request function.  Send the result through the sender channel back to the main thread.
fn add_disk(
    sender: &crossbeam_channel::Sender<(Vec<u8>, OpOutcomeResult)>,
    d: &str,
    backend: &BackendType,
    id: Option<u64>,
    config_dir: PathBuf,
    client_id: Vec<u8>,
) -> BynarResult<()> {
    let mut result = OpOutcomeResult::new();
    result.set_disk(d.to_string());
    result.set_op_type(Op::Add);
    let backend = match backend::load_backend(backend, Some(&config_dir)) {
        Ok(backend) => backend,
        Err(e) => {
            set_outcome_result!(err => result, e.to_string());
            // Bail early.  We can't load the backend
            let _ = sender.send((client_id, result));
            return Ok(());
        }
    };

    //Send back OpOutcomeResult
    match backend.add_disk(&Path::new(d), id, false) {
        Ok(outcome) => {
            set_outcome_result!(ok => result, outcome);
        }
        Err(e) => {
            set_outcome_result!(err => result, e.to_string());
        }
    };
    let _ = sender.send((client_id, result));

    Ok(())
}

fn get_disks() -> BynarResult<Vec<Disk>> {
    let mut disks: Vec<Disk> = Vec::new();
    debug!("Searching for block devices");
    let devices = block_utils::get_block_devices()?;

    debug!("Gathering udev info on block devices");
    // Gather info on all devices and skip Loopback devices
    let device_info: Vec<Device> =
        block_utils::get_all_device_info(devices.as_slice())?.into_iter().collect();
    debug!("Device info found: {:?}", device_info);

    debug!("Gathering partition info");

    for device in device_info {
        let mut d = Disk::new();
        let dev_path = Path::new("/dev/").join(device.name);
        // This will skip partition_info if it fails to gather.  Blank disks will fail
        let p = get_partition_info(&dev_path).unwrap_or_else(|_| PartitionInfo::new());
        //Translate block_utils MediaType -> Protobuf DiskType
        d.set_field_type(convert_media_to_disk_type(&device.media_type));
        d.set_dev_path(dev_path.to_string_lossy().into_owned());
        d.set_partitions(p);
        if let Some(serial) = device.serial_number {
            d.set_serial_number(serial);
        }
        disks.push(d);
    }

    Ok(disks)
}

fn get_partition_info(dev_path: &Path) -> BynarResult<PartitionInfo> {
    let mut partition_info = PartitionInfo::new();
    let h = read_header(dev_path, disk::DEFAULT_SECTOR_SIZE)?;
    let partitions = read_partitions(dev_path, &h, disk::DEFAULT_SECTOR_SIZE)?;

    // Transform partitions to protobuf
    let proto_parts: Vec<Partition> = partitions
        .iter()
        .map(|part| {
            let mut p = Partition::new();
            p.set_uuid(part.1.part_guid.to_hyphenated().to_string());
            p.set_first_lba(part.1.first_lba);
            p.set_last_lba(part.1.last_lba);
            p.set_flags(part.1.flags);
            p.set_name(part.1.name.clone());
            p
        })
        .collect();
    partition_info.set_partition(RepeatedField::from_vec(proto_parts));
    Ok(partition_info)
}

fn list_disks(
    c: &crossbeam_channel::Sender<(Vec<u8>, Disks)>,
    client_id: Vec<u8>,
) -> BynarResult<()> {
    let disk_list: Vec<Disk> = get_disks()?;

    let mut disks = Disks::new();
    disks.set_disk(RepeatedField::from_vec(disk_list));
    let _ = c.send((client_id, disks));
    Ok(())
}

fn remove_disk(
    sender: &crossbeam_channel::Sender<(Vec<u8>, OpOutcomeResult)>,
    d: &str,
    backend: &BackendType,
    config_dir: &Path,
    client_id: Vec<u8>,
) -> BynarResult<()> {
    //Returns OpOutcomeResult
    let mut result = OpOutcomeResult::new();
    result.set_disk(d.to_string());
    result.set_op_type(Op::Remove);
    let backend = match backend::load_backend(backend, Some(config_dir)) {
        Ok(b) => b,
        Err(e) => {
            set_outcome_result!(err => result, e.to_string());
            // Bail early.  We can't load the backend
            let _ = sender.send((client_id, result));
            return Ok(());
        }
    };
    match backend.remove_disk(&Path::new(d), false) {
        Ok(outcome) => {
            set_outcome_result!(ok => result, outcome);
        }
        Err(e) => {
            set_outcome_result!(err => result, e.to_string());
        }
    };
    let _ = sender.send((client_id, result));
    Ok(())
}

fn safe_to_remove(
    d: &Path,
    backend: &BackendType,
    config_dir: &Path,
) -> BynarResult<(OpOutcome, bool)> {
    let backend = backend::load_backend(backend, Some(config_dir))?;
    let safe = backend.safe_to_remove(d, false)?;
    Ok(safe)
}

fn safe_to_remove_disk(
    sender: &crossbeam_channel::Sender<(Vec<u8>, OpOutcomeResult)>,
    d: &str,
    backend: &BackendType,
    config_dir: &Path,
    client_id: Vec<u8>,
) -> BynarResult<()> {
    debug!("Checking if {} is safe to remove", d);
    let mut result = OpOutcomeResult::new();
    result.set_disk(d.to_string());
    result.set_op_type(Op::SafeToRemove);
    match safe_to_remove(&Path::new(d), &backend, &config_dir) {
        Ok((outcome, val)) => {
            debug!("Safe to remove: {}", val);
            set_outcome_result!(ok => result, outcome, val);
        }
        Err(e) => {
            debug!("Safe to remove err: {}", e);
            set_outcome_result!(err => result, e.to_string());
            let _ = sender.send((client_id, result));
            return Err(BynarError::new(format!("safe to remove error: {}", e)));
        }
    };
    let _ = sender.send((client_id, result));
    Ok(())
}

pub fn get_jira_tickets(
    sender: &crossbeam_channel::Sender<(Vec<u8>, OpJiraTicketsResult)>,
    config_dir: &Path,
    client_id: Vec<u8>,
) -> BynarResult<()> {
    let mut result = OpJiraTicketsResult::new();
    let config: ConfigSettings = match helpers::load_config(&config_dir, "bynar.json") {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to load config file {}", e);
            set_outcome_result!(err => result, e.to_string());
            // unable to load config file
            let _ = sender.send((client_id, result));
            return Ok(());
        }
    };
    let db_config = config.database;
    let db_pool = match in_progress::create_db_connection_pool(&db_config) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create database pool {}", e);
            set_outcome_result!(err => result, e.to_string());
            // unable to create DB connection
            let _ = sender.send((client_id, result));
            return Ok(());
        }
    };

    info!("Getting all pending repair tickets");
    let tickets = in_progress::get_all_pending_tickets(&db_pool)?;
    debug!("outstanding tickets: {:?}", tickets);
    result.set_result(ResultType::OK);
    let proto_jira: Vec<JiraInfo> = tickets
        .iter()
        .map(|j| {
            let mut jira_result = JiraInfo::new();
            jira_result.set_ticket_id(j.ticket_id.clone());
            let host_name = in_progress::get_host_name(&db_pool, j.device_id);
            jira_result.set_server_name(host_name.unwrap().unwrap());
            jira_result
        })
        .collect();
    result.set_tickets(RepeatedField::from_vec(proto_jira));
    let _ = sender.send((client_id, result));
    Ok(())
}

fn main() {
    let matches = App::new("Disk Manager")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Detect dead hard drives, create a support ticket and watch for resolution")
        .arg(
            Arg::with_name("backend")
                .default_value("ceph")
                .help("Backend cluster type to manage disks for")
                .long("backend")
                .possible_values(&["ceph"])
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("listen")
                .default_value("*")
                .help("Address to listen on.  Default is all interfaces")
                .long("listenaddress")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("configdir")
                .default_value("/etc/bynar")
                .help("The directory where all config files can be found")
                .long("configdir")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("log")
                .default_value("/var/log/bynar-disk-manager.log")
                .help("Default log file location")
                .long("logfile")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("vault")
                .default_value("false")
                .help("Enable vault support. Remember to set the vault_token and vault_endpoint")
                .long("vault")
                .possible_values(&["true", "false"])
                .takes_value(true)
                .required(false),
        )
        .arg(Arg::with_name("v").short("v").multiple(true).help("Sets the level of verbosity"))
        .arg(Arg::with_name("daemon").help("Run Bynar as a daemon").long("daemon").required(false))
        .get_matches();
    let daemon = matches.is_present("daemon");
    let level = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Info, //default
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    let log = Path::new(matches.value_of("log").unwrap());
    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![];
    if let Some(term_logger) = TermLogger::new(level, Config::default()) {
        //systemd doesn't use a terminal
        loggers.push(term_logger);
    }
    loggers.push(WriteLogger::new(
        level,
        Config::default(),
        File::create(log).expect("log file creation failed"),
    ));
    let _ = CombinedLogger::init(loggers);
    //Sanity check
    let config_dir = Path::new(matches.value_of("configdir").unwrap());
    if !config_dir.exists() {
        warn!("Config directory {} doesn't exist. Creating", config_dir.display());
        if let Err(e) = create_dir(config_dir) {
            error!("Unable to create directory {}: {}", config_dir.display(), e.to_string());
            return;
        }
    }
    let config = helpers::load_config::<DiskManagerConfig>(config_dir, "disk-manager.json");
    if let Err(e) = config {
        error!(
            "Failed to load config file {}. error: {}",
            config_dir.join("disk-manager.json").display(),
            e
        );
        return;
    }
    let config: DiskManagerConfig = config.expect("Failed to load config");
    let pidfile = format!("/var/log/{}", config.daemon_pid);

    //check if the pidfile exists
    let pidpath = Path::new(&pidfile);
    if pidpath.exists() {
        //open pidfile and check if process with pid exists
        let pid = read_to_string(pidpath).expect("Unable to read pid from pidfile");
        let output = Command::new("ps")
            .args(&["-p", &pid])
            .output()
            .expect("Unable to open shell to run ps command");
        if let Some(0) = output.status.code() {
            let out = String::from_utf8_lossy(&output.stdout);
            if out.contains("disk-manager") {
                //skip
                error!("There is already a running instance of disk-manager! Abort!");
                return;
            }
        }
    }
    let signals = Signals::new(&[
        signal_hook::SIGHUP,
        signal_hook::SIGTERM,
        signal_hook::SIGINT,
        signal_hook::SIGCHLD,
    ])
    .expect("Unable to create iterator signal handler");
    //Check if daemon, if so, start the daemon
    if daemon {
        let outfile = format!("/var/log/{}", config.daemon_output);
        let errfile = format!("/var/log/{}", config.daemon_error);

        let stdout =
            File::create(&outfile).unwrap_or_else(|_| panic!("{} creation failed", outfile));
        let stderr =
            File::create(&errfile).unwrap_or_else(|_| panic!("{} creation failed", errfile));

        trace!("I'm Parent and My pid is {}", process::id());

        let daemon = Daemonize::new()
            .pid_file(&pidfile) // Every method except `new` and `start`
            .chown_pid_file(true)
            .working_directory("/")
            .user("root")
            .group(2) // 2 is the bin user
            .umask(0o027) // Set umask, this gives 750 permission
            .stdout(stdout) // Redirect stdout
            .stderr(stderr) // Redirect stderr
            .exit_action(|| trace!("This is executed before master process exits"));

        match daemon.start() {
            Ok(_) => trace!("Success, daemonized"),
            Err(e) => eprintln!("Error, {}", e),
        }
        println!("I'm child process and My pid is {}", process::id());
    } else {
        signals.close();
    }

    info!("---------------------------------\nStarting up");
    let backend = BackendType::from_str(matches.value_of("backend").unwrap())
        .expect("unable to convert backend option to BackendType");
    let vault_support = {
        bool::from_str(matches.value_of("vault").unwrap())
            .expect("unable to convert vault option to bool")
    };
    let config = helpers::load_config(&config_dir, "disk-manager.json");
    if let Err(e) = config {
        error!(
            "Failed to load config file {}. error: {}",
            config_dir.join("disk-manager.json").display(),
            e
        );
        return;
    }
    let config: DiskManagerConfig = config.expect("Failed to load config");

    let h_info = Host::new();
    if h_info.is_err() {
        error!("Failed to gather host information");
        //gracefully exit
        return;
    }
    let host_info = h_info.expect("Failed to gather host information");
    match listen(
        &backend,
        config_dir,
        matches.value_of("listen").unwrap(),
        &signals,
        daemon,
        vault_support,
    ) {
        Ok(_) => {
            println!("Finished");
            notify_slack(
                &config,
                &format!("Disk-Manager Exited Successfully on host {}", host_info.hostname),
            )
            .expect("Unable to connect to slack");
        }
        Err(e) => {
            println!("Error: {:?}", e);
            notify_slack(
                &config,
                &format!("Disk-Manager Errored out on host {} with {:?}", host_info.hostname, e),
            )
            .expect("Unable to connect to slack");
        }
    };
}
