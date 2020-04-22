/*
@license
Copyright 2017 Comcast Cable Communications Management, LLC
Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at
http://www.apache.org/licenses/LICENSE-2.0
Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/
///#![cfg_attr(test, feature(test, proc_macro_mod))]
/// Detect dead disks in a ceph cluster
/// 1. Detect dead disk
/// 2. Report dead disk to JIRA for repairs
/// 3. Test for resolution
/// 4. Put disk back into cluster
mod create_support_ticket;
mod in_progress;
mod test_disk;
mod test_hardware;
#[macro_use]
mod util;

use crate::create_support_ticket::{create_support_ticket, ticket_resolved};
use crate::in_progress::*;
use crate::test_disk::{State, StateMachine};
use api::service::{Op, OpJiraTicketsResult, OpOutcome, OpOutcomeResult, Operation, ResultType};
use clap::{crate_authors, crate_version, App, Arg};
use daemonize::Daemonize;
use helpers::{error::*, get_first_instance, host_information::Host, ConfigSettings};
use libc::c_int;
use log::{debug, error, info, trace, warn};
use protobuf::parse_from_bytes;
use protobuf::Message as ProtobufMsg;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager as ConnectionManager;
use signal_hook::iterator::Signals;
use signal_hook::*;
use simplelog::{CombinedLogger, Config, SharedLogger, TermLogger, WriteLogger};
use slack_hook::{PayloadBuilder, Slack};
use zmq::Socket;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{create_dir, read_to_string, File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::process::Command;
use std::time::{Duration, Instant};

/// a specific operation and its outcome
#[derive(Debug, Clone)]
struct DiskOp {
    /// the operation type
    pub op_type: Op,
    /// the description for a JIRA ticket if necessary (None if not Safe-to-remove/Remove-disk)
    /// Or, if an add_disk request, description is the ticket_id
    pub description: Option<String>,
    /// the operation id in the database if one exists for Safe-To-Remove/Remove requst handling
    pub operation_id: Option<u32>,
    /// This value is None if the outcome has not yet been recieved
    pub ret_val: Option<OpOutcomeResult>,
}

impl DiskOp {
    /// create a new DiskOp from an operation, description, and operation id
    pub fn new(op: Operation, description: Option<String>, operation_id: Option<u32>) -> DiskOp {
        DiskOp { op_type: op.get_Op_type(), description, operation_id, ret_val: None }
    }
}

// create a message map to handle a list of disk-manager requests
// The message map is a nested HashMap, mapping a disk to a list of partitions (including the disk path itself), which maps associated operations in progress
fn create_msg_map(
    pool: &Pool<ConnectionManager>,
    host_mapping: &HostDetailsMapping,
) -> BynarResult<HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>> {
    // List out currently mounted block_devices, filtering out loop and cd/rom devices
    let mut devices: Vec<PathBuf> = block_utils::get_block_devices()?
        .into_iter()
        .filter(|block_device| {
            !(if let Some(path) = block_device.as_path().file_name() {
                (path.to_string_lossy().starts_with("sr")
                    || path.to_string_lossy().starts_with("loop"))
            } else {
                true
            })
        })
        .collect();
    let db_devices: Vec<PathBuf> =
        in_progress::get_devices_from_db(pool, host_mapping.storage_detail_id)?
            .into_iter()
            .map(|(_id, _name, path)| path)
            .collect();
    let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();

    // get a list of partition device paths
    let partitions: Vec<PathBuf> = db_devices
        .clone()
        .into_iter()
        .filter(|path| match block_utils::is_disk(path) {
            Err(_) => path.to_string_lossy().chars().last().unwrap().is_digit(10), // check if the last character is a digit (in case the disk is unmounted)
            Ok(is_disk) => is_disk,
        })
        .collect();
    // get the list of disk paths from the database devices
    let mut disks: Vec<PathBuf> = db_devices
        .into_iter()
        .filter(|path| match block_utils::is_disk(path) {
            Err(_) => !path.to_string_lossy().chars().last().unwrap().is_digit(10),
            Ok(is_disk) => is_disk,
        })
        .collect();
    devices.append(&mut disks);
    // for each block device add its partitions to the HashMap
    // add them to HashMap
    devices.iter().for_each(|device| {
        // make a new hashmap
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        disk_map.insert(device.to_path_buf(), None);
        // check if partition parent is device
        partitions
            .iter()
            .filter(|partition| {
                partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
            })
            .for_each(|partition| {
                disk_map.insert(partition.to_path_buf(), None);
            });
        map.insert(device.to_path_buf(), disk_map);
    });
    Ok(map)
}

// given a path, return a (parent,child), or (parent,parent) tuple to
// look through the request map with, or error out
fn get_request_keys(dev_path: &PathBuf) -> BynarResult<(PathBuf, &PathBuf)> {
    if let Some(parent) = block_utils::get_parent_devpath_from_path(dev_path)? {
        Ok((parent, dev_path))
    } else if dev_path.exists() {
        Ok((dev_path.to_path_buf(), dev_path))
    } else {
        // partition was destroyed...probably
        // make parent path
        let mut str_path = dev_path.to_string_lossy().to_string();
        // device and partition naming conventions have ssd and hard disks end in the partition number for a partition, remove the numbers and you get the disk path
        while str_path.chars().last().unwrap().is_digit(10) {
            str_path = str_path[0..str_path.len() - 1].to_string();
        }
        let path = PathBuf::from(str_path.to_string());
        if path.exists() {
            Ok((path, dev_path)) // partition probably
        } else if str_path.starts_with("/dev/sd")
            || str_path.starts_with("/dev/hd")
            || str_path.starts_with("/dev/nvme")
        //note nvme devices are slightly different in naming convention
        {
            Ok((dev_path.to_path_buf(), dev_path)) // this is the disk path, unless the path is an nvme device
        } else {
            // path just doesn't exist, so error...
            error!("Path {} does not exist, nor does its parent.", dev_path.display());
            Err(BynarError::from(format!(
                "Path {} does not exist, nor does its parent.",
                dev_path.display()
            )))
        }
    }
}
// add or update an operation to the message map.  If an operation is already ongoing, update op and return the old operation
fn add_or_update_map_op(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
    op: Option<DiskOp>,
) -> BynarResult<Option<DiskOp>> {
    let (parent, dev_path) = get_request_keys(dev_path)?;
    if let Some(disk) = message_map.get_mut(&parent) {
        if let Some(partition) = disk.clone().get(dev_path) {
            // partition in map
            disk.insert(dev_path.to_path_buf(), op);
            return Ok(partition.clone());
        }
        if &parent == dev_path {
            // if exists Some(disk) then dev_path should also exist (since creation) of entry in map requires it
            error!("Map is missing the disk entry but disk {} exists in the map", parent.display());
            return Err(BynarError::from(format!(
                "Map is missing the disk entry but disk {} exists in the map",
                parent.display()
            )));
        }
        disk.insert(dev_path.to_path_buf(), op);
    } else {
        //add to map
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        disk_map.insert(parent.to_path_buf(), None);
        // check if partition parent is device
        block_utils::get_block_partitions()?
            .iter()
            .filter(|partition| {
                partition.to_string_lossy().contains(&parent.to_string_lossy().to_string())
            })
            .for_each(|partition| {
                disk_map.insert(partition.to_path_buf(), None);
            });
        disk_map.insert(dev_path.to_path_buf(), op);
        message_map.insert(parent.to_path_buf(), disk_map);
    }
    Ok(None)
}

// get the operation for a device (disk/partition) if one exists
fn get_map_op(
    message_map: &HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
) -> BynarResult<Option<DiskOp>> {
    let (parent, dev_path) = get_request_keys(dev_path)?;
    //parent is in the map
    if let Some(disk) = message_map.get(&parent) {
        if let Some(partition) = disk.get(dev_path) {
            // partition in map
            return Ok(partition.clone());
        }
    }
    Ok(None)
}

// replace the DiskOp associated with the input dev_path None and return the previous DiskOp
// If the dev_path is not in the map error out
fn remove_map_op(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
) -> BynarResult<Option<DiskOp>> {
    let (parent, dev_path) = get_request_keys(dev_path)?;
    //parent is in the map
    if let Some(disk) = message_map.get_mut(&parent) {
        if let Some(partition) = disk.clone().get(dev_path) {
            //set point as None
            disk.insert(dev_path.to_path_buf(), None);
            // partition in map
            return Ok(partition.clone());
        }
    }
    Err(BynarError::from(format!("Path {} is not in the message map", dev_path.display())))
}

// get the hashmap associated with a diskpath from the op map
fn get_disk_map_op(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
) -> BynarResult<HashMap<PathBuf, Option<DiskOp>>> {
    let (parent, _) = get_request_keys(dev_path)?;
    //parent is in the map
    if let Some(disk) = message_map.get(&parent) {
        return Ok(disk.clone());
    }
    Err(BynarError::from(format!("Path {} is not a disk in the map", dev_path.display())))
}

// Send a message to Slack
fn notify_slack(config: &ConfigSettings, msg: &str) -> BynarResult<()> {
    let conf = config.clone();
    let slack = Slack::new(conf.slack_webhook.expect("slack webhook option is None").as_ref())?;
    let slack_channel = conf.slack_channel.unwrap_or_else(|| "".to_string());
    let bot_name = conf.slack_botname.unwrap_or_else(|| "".to_string());
    let payload =
        PayloadBuilder::new().text(msg).channel(slack_channel).username(bot_name).build()?;

    let res = slack.send(&payload);
    match res {
        Ok(_) => debug!("Slack notified"),
        Err(e) => error!("Slack error: {:?}", e),
    };
    Ok(())
}

// get the public key needed to connect to the disk-manager
fn get_public_key(config: &ConfigSettings, host_info: &Host) -> BynarResult<Vec<u8>> {
    // If vault_endpoint and token are set we should get the key from vault
    // Otherwise we need to know where the public_key is located?
    if config.vault_endpoint.is_some() && config.vault_token.is_some() {
        let key = helpers::get_vault_token(
            config.vault_endpoint.as_ref().expect("vault endpoint is None").as_ref(),
            config.vault_token.as_ref().expect("vault_token is None").as_ref(),
            &host_info.hostname,
        )?;
        Ok(key.as_bytes().to_vec())
    } else {
        let p = Path::new("/etc").join("bynar").join(format!("{}.pem", host_info.hostname));
        if !p.exists() {
            error!("{} does not exist", p.display());
        }
        let mut f = File::open(p)?;
        let mut key = Vec::new();
        f.read_to_end(&mut key)?;
        Ok(key)
    }
}

// add the disk in the state machine's information to the description
fn add_disk_to_description(
    description: &mut String,
    dev_path: &Path,
    state_machine: &StateMachine,
) {
    description.push_str(&format!("\nDisk path: {}", dev_path.display()));
    if let Some(serial) = &state_machine.block_device.device.serial_number {
        description.push_str(&format!("\nDisk serial: {}", serial));
    }
    description.push_str(&format!(
        "\nSCSI host: {}, channel: {} id: {} lun: {}",
        state_machine.block_device.scsi_info.host,
        state_machine.block_device.scsi_info.channel,
        state_machine.block_device.scsi_info.id,
        state_machine.block_device.scsi_info.lun
    ));
    description
        .push_str(&format!("\nDisk vendor: {:?}", state_machine.block_device.scsi_info.vendor));
}

// run the state machine and check for failed disks.
// failed disks are sent to the message queue to check and attempt automatic removal
fn check_for_failed_disks(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    message_queue: &mut VecDeque<(Operation, Option<String>, Option<u32>)>,
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    host_mapping: &HostDetailsMapping,
    _simulate: bool,
) -> BynarResult<()> {
    //Host information to use in ticket creation
    let mut description = format!("A disk on {} failed. Please replace.", host_info.hostname);
    description.push_str(&format!(
        "\nHostname: {}\nServer type: {}\nServer Serial: {}\nMachine Architecture: {}\nKernel: {}",
        host_info.hostname,
        host_info.server_type,
        host_info.serial_number,
        host_info.machine_architecture,
        host_info.kernel,
    ));

    info!("Checking all drives");
    let all_states: BynarResult<Vec<_>> =
        test_disk::check_all_disks(&host_info, pool, host_mapping)?.into_iter().collect();
    // separate the states into Ok and Errors
    let usable_states: Vec<StateMachine> = match all_states {
        Ok(state) => state,
        Err(e) => {
            error!("check_all_disks failed with error: {:?}", e);
            return Err(BynarError::new(format!("check_all_disks failed with error: {:?}", e)));
        }
    };
    //filter all the disks that are in the WaitingForReplacement state and are not currently undergoing an operation
    let mut replacing: Vec<_> = usable_states
        .iter()
        .filter(|state_machine| {
            if state_machine.block_device.state == State::WaitingForReplacement {
                //check hashmap of the device path == None, or OpType != SafeToRemove || Remove
                match get_map_op(&message_map, &state_machine.block_device.dev_path).unwrap() {
                    Some(op) => {
                        // check if in_progress
                        info!("Connecting to database to check if disk is in progress");
                        let in_progress = in_progress::is_hardware_waiting_repair(
                            pool,
                            host_mapping.storage_detail_id,
                            &state_machine.block_device.dev_path.to_string_lossy(),
                            None,
                        )
                        .unwrap();
                        //check if op_type == SafeToRemove || Remove
                        !(op.op_type == Op::SafeToRemove || op.op_type == Op::Remove || in_progress)
                    }
                    None => true,
                }
            } else {
                false
            }
        })
        .collect();
    // add the partition state machines? to the replacing list
    let mut add_replacing = Vec::new();
    for state_machine in &replacing {
        if !state_machine.block_device.dev_path.exists() {
            //partition was deleted
            //add partition to the map
            add_or_update_map_op(message_map, &state_machine.block_device.dev_path, None)?;
        }
        let disks = get_disk_map_op(message_map, &state_machine.block_device.dev_path)?;
        // get list of keys in disks and filter usable list for keypath
        let mut add: Vec<_> = usable_states
            .iter()
            .filter(|state_machine| {
                if disks.contains_key(&state_machine.block_device.dev_path) {
                    //check hashmap of the device path == None, or OpType != SafeToRemove || Remove
                    match get_map_op(&message_map, &state_machine.block_device.dev_path).unwrap() {
                        Some(op) => {
                            // check if in_progress
                            info!("Connecting to database to check if disk is in progress");
                            let in_progress = in_progress::is_hardware_waiting_repair(
                                pool,
                                host_mapping.storage_detail_id,
                                &state_machine.block_device.dev_path.to_string_lossy(),
                                None,
                            )
                            .unwrap();
                            //check if op_type == SafeToRemove || Remove
                            !(op.op_type == Op::SafeToRemove
                                || op.op_type == Op::Remove
                                || in_progress)
                        }
                        None => true,
                    }
                } else {
                    false
                }
            })
            .collect();
        add_replacing.append(&mut add);
    }
    //combine with replacing, then do sort_unstable_by and dedup_rm
    replacing.append(&mut add_replacing);
    replacing.sort_unstable_by(|a, b| {
        a.block_device.dev_path.partial_cmp(&b.block_device.dev_path).unwrap()
    });
    replacing.dedup_by(|a, b| a.block_device.dev_path.eq(&b.block_device.dev_path));
    //filter Fail disks in seperate vec and soft-error those at the end before checking the errored_states
    let failed: Vec<_> = usable_states
        .iter()
        .filter(|state_machine| state_machine.block_device.state == State::Fail)
        .collect();

    replacing.iter().for_each(|state_machine| {
        // add safeToRemove + Remove request to message_queue, checking if its already in first
        // create Operation, description, and get the op_id
        let mut desc = description.clone();
        add_disk_to_description(&mut desc, &state_machine.block_device.dev_path, &state_machine);
        let op_id = match state_machine.block_device.operation_id {
            None => {
                error!(
                    "Operation not recorded for {}",
                    state_machine.block_device.dev_path.display()
                );
                0
            }
            Some(i) => i,
        };
        let op = helpers::make_op!(
            SafeToRemove,
            format!("{}", state_machine.block_device.dev_path.display())
        );
        let mess: (Operation, Option<String>, Option<u32>) = (op, Some(desc.clone()), Some(op_id));
        let op2 =
            helpers::make_op!(Remove, format!("{}", state_machine.block_device.dev_path.display()));
        let mess2: (Operation, Option<String>, Option<u32>) = (op2, Some(desc), Some(op_id));
        if !message_queue.contains(&mess) && !message_queue.contains(&mess2) {
            message_queue.push_back(mess);
            message_queue.push_back(mess2);
        }
    });
    failed.iter().for_each(|state_machine| {
        error!("Disk {} ended in a Fail state", state_machine.block_device.dev_path.display())
    });
    Ok(())
}

// Evaluate the hardware information returned from redfish
fn evaluate(
    results: Vec<BynarResult<()>>,
    config: &ConfigSettings,
    pool: &Pool<ConnectionManager>,
    host_mapping: &HostDetailsMapping,
) -> BynarResult<()> {
    for result in results {
        if let Err(e) = result {
            match e {
                // This is the error we're after
                BynarError::HardwareError(HardwareError {
                    ref name, ref serial_number, ..
                }) => {
                    let serial = serial_number.as_ref().map(|s| &**s);
                    let in_progress = in_progress::is_hardware_waiting_repair(
                        pool,
                        host_mapping.storage_detail_id,
                        name,
                        serial,
                    )?;
                    if !in_progress {
                        //file a ticket
                        debug!("Creating support ticket");
                        let mut op_info = OperationInfo::new(host_mapping.entry_id, 0);
                        add_or_update_operation(pool, &mut op_info)?;
                        let ticket_id = create_support_ticket(
                            config,
                            "Bynar: Hardware Failure",
                            &format!("{}", e),
                        )?;
                        let op_id = match op_info.operation_id {
                            None => {
                                error!("Operation not recorded for {}", "",);
                                0
                            }
                            Some(i) => i,
                        };
                        debug!("Recording ticket id {} in database", ticket_id);
                        let mut operation_detail =
                            OperationDetail::new(op_id, OperationType::WaitingForReplacement);
                        operation_detail.set_tracking_id(ticket_id);
                        add_or_update_operation_detail(pool, &mut operation_detail)?;
                    }
                }
                _ => {
                    //Ignore other error types?
                    error!("evaluate error: {:?}", e);
                    return Err(e);
                }
            };
        }
    }
    Ok(())
}

fn check_for_failed_hardware(
    config: &ConfigSettings,
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    host_mapping: &HostDetailsMapping,
    simulate: bool,
) -> BynarResult<()> {
    info!("Checking hardware");
    let mut description = String::new();
    description.push_str(&format!(
        "\nHostname: {}\nServer type: {}\nServer Serial: {}\nMachine Architecture: {}\nKernel: {}",
        host_info.hostname,
        host_info.server_type,
        host_info.serial_number,
        host_info.machine_architecture,
        host_info.kernel,
    ));
    let results = test_hardware::check_hardware(&config)?;
    if !simulate {
        // Check if evaluate found any errors and log anything other then hardware errors
        if let Err(e) = evaluate(results.disk_drives, config, pool, host_mapping) {
            error!("Disk drive evaluation error: {:?}", e);
        }
        if let Err(e) = evaluate(results.manager, config, pool, host_mapping) {
            error!("Hardware manager evaluation error: {:?}", e);
        }
        if let Err(e) = evaluate(results.power, config, pool, host_mapping) {
            error!("Power supply evaluation error: {:?}", e);
        }
        if let Err(e) = evaluate(results.storage_enclosures, config, pool, host_mapping) {
            error!("Storage enclosures evaluation error: {:?}", e);
        }
        if let Err(e) = evaluate(results.thermals, config, pool, host_mapping) {
            error!("Thermal evaluation error: {:?}", e);
        }
    }

    Ok(())
}

// Actually, this function now checks the outstanding tickets, and if any of them are resolved, adds
// an add_disk request to the message_queue
fn add_repaired_disks(
    config: &ConfigSettings,
    //host_info: &Host,
    message_queue: &mut VecDeque<(Operation, Option<String>, Option<u32>)>,
    pool: &Pool<ConnectionManager>,
    storage_detail_id: u32,
    simulate: bool,
) -> BynarResult<()> {
    info!("Getting outstanding repair tickets");
    let tickets = in_progress::get_outstanding_repair_tickets(&pool, storage_detail_id)?;
    debug!("outstanding tickets: {:?}", tickets);
    info!("Checking for resolved repair tickets");
    for ticket in tickets {
        match ticket_resolved(config, &ticket.ticket_id.to_string()) {
            Ok(true) => {
                debug!("Creating add disk operation request");
                let op = helpers::make_op!(
                    Add,
                    format!("{}", Path::new(&ticket.device_path).display()),
                    simulate
                );
                let tid = Some(ticket.ticket_id.to_string());
                message_queue.push_back((op, tid, None));
            }
            Ok(false) => {}
            Err(e) => {
                error!("Error getting resolved ticket status for {}.  {:?}", &ticket.ticket_id, e);
            }
        };
    }
    Ok(())
}

// send a requst and update the message map
fn send_and_update(
    s: &Socket,
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    client_id: Vec<u8>,
    (mess, desc, op_id): (Operation, Option<String>, Option<u32>),
    path: &PathBuf,
) -> BynarResult<()> {
    trace!("Send request {:?}", mess);
    helpers::request(s, mess.clone(), client_id)?;
    //add or update to message_map if path != emptyyyy
    if mess.clone().get_disk() != "" {
        trace!("add operation to map");
        //check optype, make op
        let disk_op = DiskOp::new(mess, desc, op_id);
        add_or_update_map_op(message_map, &path, Some(disk_op))?;
    }
    Ok(())
}

// handle the return value from an add_disk request
fn handle_add_disk_res(
    pool: &Pool<ConnectionManager>,
    outcome: &OpOutcomeResult,
    ticket_id: String,
) {
    match outcome.get_outcome() {
        OpOutcome::Success => debug!("Disk added successfully. Updating database record"),
        // Disk was either boot or something that shouldn't be added via backend
        OpOutcome::Skipped => debug!("Disk Skipped.  Updating database record"),
        // Disk is already in the cluster
        OpOutcome::SkipRepeat => {
            if !outcome.has_value() {
                debug!("Disk already added.  Skipping.  Updating database record")
            } else {
                debug!("Disk already undergoing an operation.  Skipping.  Do not update database record");
                return;
            }
        }
    }
    match in_progress::resolve_ticket_in_db(pool, &ticket_id) {
        Ok(_) => debug!("Database updated"),
        Err(e) => error!("Failed to resolve ticket {}.  {:?}", ticket_id, e),
    };
}

//update map with operation result
fn update_map_result(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    host_info: &Host,
    dev_path: &PathBuf,
    op_res: OpOutcomeResult,
) -> BynarResult<()> {
    if let Some(mut current_op) = get_map_op(message_map, &dev_path)? {
        current_op.ret_val = Some(op_res);
        //push op back into map
        add_or_update_map_op(message_map, &dev_path, Some(current_op))?;
        Ok(())
    } else {
        Err(BynarError::from(format!(
            "{} on host {} does not have a currently running operation!",
            dev_path.display(),
            host_info.hostname
        )))
    }
}

// check if all operations on a disk have finished (assuming SafeToRemove/Remove operations)
fn is_all_finished(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
) -> BynarResult<bool> {
    // check if all the other paths in disk are SafeToRemove (and not Success)
    // check if all ops in the disk have finished
    let disk = get_disk_map_op(message_map, &dev_path)?;
    let mut all_finished = true;
    disk.iter().for_each(|(_partition, operation)| {
        //check if value finished
        // if OpOutcome:: Success and OpSafeToRemove, then true
        //if safeToRemove Success and false => true
        // if OpOutcome:: Success + Op::Remove, is fine?
        if let Some(op) = operation {
            if let Some(ret) = &op.ret_val {
                //if Err, then its done
                // if its safeToRemove Success + false then all_finished is true
                if !(ret.get_result() == ResultType::ERR)
                    && !(ret.get_outcome() != OpOutcome::Success
                        && (ret.get_op_type() == Op::SafeToRemove
                            || ret.get_op_type() == Op::Remove))
                    && !(ret.get_outcome() == OpOutcome::Success && ret.get_op_type() == Op::Remove)
                    && !(ret.get_outcome() == OpOutcome::Success
                        && !ret.get_value()
                        && ret.get_op_type() == Op::SafeToRemove)
                {
                    all_finished = false;
                }
            } else {
                all_finished = false;
            }
        } else {
            all_finished = false;
        }
    });
    Ok(all_finished)
}

// Open a ticket
fn open_jira_ticket(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    config: &ConfigSettings,
    dev_path: &PathBuf,
) -> BynarResult<()> {
    // get the path of the disk
    let path = get_request_keys(&dev_path)?.0;
    // get the current op associated with the disk
    if let Some(current_op) = get_map_op(message_map, &path)? {
        let description = match current_op.description {
            Some(d) => d,
            None => {
                return Err(BynarError::from(format!(
                    "Disk {} on host {} is missing a description",
                    path.display(),
                    host_info.hostname
                )))
            }
        };
        let op_id = match current_op.operation_id {
            None => {
                error!("Operation not recorded for {}", path.display());
                0
            }
            Some(i) => i,
        };
        //open JIRA ticket+ notify slack
        debug!("Creating support ticket");
        // temporarily disable error out
        match create_support_ticket(config, "Bynar: Dead disk", &description) {
            Ok(ticket_id) => {
                debug!("Recording ticket id {} in database", ticket_id);
                // update operation details in DB
                let mut operation_detail =
                    OperationDetail::new(op_id, OperationType::WaitingForReplacement);
                operation_detail.set_tracking_id(ticket_id);
                add_or_update_operation_detail(pool, &mut operation_detail)?;
            }
            Err(e) => {
                let _ = notify_slack(
                    config,
                    &format!("Unable to create ticket {:?} with description:\n {}", e, description),
                );
            }
        }
        /*
        let ticket_id =
            create_support_ticket(config, "Bynar: Dead disk", &description)?;
        debug!("Recording ticket id {} in database", ticket_id);
        // update operation detials in DB
        let mut operation_detail =
            OperationDetail::new(op_id, OperationType::WaitingForReplacement);
        operation_detail.set_tracking_id(ticket_id);
        add_or_update_operation_detail(pool, &mut operation_detail)?;
        */
        return Ok(());
    }
    Err(BynarError::from(format!(
        "Disk {} on host {} is missing the current operation",
        path.display(),
        host_info.hostname
    )))
}

//handle return of Operation
fn handle_operation_result(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    op_res: OpOutcomeResult,
    config: &ConfigSettings,
) -> BynarResult<()> {
    if let (ResultType::ERR, true) = (op_res.get_result(), op_res.has_error_msg()) {
        let msg = op_res.get_error_msg();
        match op_res.get_op_type() {
            Op::Add => {
                error!("Add disk failed : {}", msg);
                return Err(BynarError::from(msg));
            }
            Op::Remove => {
                error!("Remove disk failed : {}", msg);
                // no need to error out, but update the map.  Error outcomes are also expected for Remove,
                // since remove might be run on the disk and the partition...or the input path is not in the config file
            }
            Op::SafeToRemove => {
                error!("SafeToRemove disk failed : {}", msg);
                // no need to error out, but update the map.  Error outcomes are expected for SafeToRemove.
                // Ex. you removed a disk first before the partition.
            }
            _ => {}
        }
    }

    let dev_path = PathBuf::from(op_res.get_disk());
    match op_res.get_op_type() {
        Op::Add => {
            if let Some(disk_op) = get_map_op(message_map, &dev_path.to_path_buf())? {
                if let Some(ticket_id) = disk_op.description {
                    handle_add_disk_res(pool, &op_res, ticket_id);
                    //update result in the map (in otherwords, just set it to None)
                    remove_map_op(message_map, &dev_path.to_path_buf())?;
                    return Ok(());
                }
            }
            error!("Unable to get current operation in the map for {}", dev_path.display());
            Err(BynarError::from(format!(
                "Unable to get current operation in the map for {}",
                dev_path.display()
            )))
        }
        Op::SafeToRemove => {
            // get the op from map, update it with outcome, handle errors as necessary (just store in map)
            update_map_result(message_map, host_info, &dev_path, op_res)?;
            // if so, notify slack
            if is_all_finished(message_map, &dev_path)? {
                debug!("safe to remove: false");
                let _ = notify_slack(
                    config,
                    &format!(
                        "Need to remove disk {} but it's not safe \
                         on host: {}. I need a human.  Filing a ticket",
                        dev_path.display(),
                        host_info.hostname,
                    ),
                );
                open_jira_ticket(message_map, host_info, pool, config, &dev_path)?;
            }
            Ok(())
        }
        Op::Remove => {
            //check if successful or not and send to slack
            match op_res.get_outcome() {
                OpOutcome::Success => {
                    debug!(
                        "Disk {} on host {} removal successful",
                        dev_path.display(),
                        host_info.hostname
                    );
                    let _ = notify_slack(
                        config,
                        &format!(
                            "Disk {} on host {} removal successful",
                            dev_path.display(),
                            host_info.hostname
                        ),
                    );
                }
                OpOutcome::Skipped => {
                    debug!(
                        "Disk {} on host {} skipped, disk is not removable",
                        dev_path.display(),
                        host_info.hostname
                    );
                    let _ = notify_slack(
                        config,
                        &format!(
                            "Disk {} on host {} skipped, disk is not removable",
                            dev_path.display(),
                            host_info.hostname
                        ),
                    );
                }
                OpOutcome::SkipRepeat => {
                    if op_res.has_value() {
                        debug!(
                            "Disk {} on host {} currently undergoing another operation, skipping",
                            dev_path.display(),
                            host_info.hostname
                        );
                        let _ = notify_slack(
                            config,
                            &format!(
                                "Disk {} on host {} currently undergoing another operation, skipping",
                                dev_path.display(),  host_info.hostname
                            ),
                        );
                    } else {
                        debug!(
                            "Disk {} on host {} already removed, skipping.",
                            dev_path.display(),
                            host_info.hostname
                        );
                        let _ = notify_slack(
                            config,
                            &format!(
                                "Disk {} on host {} already removed, skipping.",
                                dev_path.display(),
                                host_info.hostname
                            ),
                        );
                    }
                }
            }
            //update map
            update_map_result(message_map, host_info, &dev_path, op_res)?;
            //if all finished open ticket+ notify slack
            if is_all_finished(message_map, &dev_path)? {
                let _ = notify_slack(
                    &config,
                    &format!(
                        "Filing a ticket for Host: {}. Drive {} needs removal",
                        host_info.hostname,
                        dev_path.display(),
                    ),
                );
                open_jira_ticket(message_map, host_info, pool, config, &dev_path)?;
            }
            Ok(())
        }
        _ => {
            // these operations should never get called by Bynar
            Err(BynarError::from(format!(
                "{} could not have run this operation!",
                op_res.get_disk()
            )))
        }
    }
}

// check if the socket is readable/writable and send/recieve message if possible
fn send_and_recieve(
    s: &Socket,
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    message_queue: &mut VecDeque<(Operation, Option<String>, Option<u32>)>,
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    config: &ConfigSettings,
    client_id: Vec<u8>,
) -> BynarResult<()> {
    // Note, all client sent messages are Operation, while return values of type OpOutcomeResult
    let events = poll_events!(s, return Ok(()));
    //check sendable first
    if events.contains(zmq::PollEvents::POLLOUT) {
        //dequeue from message_queue if it isn't empty
        if let Some((mess, desc, op_id)) = message_queue.pop_front() {
            // if mess.op_type() == Op::Remove, check if Safe-To-Remove in map complete
            // if not, send to end of queue (push_back)
            let path = PathBuf::from(mess.get_disk());
            //check if there was a previous request, and whether it was completed
            if let Some(disk_op) = get_map_op(&message_map, &path)? {
                // check if Safe-to-remove returned yet
                if let Some(val) = disk_op.ret_val {
                    // check if mess is a Remove op
                    if mess.get_Op_type() == Op::Remove {
                        // check success outcome
                        if val.get_outcome() == OpOutcome::Success && val.get_value() {
                            //then ok to run Op::Remove
                            send_and_update(s, message_map, client_id, (mess, desc, op_id), &path)?;
                            trace!("Updated map {:?}", message_map);
                        }
                    // safe-to-remove returned false or error'd so we should not remove but let manual handling
                    // delete the remove request in this case (in otherwords, do nothing)
                    } else {
                        // not remove request, since previous request is complete, run next request
                        // this technically shouldn't happen though, so print an error!
                        error!(
                            "Previous request {:?} has finished, but hasn't been reset",
                            disk_op.op_type
                        );
                        send_and_update(s, message_map, client_id, (mess, desc, op_id), &path)?;
                        trace!("Updated map {:?}", message_map);
                    }
                } else {
                    // we haven't gotten response from previous request yet, push request to back of queue
                    trace!("Have not gotten response yet, push back request {:?}", mess);
                    message_queue.push_back((mess, desc, op_id));
                }
            } else {
                // safe to run the op.  In the case of Remove op, it shouldn't be possible to NOT
                // have a safe-to-remove run before (it's always safe-to-remove then remove)
                // however since the remove operation will run safe-to-remove anyways, it's fine to just run
                send_and_update(s, message_map, client_id, (mess, desc, op_id), &path)?;
                trace!("Updated map {:?}", message_map);
            }
        }
    }
    // can get response
    if events.contains(zmq::PollEvents::POLLIN) {
        // get the message, it should be either a OpOutcomeResult, or OpJiraTicketsResult
        // NOTE: disks is not an option since list_disks is not a request that the main bynar program makes
        let mut message = helpers::get_messages(s)?;
        // skip empty initial message, and keep looping until no more messages from disk-manager
        while !message.is_empty() {
            // get message
            match helpers::get_first_op_result(&mut message) {
                Some(outcome) => {
                    //message.drain(0..outcome.write_to_bytes()?.len());
                    trace!("Sent map {:?}", message_map);
                    handle_operation_result(message_map, host_info, pool, outcome, config)?;
                }
                None => {
                    //Actually, this is a problem since Bynar only sends Add/SafeToRemove/Remove requests
                    error!("Message is not an OpOutcomeResult");
                    return Err(BynarError::from(
                        "Message received is not an OpOutcomeResult".to_string(),
                    ));
                }
            }
        }
    }
    Ok(())
}

// 1. Gather a list of all the disks
// 2. Check every disk
// 3. Decide if a disk needs to be replaced
// 4. File a ticket
// 5. Record the replacement in the in_progress sqlite database

fn main() {
    let matches = App::new("Dead Disk Detector")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Detect dead hard drives, create a support ticket and watch for resolution")
        .arg(
            Arg::with_name("configdir")
                .default_value("/etc/bynar")
                .help("The directory where all config files can be found")
                .long("configdir")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("simulate")
                .help("Log messages but take no action")
                .long("simulate")
                .required(false),
        )
        .arg(Arg::with_name("v").short("v").multiple(true).help("Sets the level of verbosity"))
        .arg(Arg::with_name("daemon").help("Run Bynar as a daemon").long("daemon").required(false))
        .arg(
            Arg::with_name("time")
                .help("Time in seconds between Bynar runs")
                .long("time")
                .default_value("60"),
        )
        .get_matches();

    let daemon = matches.is_present("daemon");
    let level = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Info, //default
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![];
    if let Some(term_logger) = TermLogger::new(level, Config::default()) {
        //systemd doesn't use a terminal
        loggers.push(term_logger);
    }
    loggers.push(WriteLogger::new(
        level,
        Config::default(),
        OpenOptions::new()
            .append(true)
            .create(true)
            .open("/var/log/bynar.log")
            .expect("/var/log/bynar.log creation failed"),
    ));
    let config_dir = Path::new(matches.value_of("configdir").unwrap());
    if !config_dir.exists() {
        warn!("Config directory {} doesn't exist. Creating", config_dir.display());
        if let Err(e) = create_dir(config_dir) {
            error!("Unable to create directory {}: {}", config_dir.display(), e.to_string());
            return;
        }
    }
    //TODO: create constant for bynar.json
    let config = helpers::load_config(config_dir, "bynar.json");
    if let Err(e) = config {
        error!(
            "Failed to load config file {}. error: {}",
            config_dir.join("bynar.json").display(),
            e
        );
        return;
    }
    let mut config: ConfigSettings = config.expect("Failed to load config");
    let _ = CombinedLogger::init(loggers);
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
            if out.contains("bynar") {
                //skip
                error!("There is already a running instance of bynar! Abort!");
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

    info!("------------------------------------------------\n\t\tStarting up");

    let simulate = matches.is_present("simulate");
    let time = matches.value_of("time").unwrap().parse::<u64>().unwrap();
    let h_info = Host::new();
    if h_info.is_err() {
        error!("Failed to gather host information");
        //gracefully exit
        return;
    }
    let host_info = h_info.expect("Failed to gather host information");
    debug!("Gathered host info: {:?}", host_info);

    let db_pool = match create_db_connection_pool(&config.database) {
        Err(e) => {
            error!("Failed to create database pool {}", e);
            return;
        }
        Ok(p) => p,
    };

    // Successfully opened a a database pool. Update information about host
    let host_details_mapping: HostDetailsMapping = match update_storage_info(&host_info, &db_pool) {
        Err(e) => {
            error!("Failed to update information in tracking database {}", e);
            // TODO [SD]: return if cannot update.
            return;
        }
        Ok(d) => {
            info!("Host information added to database");
            d
        }
    };
    let public_key = get_public_key(&config, &host_info).unwrap();
    let s =
        match helpers::connect(&config.manager_host, &config.manager_port.to_string(), &public_key)
        {
            Ok(s) => s,
            Err(e) => {
                error!("Error connecting to socket: {:?}", e);
                return;
            }
        };
    let client_id: Vec<u8> = s.get_identity().unwrap();
    debug!("Client ID {:?}, len {}", client_id, client_id.len());
    let dur = Duration::from_secs(time);
    let mut message_queue: VecDeque<(Operation, Option<String>, Option<u32>)> = VecDeque::new();
    let mut message_map = create_msg_map(&db_pool, &host_details_mapping).unwrap();
    'outer: loop {
        let now = Instant::now();
        match check_for_failed_disks(
            &mut message_map,
            &mut message_queue,
            &host_info,
            &db_pool,
            &host_details_mapping,
            simulate,
        ) {
            Err(e) => {
                error!("Check for failed disks failed with error: {}", e);
                break 'outer;
            }
            _ => {
                info!("Check for failed disks completed");
            }
        };
        match check_for_failed_hardware(
            &config,
            &host_info,
            &db_pool,
            &host_details_mapping,
            simulate,
        ) {
            Err(e) => {
                error!("Check for failed hardware failed with error: {}", e);
                break 'outer;
            }
            _ => {
                info!("Check for failed hardware completed");
            }
        };
        match add_repaired_disks(
            &config,
            &mut message_queue,
            &db_pool,
            host_details_mapping.storage_detail_id,
            simulate,
        ) {
            Err(e) => {
                error!("Add repaired disks failed with error: {}", e);
                break 'outer;
            }
            _ => {
                info!("Add repaired disks completed");
            }
        };
        debug!("Current Request Map {:?}", message_map);
        debug!("Current Message Queue {:?}", message_queue);
        while now.elapsed() < dur {
            if daemon {
                for signal in signals.pending() {
                    match signal as c_int {
                        signal_hook::SIGHUP => {
                            //Reload the config file
                            debug!("Reload Config File");
                            notify_slack(
                                &config,
                                &format!("Reload config file on {}", host_info.hostname),
                            )
                            .expect("Unable to connect to slack");
                            let config_file = helpers::load_config(config_dir, "bynar.json");
                            if let Err(e) = config_file {
                                error!(
                                    "Failed to load config file {}. error: {}",
                                    config_dir.join("bynar.json").display(),
                                    e
                                );
                                notify_slack(
                                    &config,
                                    &format!(
                                        "Failed to load config file {}. error: {}",
                                        config_dir.join("bynar.json").display(),
                                        e
                                    ),
                                )
                                .expect("Unable to connect to slack");
                                continue;
                            }
                            config = config_file.expect("Failed to load config");
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
            } else {
                break 'outer;
            }
            match send_and_recieve(
                &s,
                &mut message_map,
                &mut message_queue,
                &host_info,
                &db_pool,
                &config,
                client_id.clone(),
            ) {
                Err(e) => {
                    error!("Send or Receive messages failed with error: {}", e);
                    break 'outer;
                }
                _ => trace!("Send and Recieve successfully ran"),
            };
        }
        trace!("Request Map after looping {:?}", message_map);
    }
    info!("Bynar exited successfully");
    notify_slack(&config, &format!("Bynar on host  {} has stopped", host_info.hostname))
        .expect("Unable to connect to slack");
}

#[cfg(test)]
mod tests {
    use super::*;
    use block_utils::*;
    // list of devices to use in some test functions
    fn get_devices() -> Vec<PathBuf> {
        [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sdb"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdd"),
        ]
        .to_vec()
    }

    //list of partitions to use in some test functions
    fn get_partitions() -> Vec<PathBuf> {
        [
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd1"),
            PathBuf::from("/dev/sdd2"),
            PathBuf::from("/dev/sdd3"),
        ]
        .to_vec()
    }
    #[test]
    // This tests the filter(s) used to get a list of devices
    fn test_filter_block_devices() {
        let dev = block_utils::get_block_devices().unwrap();
        //print the list of devices
        println!("Devices before filtering: \n{:#?}", dev);

        let devices: Vec<PathBuf> = dev
            .into_iter()
            .filter(|b| {
                !(if let Some(p) = b.as_path().file_name() {
                    (p.to_string_lossy().starts_with("sr")
                        || p.to_string_lossy().starts_with("loop"))
                } else {
                    true
                })
            })
            .collect();

        println!("Devices after filtering: \n{:#?}", devices);
        //double check there are no paths that start with sr or loop
        assert_eq!(
            None,
            devices.into_iter().find(|b| {
                if let Some(p) = b.as_path().file_name() {
                    p.to_string_lossy().starts_with("loop") || p.to_string_lossy().starts_with("sr")
                } else {
                    true
                }
            })
        )
    }

    #[test]
    // Note: this isn't testing the actual function, since we can't do that,
    // this is testing the expected behavior of parts inside the function assuming certain call result
    fn test_create_msg_map_no_partitions() {
        let devices: Vec<PathBuf> = block_utils::get_block_devices()
            .unwrap()
            .into_iter()
            .filter(|b| {
                !(if let Some(p) = b.as_path().file_name() {
                    (p.to_string_lossy().starts_with("sr")
                        || p.to_string_lossy().starts_with("loop"))
                } else {
                    true
                })
            })
            .collect();
        println!("List of devices: \n{:#?}", devices);
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let partitions: Vec<PathBuf> = Vec::new();
        devices.iter().for_each(|device| {
            // make a new hashmap
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            disk_map.insert(device.to_path_buf(), None);
            // check if partition parent is device
            partitions
                .iter()
                .filter(|partition| {
                    partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
                })
                .for_each(|partition| {
                    disk_map.insert(partition.to_path_buf(), None);
                });
            map.insert(device.to_path_buf(), disk_map);
        });

        println!("Created Hashmap: \n{:#?}", map);

        // check that for every device in devices, there is a hashmap
        // in the map with just the device in it (there should be no partitions)
        devices.iter().for_each(|path| {
            let sub_map = map.get(&path.to_path_buf());
            assert!(sub_map.is_some());
            let sub_map = sub_map.unwrap(); //this should be safe
            assert_eq!(1, sub_map.len());
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });
    }

    #[test]
    // Note: this isn't testing the actual function, since we can't do that,
    // this is testing the expected behavior of parts inside the function assuming certain call result
    fn test_create_msg_map_with_partitions() {
        // since we want to test specifically the partitions we need an explicit device list
        let devices: Vec<PathBuf> = get_devices();
        println!("List of devices: \n{:#?}", devices);
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let partitions: Vec<PathBuf> = get_partitions();
        println!("List of partitions: \n{:#?}", partitions);
        devices.iter().for_each(|device| {
            // make a new hashmap
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            disk_map.insert(device.to_path_buf(), None);
            // check if partition parent is device
            partitions
                .iter()
                .filter(|partition| {
                    partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
                })
                .for_each(|partition| {
                    disk_map.insert(partition.to_path_buf(), None);
                });
            map.insert(device.to_path_buf(), disk_map);
        });

        println!("Created Hashmap: \n{:#?}", map);

        // check that for every device in devices, there is a hashmap
        // in the map with the device and all its partitions
        let sda_map =
            [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sda1"), PathBuf::from("/dev/sda2")]
                .to_vec();
        let sdb_map = [PathBuf::from("/dev/sdb")].to_vec();
        let sdc_map = [PathBuf::from("/dev/sdc"), PathBuf::from("/dev/sdc1")].to_vec();
        let sdd_map = [
            PathBuf::from("/dev/sdd"),
            PathBuf::from("/dev/sdd1"),
            PathBuf::from("/dev/sdd2"),
            PathBuf::from("/dev/sdd3"),
        ]
        .to_vec();

        //test sda
        let sub_map = map.get(&PathBuf::from("/dev/sda"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(3, sub_map.len());
        sda_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });

        //test sdb
        let sub_map = map.get(&PathBuf::from("/dev/sdb"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(1, sub_map.len());
        sdb_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });

        //test sdc
        let sub_map = map.get(&PathBuf::from("/dev/sdc"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(2, sub_map.len());
        sdc_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });

        //test sdd
        let sub_map = map.get(&PathBuf::from("/dev/sdd"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(4, sub_map.len());
        sdd_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });
    }

    #[test]
    // Note: this isn't testing the actual function, since we can't do that,
    // this is testing the expected behavior of parts inside the function assuming certain call result
    fn test_create_msg_map_with_db() {
        // since we want to test specifically the partitions we need an explicit device list
        let mut devices: Vec<PathBuf> = get_devices();
        println!("List of devices: \n{:#?}", devices);
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let db_devices: Vec<PathBuf> = [
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd1"),
            PathBuf::from("/dev/sdd2"),
            PathBuf::from("/dev/sdd3"),
        ]
        .to_vec();
        let partitions: Vec<PathBuf> = db_devices
            .clone()
            .into_iter()
            .filter(|p| p.to_string_lossy().chars().last().unwrap().is_digit(10))
            .collect();
        let mut disks: Vec<PathBuf> = db_devices
            .into_iter()
            .filter(|p| !p.to_string_lossy().chars().last().unwrap().is_digit(10))
            .collect();
        println!("List of DB partitions {:#?}", partitions);
        assert_eq!(
            [
                PathBuf::from("/dev/sda1"),
                PathBuf::from("/dev/sda2"),
                PathBuf::from("/dev/sdc1"),
                PathBuf::from("/dev/sdd1"),
                PathBuf::from("/dev/sdd2"),
                PathBuf::from("/dev/sdd3")
            ]
            .to_vec(),
            partitions
        );

        println!("List of DB disks {:?}", disks);
        assert_eq!([PathBuf::from("/dev/sdc")].to_vec(), disks);

        devices.append(&mut disks);
        devices.iter().for_each(|device| {
            // make a new hashmap
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            disk_map.insert(device.to_path_buf(), None);
            // check if partition parent is device
            partitions
                .iter()
                .filter(|partition| {
                    partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
                })
                .for_each(|partition| {
                    disk_map.insert(partition.to_path_buf(), None);
                });
            map.insert(device.to_path_buf(), disk_map);
        });

        println!("Created Hashmap: \n{:#?}", map);

        // check that for every device in devices, there is a hashmap
        // in the map with the device and all its partitions
        let sda_map =
            [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sda1"), PathBuf::from("/dev/sda2")]
                .to_vec();
        let sdb_map = [PathBuf::from("/dev/sdb")].to_vec();
        let sdc_map = [PathBuf::from("/dev/sdc"), PathBuf::from("/dev/sdc1")].to_vec();
        let sdd_map = [
            PathBuf::from("/dev/sdd"),
            PathBuf::from("/dev/sdd1"),
            PathBuf::from("/dev/sdd2"),
            PathBuf::from("/dev/sdd3"),
        ]
        .to_vec();

        //test sda
        let sub_map = map.get(&PathBuf::from("/dev/sda"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(3, sub_map.len());
        sda_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });

        //test sdb
        let sub_map = map.get(&PathBuf::from("/dev/sdb"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(1, sub_map.len());
        sdb_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });

        //test sdc
        let sub_map = map.get(&PathBuf::from("/dev/sdc"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(2, sub_map.len());
        sdc_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });

        //test sdd
        let sub_map = map.get(&PathBuf::from("/dev/sdd"));
        assert!(sub_map.is_some());
        let sub_map = sub_map.unwrap(); //this should be safe
        assert_eq!(4, sub_map.len());
        sdd_map.iter().for_each(|path| {
            assert!(sub_map.get(&path.to_path_buf()).is_some());
        });
    }

    // create empty map with just /dev/sda for testing
    fn empty_sda_map() -> HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        disk_map.insert(PathBuf::from("/dev/sda"), None);
        map.insert(PathBuf::from("/dev/sda"), disk_map);
        map
    }

    #[test]
    // test if, given a partition path that is not in the map (but the parent is)
    // add the partition to the map with the given operation
    fn test_add_or_update_map_op_partition_add() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = empty_sda_map();

        println!("Initial Map: \n{:#?}", map);
        let insert_path = PathBuf::from("/dev/sda1");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sda");
        assert!(map.get(&parent).unwrap().get(&insert_path).is_none());
        let disk = map.get_mut(&parent).unwrap(); // we know map should have this
        if let Some(_) = disk.clone().get(&insert_path) {
            // partition in map
            panic!("/dev/sda1 should not be in the map");
        }
        disk.insert(insert_path.to_path_buf(), Some(disk_op));

        println!("New Map: \n{:#?}", map);
        assert!(map.get(&parent).unwrap().get(&insert_path).is_some());
    }

    #[test]
    // test if, given a partition path that is in the map, update the map
    // with the given operation
    fn test_add_or_update_map_op_partition_update() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        disk_map.insert(PathBuf::from("/dev/sda"), None);
        let mut op = helpers::make_op!(Remove);
        let disk_op = DiskOp::new(op, Some("test update".to_string()), Some(0));
        disk_map.insert(PathBuf::from("/dev/sda1"), Some(disk_op));
        map.insert(PathBuf::from("/dev/sda"), disk_map);

        println!("Initial Map: \n{:#?}", map);
        let insert_path = PathBuf::from("/dev/sda1");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sda");
        assert!(
            map.get(&parent).unwrap().get(&insert_path).unwrap().as_ref().unwrap().op_type
                == Op::Remove
        );
        let disk = map.get_mut(&parent).unwrap(); // we know map should have this
        if let Some(_) = disk.clone().get(&insert_path) {
            // partition in map
            disk.insert(insert_path.to_path_buf(), Some(disk_op));
        } else {
            panic!("/dev/sda1 should be in the map");
        }
        println!("New Map: \n{:#?}", map);
        assert!(
            map.get(&parent).unwrap().get(&insert_path).unwrap().as_ref().unwrap().op_type
                == Op::Add
        );
    }

    #[test]
    // test if, given a partition path that is not in the map and whose parent is not
    // in the map, insert the partition + parent disk into the map
    fn test_add_or_update_map_op_partition_insert() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = empty_sda_map();

        println!("Initial Map: \n{:#?}", map);
        let insert_path = PathBuf::from("/dev/sdb1");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sdb");
        assert!(map.get(&parent).is_none());
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new(); // we know map doesn't have this
        disk_map.insert(parent.to_path_buf(), None);

        let partitions = [PathBuf::from("/dev/sdb1"), PathBuf::from("/dev/sdb2")].to_vec();
        partitions
            .iter()
            .filter(|partition| partition.to_string_lossy().contains("/dev/sdb"))
            .for_each(|partition| {
                disk_map.insert(partition.to_path_buf(), None);
            });
        disk_map.insert(insert_path.to_path_buf(), Some(disk_op));
        map.insert(parent.to_path_buf(), disk_map);
        println!("New Map: \n{:#?}", map);
        assert!(map.get(&parent).is_some());
        assert!(map.get(&parent).unwrap().get(&insert_path).is_some());
        assert!(map.get(&parent).unwrap().get(&PathBuf::from("/dev/sdb2")).is_some());
    }

    #[test]
    // test if, given a disk path that is not in the disk map (but is in the req map)
    // this should error out
    fn test_add_or_update_map_op_parent_error() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        map.insert(PathBuf::from("/dev/sda"), disk_map);

        println!("Initial Map: \n{:#?}", map);
        let insert_path = PathBuf::from("/dev/sda");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sda");
        assert!(map.get(&parent).is_some());
        assert!(map.get(&parent).unwrap().get(&insert_path).is_none());
        if parent == insert_path {
            //success, error behavior is in here, in the actual function
            println!("Function would error here");
        } else {
            panic!("These should be equivalent...");
        }
    }

    #[test]
    // test if, given a disk path that is in the disk map
    // update the disk map with the given operation
    fn test_add_or_update_map_op_parent_update() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        let mut op = helpers::make_op!(Remove);
        let disk_op = DiskOp::new(op, Some("test update".to_string()), Some(0));
        disk_map.insert(PathBuf::from("/dev/sda"), Some(disk_op));
        map.insert(PathBuf::from("/dev/sda"), disk_map);

        println!("Initial Map: \n{:#?}", map);
        let insert_path = PathBuf::from("/dev/sda");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sda");
        assert!(
            map.get(&parent).unwrap().get(&insert_path).unwrap().as_ref().unwrap().op_type
                == Op::Remove
        );
        let disk = map.get_mut(&parent).unwrap(); // we know map should have this
        if let Some(_) = disk.clone().get(&insert_path) {
            // partition in map
            disk.insert(insert_path.to_path_buf(), Some(disk_op));
        } else {
            panic!("/dev/sda should be in the map");
        }
        println!("New Map: \n{:#?}", map);
        assert!(
            map.get(&parent).unwrap().get(&insert_path).unwrap().as_ref().unwrap().op_type
                == Op::Add
        );
    }

    #[test]
    // test if, given a disk path that is not in the disk map nor the req map
    // create a new disk map with the disk path and insert into the req map
    fn test_add_or_update_map_op_parent_insert() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = empty_sda_map();

        println!("Initial Map: \n{:#?}", map);
        let insert_path = PathBuf::from("/dev/sdb");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sdb");
        assert!(map.get(&parent).is_none());
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new(); // we know map doesn't have this
        disk_map.insert(parent.to_path_buf(), None);

        let partitions = [PathBuf::from("/dev/sdb1"), PathBuf::from("/dev/sdb2")].to_vec();
        partitions
            .iter()
            .filter(|partition| partition.to_string_lossy().contains("/dev/sdb"))
            .for_each(|partition| {
                disk_map.insert(partition.to_path_buf(), None);
            });
        disk_map.insert(insert_path.to_path_buf(), Some(disk_op));
        map.insert(parent.to_path_buf(), disk_map);
        println!("New Map: \n{:#?}", map);
        assert!(map.get(&parent).is_some());
        assert!(map.get(&parent).unwrap().get(&insert_path).is_some());
        assert!(map.get(&parent).unwrap().get(&PathBuf::from("/dev/sdb")).is_some());
    }

    #[test]
    // test if getting the parent from a deleted partition path works
    fn test_get_request_keys_deleted() {
        let path = PathBuf::from("/dev/sdc2");
        let hd_path = PathBuf::from("/dev/hda12");
        let nvme_path = PathBuf::from("/dev/nvme0n1p3"); // test this one once nvme implemented
        let mut str_path = path.to_string_lossy().to_string();
        while str_path.chars().last().unwrap().is_digit(10) {
            str_path = str_path[0..str_path.len() - 1].to_string();
        }
        assert_eq!("/dev/sdc".to_string(), str_path);

        let mut str_path = hd_path.to_string_lossy().to_string();
        while str_path.chars().last().unwrap().is_digit(10) {
            str_path = str_path[0..str_path.len() - 1].to_string();
        }
        assert_eq!("/dev/hda".to_string(), str_path);
    }

    #[test]
    // test get_map_op function
    fn test_get_map_op() {
        //make map
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        let insert_path = PathBuf::from("/dev/sda1");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sda");
        assert!(map.get(&parent).is_none());
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new(); // we know map doesn't have this
        disk_map.insert(parent.to_path_buf(), None);
        disk_map.insert(insert_path.to_path_buf(), Some(disk_op));
        map.insert(parent.to_path_buf(), disk_map);
        println!("Map: \n{:#?}", map);

        assert!(get_map_op(&map, &PathBuf::from("/dev/sda")).unwrap().is_none());
        assert!(get_map_op(&map, &PathBuf::from("/dev/sda1")).unwrap().is_some());
    }

    #[test]
    // test remove_map_op function
    fn test_remove_map_op() {
        //make map
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        let insert_path = PathBuf::from("/dev/sda1");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sda");
        assert!(map.get(&parent).is_none());
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new(); // we know map doesn't have this
        disk_map.insert(parent.to_path_buf(), None);
        disk_map.insert(insert_path.to_path_buf(), Some(disk_op));
        map.insert(parent.to_path_buf(), disk_map);
        println!("Map: \n{:#?}", map);

        assert!(map.get(&parent).unwrap().get(&insert_path).unwrap().is_some());
        remove_map_op(&mut map, &insert_path);
        assert!(map.get(&parent).unwrap().get(&insert_path).unwrap().is_none());
        println!("After Removal: \n{:#?}", map);
    }

    #[test]
    // test get_disk_map_op
    fn test_get_disk_map_op() {
        //make map
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        let insert_path = PathBuf::from("/dev/sda1");
        let op = Operation::new();
        let disk_op = DiskOp::new(op, Some("test".to_string()), None);

        let parent = PathBuf::from("/dev/sda");
        assert!(map.get(&parent).is_none());
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new(); // we know map doesn't have this
        disk_map.insert(parent.to_path_buf(), None);
        disk_map.insert(insert_path.to_path_buf(), Some(disk_op));
        map.insert(parent.to_path_buf(), disk_map);
        println!("Map: \n{:#?}", map);

        let req_disk_map = get_disk_map_op(&mut map, &insert_path).unwrap();
        assert_eq!(2, req_disk_map.len());
        assert!(req_disk_map.get(&insert_path).is_some());
        assert!(req_disk_map.get(&insert_path).unwrap().is_some());
        assert!(req_disk_map.get(&parent).is_some());
        assert!(req_disk_map.get(&parent).unwrap().is_none());
    }

    // create empty map for testing with None values
    fn create_none_map() -> HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> {
        let devices: Vec<PathBuf> = get_devices();
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let partitions: Vec<PathBuf> = get_partitions();
        devices.iter().for_each(|device| {
            // make a new hashmap
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            disk_map.insert(device.to_path_buf(), None);
            // check if partition parent is device
            partitions
                .iter()
                .filter(|partition| {
                    partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
                })
                .for_each(|partition| {
                    disk_map.insert(partition.to_path_buf(), None);
                });
            map.insert(device.to_path_buf(), disk_map);
        });
        map
    }
    #[test]
    // check filter disks that are Waiting for Replacement with map having None
    // no in progress check since all paths should have None
    fn test_get_replacing_vec_none() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = create_none_map();

        println!("Initial Hashmap: \n{:#?}", map);
        let states: Vec<PathBuf> = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sdb"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd"),
        ]
        .to_vec();
        // Testing, assuming /dev/sda and /dev/sdc1 are in "WaitingForReplacement" state
        // no need to make in progress a variable since all map objects are None
        let replacing: Vec<_> = states
            .into_iter()
            .filter(|path| {
                if path == &PathBuf::from("/dev/sda") || path == &PathBuf::from("/dev/sdc1") {
                    // the two "Waiting for Replacement" states
                    //simulate get_map_op
                    let parent = if path == &PathBuf::from("/dev/sdc1") {
                        PathBuf::from("/dev/sdc")
                    } else {
                        path.to_path_buf()
                    };
                    let op = map.get(&parent).unwrap().get(path).unwrap();
                    match op {
                        Some(op) => panic!("Should be None"),
                        None => true,
                    }
                } else {
                    false
                }
            })
            .collect();

        println!("Replacing: {:#?}", replacing);
        assert_eq!(replacing, [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sdc1")].to_vec());
    }

    // create initial map for testing Add
    fn create_add_map() -> HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> {
        let devices: Vec<PathBuf> = get_devices();
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let partitions: Vec<PathBuf> = get_partitions();
        devices.iter().for_each(|device| {
            // make a new hashmap
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            let op = Operation::new();
            let disk_op = DiskOp::new(op, None, None);
            disk_map.insert(device.to_path_buf(), Some(disk_op));
            // check if partition parent is device
            partitions
                .iter()
                .filter(|partition| {
                    partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
                })
                .for_each(|partition| {
                    let op = Operation::new();
                    let disk_op = DiskOp::new(op, None, None);
                    disk_map.insert(partition.to_path_buf(), Some(disk_op));
                });
            map.insert(device.to_path_buf(), disk_map);
        });
        map
    }

    #[test]
    // check filter disks that are Waiting for Replacement with map having Add
    // in progress yes or no
    fn test_get_replacing_vec_add() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = create_add_map();

        println!("Initial Hashmap: \n{:#?}", map);
        let states: Vec<PathBuf> = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sdb"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd"),
        ]
        .to_vec();
        // Testing, assuming /dev/sda and /dev/sdc1 are in "WaitingForReplacement" state
        // in progress is now variable...
        let replacing: Vec<_> = states
            .into_iter()
            .filter(|path| {
                if path == &PathBuf::from("/dev/sda") || path == &PathBuf::from("/dev/sdc1") {
                    // the two "Waiting for Replacement" states
                    //simulate get_map_op
                    let parent = if path == &PathBuf::from("/dev/sdc1") {
                        PathBuf::from("/dev/sdc")
                    } else {
                        path.to_path_buf()
                    };
                    let in_progress = path == &PathBuf::from("/dev/sdc1"); //sdc1 in progress, sda is not
                    let op = map.get(&parent).unwrap().get(path).unwrap();
                    match op {
                        Some(op) => {
                            !(op.op_type == Op::SafeToRemove
                                || op.op_type == Op::Remove
                                || in_progress)
                        }
                        None => panic!("Should be Some"),
                    }
                } else {
                    false
                }
            })
            .collect();

        println!("Replacing: {:#?}", replacing);
        assert_eq!(replacing, [PathBuf::from("/dev/sda")]);
    }
    #[test]
    // check filter disks that are Waiting for Replacement with map having SafeToRemove || Remove
    fn test_get_replacing_vec_exists() {
        let devices: Vec<PathBuf> = get_devices();
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let partitions: Vec<PathBuf> = get_partitions();
        devices.iter().for_each(|device| {
            // make a new hashmap
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            let mut op = Operation::new();
            if device == &PathBuf::from("/dev/sda") {
                op.set_Op_type(Op::SafeToRemove);
            }
            let disk_op = DiskOp::new(op, None, None);
            disk_map.insert(device.to_path_buf(), Some(disk_op));
            // check if partition parent is device
            partitions
                .iter()
                .filter(|partition| {
                    partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
                })
                .for_each(|partition| {
                    let mut op = Operation::new();
                    if partition == &PathBuf::from("/dev/sdc1") {
                        op.set_Op_type(Op::Remove);
                    }
                    let disk_op = DiskOp::new(op, None, None);
                    disk_map.insert(partition.to_path_buf(), Some(disk_op));
                });
            map.insert(device.to_path_buf(), disk_map);
        });

        println!("Initial Hashmap: \n{:#?}", map);
        let states: Vec<PathBuf> = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sdb"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd"),
        ]
        .to_vec();
        // Testing, assuming /dev/sda and /dev/sdc1 are in "WaitingForReplacement" state
        // in progress is now variable...
        let replacing: Vec<PathBuf> = states
            .into_iter()
            .filter(|path| {
                if path == &PathBuf::from("/dev/sda") || path == &PathBuf::from("/dev/sdc1") {
                    // the two "Waiting for Replacement" states
                    //simulate get_map_op
                    let parent = if path == &PathBuf::from("/dev/sdc1") {
                        PathBuf::from("/dev/sdc")
                    } else {
                        path.to_path_buf()
                    };
                    let in_progress = path == &PathBuf::from("/dev/sdc1"); //sdc1 in progress, sda is not
                    let op = map.get(&parent).unwrap().get(path).unwrap();
                    match op {
                        Some(op) => {
                            !(op.op_type == Op::SafeToRemove
                                || op.op_type == Op::Remove
                                || in_progress)
                        }
                        None => panic!("Should be Some"),
                    }
                } else {
                    false
                }
            })
            .collect();

        println!("Replacing: {:#?}", replacing);
        let empty: Vec<PathBuf> = [].to_vec();
        assert_eq!(replacing, empty);
    }

    #[test]
    // test adding related partitions/disks to list
    // map all nones
    fn test_add_related_paths_none() {
        // init the map
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = create_none_map();

        println!("Initial Hashmap: \n{:#?}", map);

        let states: Vec<PathBuf> = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sdb"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd"),
        ]
        .to_vec();
        // create list of "replacing paths"
        let mut replacing = [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sdc1")].to_vec();
        println!("Initial Replacing: {:#?}", replacing);
        // test adding paths
        let mut add_replacing = Vec::new();
        replacing.iter().for_each(|path| {
            let parent = if path == &PathBuf::from("/dev/sdc1") {
                PathBuf::from("/dev/sdc")
            } else {
                path.to_path_buf()
            };
            let disks = map.get(&parent).unwrap();
            let mut add: Vec<_> = states
                .iter()
                .filter(|state| {
                    if disks.contains_key(&state.to_path_buf()) {
                        match map.get(&parent).unwrap().get(&state.to_path_buf()).unwrap() {
                            Some(op) => panic!("all items in map should be NONE"),
                            None => true,
                        }
                    } else {
                        false
                    }
                })
                .collect();
            add_replacing.append(&mut add);
        });

        println!("Added values: {:#?}", add_replacing);
        let paths = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
        ];
        paths.iter().for_each(|path| {
            assert!(add_replacing.contains(&path));
        });
    }

    #[test]
    // test adding related partitions/disks to list
    // map all Add
    fn test_add_related_paths_add() {
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = create_add_map();

        println!("Initial Hashmap: \n{:#?}", map);
        let states: Vec<PathBuf> = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sdb"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd"),
        ]
        .to_vec();
        // create list of "replacing paths"
        let mut replacing = [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sdc1")].to_vec();
        println!("Initial Replacing: {:#?}", replacing);
        // test adding paths
        let mut add_replacing = Vec::new();
        replacing.iter().for_each(|path| {
            let parent = if path == &PathBuf::from("/dev/sdc1") {
                PathBuf::from("/dev/sdc")
            } else {
                path.to_path_buf()
            };
            let disks = map.get(&parent).unwrap();
            let in_progress = path == &PathBuf::from("/dev/sdc");
            let mut add: Vec<_> = states
                .iter()
                .filter(|state| {
                    if disks.contains_key(&state.to_path_buf()) {
                        match map.get(&parent).unwrap().get(&state.to_path_buf()).unwrap() {
                            Some(op) => {
                                !(op.op_type == Op::SafeToRemove
                                    || op.op_type == Op::Remove
                                    || in_progress)
                            }
                            None => panic!("all items in map should be SOME"),
                        }
                    } else {
                        false
                    }
                })
                .collect();
            add_replacing.append(&mut add);
        });

        println!("Added values: {:#?}", add_replacing);
        let paths = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sdc1"),
        ];
        paths.iter().for_each(|path| {
            assert!(add_replacing.contains(&path));
        });
    }

    #[test]
    // test adding related partitions/disks to list
    // map all SafeToRemove or Removes
    fn test_add_related_paths_empty() {
        let devices = get_devices();
        let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
        let partitions: Vec<PathBuf> = get_partitions();
        devices.iter().for_each(|device| {
            // make a new hashmap
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            let mut op = Operation::new();
            if !(device == &PathBuf::from("/dev/sda")) {
                op.set_Op_type(Op::SafeToRemove);
            }
            let disk_op = DiskOp::new(op, None, None);
            disk_map.insert(device.to_path_buf(), Some(disk_op));
            // check if partition parent is device
            partitions
                .iter()
                .filter(|partition| {
                    partition.to_string_lossy().contains(&device.to_string_lossy().to_string())
                })
                .for_each(|partition| {
                    let mut op = Operation::new();
                    if !(partition == &PathBuf::from("/dev/sdc1")) {
                        op.set_Op_type(Op::Remove);
                    }
                    let disk_op = DiskOp::new(op, None, None);
                    disk_map.insert(partition.to_path_buf(), Some(disk_op));
                });
            map.insert(device.to_path_buf(), disk_map);
        });

        println!("Initial Hashmap: \n{:#?}", map);
        let states: Vec<PathBuf> = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sdb"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
            PathBuf::from("/dev/sdd"),
        ]
        .to_vec();
        // create list of "replacing paths"
        let mut replacing = [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sdc1")].to_vec();
        println!("Initial Replacing: {:#?}", replacing);
        // test adding paths
        let mut add_replacing = Vec::new();
        replacing.iter().for_each(|path| {
            let parent = if path == &PathBuf::from("/dev/sdc1") {
                PathBuf::from("/dev/sdc")
            } else {
                path.to_path_buf()
            };
            let disks = map.get(&parent).unwrap();
            let in_progress = path == &PathBuf::from("/dev/sdc");
            let mut add: Vec<_> = states
                .iter()
                .filter(|state| {
                    if disks.contains_key(&state.to_path_buf()) {
                        match map.get(&parent).unwrap().get(&state.to_path_buf()).unwrap() {
                            Some(op) => {
                                !(op.op_type == Op::SafeToRemove
                                    || op.op_type == Op::Remove
                                    || in_progress)
                            }
                            None => panic!("all items in map should be SOME"),
                        }
                    } else {
                        false
                    }
                })
                .collect();
            add_replacing.append(&mut add);
        });

        println!("Added values: {:#?}", add_replacing);
        let paths = [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sdc1")];
        paths.iter().for_each(|path| {
            assert!(add_replacing.contains(&path));
        });
    }

    #[test]
    // remove all duplicates from the replacing vector
    fn test_remove_duplicates() {
        let mut replacing = [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sdc1")].to_vec();
        let mut paths = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
        ]
        .to_vec();
        replacing.append(&mut paths);
        println!("Appended: {:#?}", replacing);
        replacing.sort_unstable_by(|a, b| a.partial_cmp(&b.to_path_buf()).unwrap());
        replacing.dedup_by(|a, b| a == b);
        let compare = [
            PathBuf::from("/dev/sda"),
            PathBuf::from("/dev/sda1"),
            PathBuf::from("/dev/sda2"),
            PathBuf::from("/dev/sdc"),
            PathBuf::from("/dev/sdc1"),
        ]
        .to_vec();
        println!("Sorted and unique: {:#?}", replacing);
        assert_eq!(compare, replacing);
    }

    #[test]
    // test all finished check where disk_map is all finished and mixed Remove/SafeToRemove
    fn test_all_finished_mixed() {
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();

        let disk_paths =
            [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sda1"), PathBuf::from("/dev/sda2")]
                .to_vec();
        disk_paths.iter().for_each(|path| {
            let mut safe_to_rem = OpOutcomeResult::new();
            let mut op = Operation::new();
            safe_to_rem.set_result(ResultType::OK);
            safe_to_rem.set_outcome(OpOutcome::Skipped);
            if path == &PathBuf::from("/dev/sda") {
                safe_to_rem.set_outcome(OpOutcome::Success);
                safe_to_rem.set_op_type(Op::Remove);
                op.set_Op_type(Op::Remove);
            } else {
                safe_to_rem.set_op_type(Op::SafeToRemove);
                op.set_Op_type(Op::SafeToRemove);
            }
            let mut disk_op = DiskOp::new(op, None, None);
            disk_op.ret_val = Some(safe_to_rem);
            disk_map.insert(path.to_path_buf(), Some(disk_op));
        });
        println!("Initial Disk Map: {:#?}", disk_map);

        let mut all_finished = true;
        disk_map.iter().for_each(|(k, v)| {
            //check if value finished
            if let Some(val) = v {
                if let Some(ret) = &val.ret_val {
                    if !(ret.get_result() == ResultType::ERR)
                        && !(ret.get_outcome() != OpOutcome::Success
                            && (ret.get_op_type() == Op::SafeToRemove
                                || ret.get_op_type() == Op::Remove))
                        && !(ret.get_outcome() == OpOutcome::Success
                            && ret.get_op_type() == Op::Remove)
                    {
                        all_finished = false;
                    }
                } else {
                    all_finished = false;
                }
            } else {
                all_finished = false;
            }
        });
        assert!(all_finished);
    }

    #[test]
    // test all finished check where disk_map is not finished
    fn test_all_finished_mixed_fail() {
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();

        let disk_paths =
            [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sda1"), PathBuf::from("/dev/sda2")]
                .to_vec();
        disk_paths.iter().for_each(|path| {
            let mut safe_to_rem = OpOutcomeResult::new();
            let mut op = Operation::new();
            safe_to_rem.set_result(ResultType::OK);
            if path == &PathBuf::from("/dev/sda2") {
                safe_to_rem.set_outcome(OpOutcome::Success);
            } else {
                safe_to_rem.set_outcome(OpOutcome::Skipped);
            }
            if path == &PathBuf::from("/dev/sda") {
                safe_to_rem.set_op_type(Op::Remove);
                op.set_Op_type(Op::Remove);
            } else {
                safe_to_rem.set_op_type(Op::SafeToRemove);
                op.set_Op_type(Op::SafeToRemove);
            }
            let mut disk_op = DiskOp::new(op, None, None);
            disk_op.ret_val = Some(safe_to_rem);
            disk_map.insert(path.to_path_buf(), Some(disk_op));
        });
        println!("Initial Disk Map: {:#?}", disk_map);

        let mut all_finished = true;
        disk_map.iter().for_each(|(k, v)| {
            //check if value finished
            if let Some(val) = v {
                if let Some(ret) = &val.ret_val {
                    if !(ret.get_result() == ResultType::ERR)
                        && !(ret.get_outcome() != OpOutcome::Success
                            && (ret.get_op_type() == Op::SafeToRemove
                                || ret.get_op_type() == Op::Remove))
                        && !(ret.get_outcome() == OpOutcome::Success
                            && ret.get_op_type() == Op::Remove)
                    {
                        all_finished = false;
                    }
                } else {
                    all_finished = false;
                }
            } else {
                all_finished = false;
            }
        });
        assert!(!all_finished);
    }
    #[test]
    // test all finished check where disk_map is finished and everything error'd
    fn test_all_finished_err() {
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
        let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();

        let disk_paths =
            [PathBuf::from("/dev/sda"), PathBuf::from("/dev/sda1"), PathBuf::from("/dev/sda2")]
                .to_vec();
        disk_paths.iter().for_each(|path| {
            let mut safe_to_rem = OpOutcomeResult::new();
            let mut op = Operation::new();
            safe_to_rem.set_result(ResultType::ERR);
            if path == &PathBuf::from("/dev/sda") {
                safe_to_rem.set_op_type(Op::Remove);
                op.set_Op_type(Op::Remove);
            } else {
                safe_to_rem.set_op_type(Op::SafeToRemove);
                op.set_Op_type(Op::SafeToRemove);
            }
            let mut disk_op = DiskOp::new(op, None, None);
            disk_op.ret_val = Some(safe_to_rem);
            disk_map.insert(path.to_path_buf(), Some(disk_op));
        });
        println!("Initial Disk Map: {:#?}", disk_map);

        let mut all_finished = true;
        disk_map.iter().for_each(|(k, v)| {
            //check if value finished
            if let Some(val) = v {
                if let Some(ret) = &val.ret_val {
                    if !(ret.get_result() == ResultType::ERR)
                        && !(ret.get_outcome() != OpOutcome::Success
                            && (ret.get_op_type() == Op::SafeToRemove
                                || ret.get_op_type() == Op::Remove))
                        && !(ret.get_outcome() == OpOutcome::Success
                            && ret.get_op_type() == Op::Remove)
                    {
                        all_finished = false;
                    }
                } else {
                    all_finished = false;
                }
            } else {
                all_finished = false;
            }
        });
        assert!(all_finished);
    }
}
