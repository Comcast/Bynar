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

// a specific operation and its outcome
#[derive(Debug, Clone)]
struct DiskOp {
    pub op_type: Op, // operation type
    // the description for a JIRA ticket if necessary (None if not Safe-to-remove/Remove-disk)
    // Or, if an add_disk request, description is the ticket_id
    pub description: Option<String>,
    pub operation_id: Option<u32>, // the operation id if one exists (for safe-to-remove, remove request handling)
    pub ret_val: Option<OpOutcomeResult>, //None if outcome not yet determined
}

impl DiskOp {
    pub fn new(op: Operation, description: Option<String>, operation_id: Option<u32>) -> DiskOp {
        DiskOp {
            op_type: op.get_Op_type(),
            description,
            operation_id,
            ret_val: None,
        }
    }
}

// create a message map to handle list of disk-manager requests
fn create_msg_map() -> BynarResult<HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>> {
    // List out currently mounted block_devices
    let devices: Vec<PathBuf> = block_utils::get_block_devices()?
        .into_iter()
        .filter(|b| {
            !(if let Some(p) = b.as_path().file_name() {
                p.to_string_lossy().starts_with("sr")
            } else {
                true
            })
        })
        .filter(|b| {
            !(if let Some(p) = b.as_path().file_name() {
                p.to_string_lossy().starts_with("loop")
            } else {
                true
            })
        })
        .collect();
    let mut map: HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>> = HashMap::new();
    let partitions = block_utils::get_block_partitions()?;
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
                partition
                    .to_string_lossy()
                    .contains(&device.to_string_lossy().to_string())
            })
            .for_each(|partition| {
                disk_map.insert(partition.to_path_buf(), None);
            });
        map.insert(device.to_path_buf(), disk_map);
    });
    Ok(map)
}

// add or update an operation to the message map.  If an operation is already ongoing, update op and return the old operation
fn add_or_update_map_op(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
    op: DiskOp,
) -> BynarResult<Option<DiskOp>> {
    if let Some(parent) = block_utils::get_parent_devpath_from_path(dev_path)? {
        //parent is in the map
        if let Some(disk) = message_map.get_mut(&parent) {
            if let Some(partition) = disk.clone().get(dev_path) {
                // partition in map
                disk.insert(dev_path.to_path_buf(), Some(op));
                return Ok(partition.clone());
            }
            disk.insert(dev_path.to_path_buf(), Some(op));
        } else {
            //add to map
            let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
            disk_map.insert(parent.to_path_buf(), Some(op));
            let partitions = block_utils::get_block_partitions()?;
            // check if partition parent is device
            for partition in &partitions {
                if let Some(disk) = block_utils::get_parent_devpath_from_path(&partition)? {
                    if disk == parent {
                        disk_map.insert(partition.to_path_buf(), None);
                    }
                }
            }
            message_map.insert(parent.to_path_buf(), disk_map);
        }
    } else {
        //not partition or partition destroyed
        if dev_path.exists() {
            //parent is in the map
            if let Some(disk) = message_map.get_mut(dev_path) {
                if let Some(partition) = disk.clone().get(dev_path) {
                    // partition in map
                    disk.insert(dev_path.to_path_buf(), Some(op));
                    return Ok(partition.clone());
                }
                disk.insert(dev_path.to_path_buf(), Some(op));
            } else {
                //add to map
                let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
                disk_map.insert(dev_path.to_path_buf(), Some(op));
                let partitions = block_utils::get_block_partitions()?;
                // check if partition parent is device
                for partition in &partitions {
                    if let Some(disk) = block_utils::get_parent_devpath_from_path(&partition)? {
                        if &disk == dev_path {
                            disk_map.insert(partition.to_path_buf(), None);
                        }
                    }
                }
                message_map.insert(dev_path.to_path_buf(), disk_map);
            }
        } else {
            // partition was destroyed...probably
            // make parent path
            let path = dev_path.to_string_lossy();
            let path = &path[0..path.len() - 1];
            let path = PathBuf::from(path.to_string());
            if path.exists() {
                //then make new entry to insert...
                if let Some(disk) = message_map.get_mut(&path) {
                    // we know the partition isn't in the map already...
                    disk.insert(dev_path.to_path_buf(), Some(op));
                } else {
                    //add to map
                    let mut disk_map: HashMap<PathBuf, Option<DiskOp>> = HashMap::new();
                    disk_map.insert(path.to_path_buf(), Some(op));
                    let partitions = block_utils::get_block_partitions()?;
                    // check if partition parent is device
                    for partition in &partitions {
                        if let Some(disk) = block_utils::get_parent_devpath_from_path(&partition)? {
                            if disk == path {
                                disk_map.insert(partition.to_path_buf(), None);
                            }
                        }
                    }
                    message_map.insert(path.to_path_buf(), disk_map);
                }
            } else {
                // path just doesn't exist, so error...
                error!(
                    "Path {} does not exist, nor does its parent.",
                    dev_path.display()
                );
                return Err(BynarError::from(format!(
                    "Path {} does not exist, nor does its parent.",
                    dev_path.display()
                )));
            }
        }
    }
    Ok(None)
}

// get the operation for a device (disk/partition) if one exists
fn get_map_op(
    message_map: &HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
) -> BynarResult<Option<DiskOp>> {
    if let Some(parent) = block_utils::get_parent_devpath_from_path(dev_path)? {
        //parent is in the map
        if let Some(disk) = message_map.get(&parent) {
            if let Some(partition) = disk.get(dev_path) {
                // partition in map
                return Ok(partition.clone());
            }
        }
    } else {
        if dev_path.exists() {
            //not partition
            //parent is in the map
            if let Some(disk) = message_map.get(dev_path) {
                if let Some(partition) = disk.get(dev_path) {
                    // partition in map
                    return Ok(partition.clone());
                }
            }
        } else {
            // partition was destroyed...probably
            // make parent path
            let path = dev_path.to_string_lossy();
            let path = &path[0..path.len() - 1];
            let path = PathBuf::from(path.to_string());
            if let Some(disk) = message_map.get(&path) {
                if let Some(partition) = disk.get(dev_path) {
                    // partition in map
                    return Ok(partition.clone());
                }
            }
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
    if let Some(parent) = block_utils::get_parent_devpath_from_path(dev_path)? {
        //parent is in the map
        if let Some(disk) = message_map.get_mut(&parent) {
            if let Some(partition) = disk.clone().get(dev_path) {
                //set point as None
                disk.insert(dev_path.to_path_buf(), None);
                // partition in map
                return Ok(partition.clone());
            }
        }
    } else {
        if dev_path.exists() {
            //not partition
            //parent is in the map
            if let Some(disk) = message_map.get_mut(dev_path) {
                if let Some(partition) = disk.clone().get(dev_path) {
                    // partition in map
                    disk.insert(dev_path.to_path_buf(), None);
                    return Ok(partition.clone());
                }
            }
        } else {
            // partition was destroyed...probably
            // make parent path
            let path = dev_path.to_string_lossy();
            let path = &path[0..path.len() - 1];
            let path = PathBuf::from(path.to_string());
            if let Some(disk) = message_map.get_mut(&path) {
                if let Some(partition) = disk.clone().get(dev_path) {
                    // partition in map
                    disk.insert(dev_path.to_path_buf(), None);
                    return Ok(partition.clone());
                }
            }
        }
    }
    Err(BynarError::from(format!(
        "Path {} is not in the message map",
        dev_path.display()
    )))
}

// get the hashmap associated with a diskpath from the op map
fn get_disk_map_op(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    dev_path: &PathBuf,
) -> BynarResult<HashMap<PathBuf, Option<DiskOp>>> {
    if let Some(parent) = block_utils::get_parent_devpath_from_path(dev_path)? {
        //parent is in the map
        if let Some(disk) = message_map.get(&parent) {
            return Ok(disk.clone());
        }
    } else {
        //parent is in the map
        if dev_path.exists() {
            if let Some(disk) = message_map.get(dev_path) {
                return Ok(disk.clone());
            }
        } else {
            // partition was destroyed...probably
            // make parent path
            let path = dev_path.to_string_lossy();
            let path = &path[0..path.len() - 1];
            let path = PathBuf::from(path.to_string());
            if let Some(disk) = message_map.get(&path) {
                return Ok(disk.clone());
            }
        }
    }
    Err(BynarError::from(format!(
        "Path {} is not a disk in the map",
        dev_path.display()
    )))
}

fn notify_slack(config: &ConfigSettings, msg: &str) -> BynarResult<()> {
    let c = config.clone();
    let slack = Slack::new(
        c.slack_webhook
            .expect("slack webhook option is None")
            .as_ref(),
    )?;
    let slack_channel = c.slack_channel.unwrap_or_else(|| "".to_string());
    let bot_name = c.slack_botname.unwrap_or_else(|| "".to_string());
    let p = PayloadBuilder::new()
        .text(msg)
        .channel(slack_channel)
        .username(bot_name)
        .build()?;

    let res = slack.send(&p);
    match res {
        Ok(_) => debug!("Slack notified"),
        Err(e) => error!("Slack error: {:?}", e),
    };
    Ok(())
}

fn get_public_key(config: &ConfigSettings, host_info: &Host) -> BynarResult<Vec<u8>> {
    // If vault_endpoint and token are set we should get the key from vault
    // Otherwise we need to know where the public_key is located?
    if config.vault_endpoint.is_some() && config.vault_token.is_some() {
        let key = helpers::get_vault_token(
            config
                .vault_endpoint
                .as_ref()
                .expect("vault endpoint is None")
                .as_ref(),
            config
                .vault_token
                .as_ref()
                .expect("vault_token is None")
                .as_ref(),
            &host_info.hostname,
        )?;
        Ok(key.as_bytes().to_vec())
    } else {
        let p = Path::new("/etc")
            .join("bynar")
            .join(format!("{}.pem", host_info.hostname));
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
    description.push_str(&format!(
        "\nDisk vendor: {:?}",
        state_machine.block_device.scsi_info.vendor
    ));
}

fn check_for_failed_disks(
    config: &ConfigSettings,
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    message_queue: &mut VecDeque<(Operation, Option<String>, Option<u32>)>,
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    host_mapping: &HostDetailsMapping,
    simulate: bool,
) -> BynarResult<()> {
    let public_key = get_public_key(config, &host_info)?;
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
        test_disk::check_all_disks(&host_info, pool, host_mapping)?
            .into_iter()
            .collect();
    // separate the states into Ok and Errors
    let usable_states: Vec<_> = match all_states {
        Ok(s) => s,
        Err(e) => {
            error!("check_all_disks failed with error: {:?}", e);
            return Err(BynarError::new(format!(
                "check_all_disks failed with error: {:?}",
                e
            )));
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
            add_or_update_map_op(
                message_map,
                &state_machine.block_device.dev_path,
                DiskOp::new(Operation::new(), None, None),
            )?;
        }
        let disks = get_disk_map_op(message_map, &state_machine.block_device.dev_path)?;
        // uh, get list of keys in disks and filter usable list for keypath?
        let mut add: Vec<_> = usable_states
            .iter()
            .filter(|state_machine| disks.contains_key(&state_machine.block_device.dev_path))
            .collect();
        add_replacing.append(&mut add);
    }
    //combine with replacing, then do sort_unstable_by and dedup_rm
    replacing.append(&mut add_replacing);
    replacing.sort_unstable_by(|a, b| {
        a.block_device
            .dev_path
            .partial_cmp(&b.block_device.dev_path)
            .unwrap()
    });
    replacing.dedup_by(|a, b| a.block_device.dev_path.eq(&b.block_device.dev_path));
    //filter Fail disks in seperate vec and soft-error those at the end before checking the errored_states
    let failed: Vec<_> = usable_states
        .iter()
        .filter(|state_machine| state_machine.block_device.state == State::Fail)
        .collect();

    replacing.iter().for_each(|state_machine| {
        //add safeToRemove + Remove request to message_queue, checking if its already in first
        // create Operation, description, and get the op_id
        let mut desc = description.clone();
        add_disk_to_description(
            &mut desc,
            &state_machine.block_device.dev_path,
            &state_machine,
        );
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
        let mut op = helpers::make_op!(
            SafeToRemove,
            format!("{}", state_machine.block_device.dev_path.display())
        );
        let mess: (Operation, Option<String>, Option<u32>) = (op, Some(desc.clone()), Some(op_id));
        let mut op2 = helpers::make_op!(
            Remove,
            format!("{}", state_machine.block_device.dev_path.display())
        );
        let mess2: (Operation, Option<String>, Option<u32>) = (op2, Some(desc), Some(op_id));
        if !message_queue.contains(&mess) && !message_queue.contains(&mess2) {
            message_queue.push_back(mess);
            message_queue.push_back(mess2);
        }
    });
    /*for result in test_disk::check_all_disks(&host_info, pool, host_mapping)? {
        match result {
            Ok(state_machine) => {
                info!(
                    "Disk status: /dev/{} {:?}",
                    state_machine.block_device.device.name, state_machine
                );
                // just use state_machine.block_device.dev_path???
                let mut dev_path = PathBuf::from("/dev");
                let dev_name = &state_machine.block_device.device.name;
                dev_path.push(&dev_name);

                if state_machine.block_device.state == State::WaitingForReplacement {
                    add_disk_to_description(&mut description, &dev_path, &state_machine);
                    trace!("Description: {}", description);
                    info!("Connecting to database to check if disk is in progress");
                    let in_progress = in_progress::is_hardware_waiting_repair(
                        pool,
                        host_mapping.storage_detail_id,
                        &dev_name,
                        None,
                    )?;
                    match (simulate, in_progress) {
                        (false, true) => {
                            debug!("Device is already in the repair queue");
                        }
                        (false, false) => {
                            debug!("Sending Safe-to-Remove and Remove requests");
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
                            /*debug!("Asking disk-manager if it's safe to remove disk");
                            // CALL RPC
                            let socket = helpers::connect(
                                &config.manager_host,
                                &config.manager_port.to_string(),
                                &public_key,
                            )?;
                            match (
                                helpers::safe_to_remove_request(&socket, &dev_path),
                                config.slack_webhook.is_some(),
                            ) {
                                (Ok((OpOutcome::Success, true)), true) => {
                                    debug!("safe to remove: true");
                                    //Ok to remove the disk
                                    let _ = notify_slack(
                                        config,
                                        &format!(
                                            "Removing disk: {} on host: {}",
                                            dev_path.display(),
                                            host_info.hostname
                                        ),
                                    );

                                    match helpers::remove_disk_request(
                                        &socket, &dev_path, None, false,
                                    ) {
                                        Ok(outcome) => match outcome {
                                            OpOutcome::Success => debug!("Disk removal successful"),
                                            OpOutcome::Skipped => {
                                                debug!("Disk skipped, disk is not removable")
                                            }
                                            OpOutcome::SkipRepeat => {
                                                debug!("Disk already removed, skipping.")
                                            }
                                        },
                                        Err(e) => {
                                            error!("Disk removal failed: {}", e);
                                        }
                                    };
                                }
                                (Ok((_, false)), true) => {
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
                                }
                                (Err(err), true) => {
                                    //Not ok to remove the disk but we need to
                                    let _ = notify_slack(
                                        &config,
                                        &format!(
                                            "Need to remove disk {} but can't tell if it's \
                                             safe on host: {}. Error: {:?}.  Filing a ticket",
                                            dev_path.display(),
                                            host_info.hostname,
                                            err
                                        ),
                                    );
                                }
                                (..) => {}
                            };
                            debug!("Creating support ticket");
                            let ticket_id =
                                create_support_ticket(config, "Bynar: Dead disk", &description)?;
                            debug!("Recording ticket id {} in database", ticket_id);
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
                            // update operation detials in DB
                            let mut operation_detail =
                                OperationDetail::new(op_id, OperationType::WaitingForReplacement);
                            operation_detail.set_tracking_id(ticket_id);
                            add_or_update_operation_detail(pool, &mut operation_detail)?;*/
                        }
                        (..) => {}
                    }
                // Handle the ones that ended up stuck in Fail
                } else if state_machine.block_device.state == State::Fail {
                    error!("Disk {} ended in a Fail state", dev_path.display(),);
                } else {
                    // The rest should be State::Good ?
                }
            }
            Err(e) => {
                error!("check_all_disks failed with error: {:?}", e);
                return Err(BynarError::new(format!(
                    "check_all_disks failed with error: {:?}",
                    e
                )));
            }
        };
    }*/
    failed.iter().for_each(|state_machine| {
        error!(
            "Disk {} ended in a Fail state",
            state_machine.block_device.dev_path.display()
        )
    });
    Ok(())
}

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
                    ref name,
                    ref serial_number,
                    ..
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
                //CALL RPC
                // add add_disk request to message_queue
                /*
                debug!("Connecting to disk-manager");
                let socket = helpers::connect(
                    &config.manager_host,
                    &config.manager_port.to_string(),
                    &public_key,
                )?;

                match helpers::add_disk_request(
                    &socket,
                    &Path::new(&ticket.device_path),
                    None,
                    simulate,
                ) {
                    Ok(outcome) => {
                        match outcome {
                            OpOutcome::Success => {
                                debug!("Disk added successfully. Updating database record")
                            }
                            // Disk was either boot or something that shouldn't be added via backend
                            OpOutcome::Skipped => debug!("Disk Skipped.  Updating database record"),
                            // Disk is already in the cluster
                            OpOutcome::SkipRepeat => {
                                debug!("Disk already added.  Skipping.  Updating database record")
                            }
                        }
                        match in_progress::resolve_ticket_in_db(pool, &ticket.ticket_id) {
                            Ok(_) => debug!("Database updated"),
                            Err(e) => {
                                error!("Failed to resolve ticket {}.  {:?}", ticket.ticket_id, e)
                            }
                        };
                    }
                    Err(e) => {
                        error!("Failed to add disk: {:?}", e);
                    }
                };
                */
            }
            Ok(false) => {}
            Err(e) => {
                error!(
                    "Error getting resolved ticket status for {}.  {:?}",
                    &ticket.ticket_id, e
                );
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
        add_or_update_map_op(message_map, &path, disk_op)?;
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

//handle return of Operation
fn handle_operation_result(
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    pool: &Pool<ConnectionManager>,
    op_res: OpOutcomeResult,
    config: &ConfigSettings,
) -> BynarResult<()> {
    match op_res.get_result() {
        ResultType::OK => {}
        ResultType::ERR => {
            if op_res.has_error_msg() {
                let msg = op_res.get_error_msg();
                match op_res.get_op_type() {
                    Op::Add => {
                        error!("Add disk failed : {}", msg);
                        return Err(BynarError::from(msg));
                    }
                    Op::Remove => {
                        error!("Remove disk failed : {}", msg);
                        return Err(BynarError::from(msg));
                    }
                    Op::SafeToRemove => {
                        error!("SafeToRemove disk failed : {}", msg);
                        // no need to error out, but update the map.  Error outcomes are expected for SafeToRemove.  
                        // Ex. you removed a disk first before the partition.
                    }
                    _ => {}
                }
            }
        }
    }

    match op_res.get_op_type() {
        Op::Add => {
            let path = Path::new(op_res.get_disk());
            if let Some(disk_op) = get_map_op(message_map, &path.to_path_buf())? {
                if let Some(ticket_id) = disk_op.description {
                    handle_add_disk_res(pool, &op_res, ticket_id);
                    //update result in the map (in otherwords, just set it to None)
                    remove_map_op(message_map, &path.to_path_buf())?;
                    return Ok(());
                }
            }
            error!(
                "Unable to get current operation in the map for {}",
                path.display()
            );
            Err(BynarError::from(format!(
                "Unable to get current operation in the map for {}",
                path.display()
            )))
        }
        Op::SafeToRemove => {
            // get the op from map, update it with outcome, handle errors as necessary (just store in map)
            let dev_path = PathBuf::from(op_res.get_disk());
            if let Some(mut current_op) = get_map_op(message_map, &dev_path)? {
                current_op.ret_val = Some(op_res);
                //push op back into map
                add_or_update_map_op(message_map, &dev_path, current_op)?;
                return Ok(());
            }
            //otherwise error....
            return Err(BynarError::from(format!(
                "{} does not have a currently running operation!",
                dev_path.display()
            )));
        }
        Op::Remove => {
            //check if successful or not and send to slack
            let dev_path = PathBuf::from(op_res.get_disk());
            match op_res.get_outcome() {
                OpOutcome::Success => {
                    debug!("Disk {} removal successful", dev_path.display());
                    let _ = notify_slack(
                        config,
                        &format!("Disk {} removal successful", dev_path.display()),
                    );
                }
                OpOutcome::Skipped => {
                    debug!("Disk {} skipped, disk is not removable", dev_path.display());
                    let _ = notify_slack(
                        config,
                        &format!("Disk {} skipped, disk is not removable", dev_path.display()),
                    );
                }
                OpOutcome::SkipRepeat => {
                    if op_res.has_value() {
                        debug!(
                            "Disk {} currently undergoing another operation, skipping",
                            dev_path.display()
                        );
                        let _ = notify_slack(
                            config,
                            &format!(
                                "Disk {} currently undergoing another operation, skipping",
                                dev_path.display()
                            ),
                        );
                    } else {
                        debug!("Disk {} already removed, skipping.", dev_path.display());
                        let _ = notify_slack(
                            config,
                            &format!("Disk {} already removed, skipping.", dev_path.display()),
                        );
                    }
                }
            }
            //update map
            if let Some(mut current_op) = get_map_op(message_map, &dev_path)? {
                current_op.ret_val = Some(op_res);
                //push op back into map
                add_or_update_map_op(message_map, &dev_path, current_op)?;
            } else {
                return Err(BynarError::from(format!(
                    "{} does not have a currently running operation!",
                    dev_path.display()
                )));
            }
            // check if all ops in the disk have finished
            let disk = get_disk_map_op(message_map, &dev_path)?;
            let mut all_finished = true;
            disk.iter().for_each(|(k, v)| {
                //check if value finished
                if let Some(val) = v {
                    if val.ret_val.is_none() {
                        all_finished = false;
                    }
                }
            });
            //if all finished open ticket+ notify slack
            if all_finished {
                // get the path of the disk
                let path =
                    if let Some(parent) = block_utils::get_parent_devpath_from_path(&dev_path)? {
                        parent
                    } else {
                        dev_path
                    };
                // get the current op associated with the disk
                if let Some(current_op) = get_map_op(message_map, &path)? {
                    let description = match current_op.description {
                        Some(d) => d,
                        None => {
                            return Err(BynarError::from(format!(
                                "Disk {} is missing a description",
                                path.display()
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
                    let ticket_id =
                        create_support_ticket(config, "Bynar: Dead disk", &description)?;
                    debug!("Recording ticket id {} in database", ticket_id);
                    // update operation detials in DB
                    let mut operation_detail =
                        OperationDetail::new(op_id, OperationType::WaitingForReplacement);
                    operation_detail.set_tracking_id(ticket_id);
                    add_or_update_operation_detail(pool, &mut operation_detail)?;
                    return Ok(());
                }
                return Err(BynarError::from(format!(
                    "Disk {} is missing the current operation",
                    path.display()
                )));
            }
            Ok(())
        }
        _ => {
            // these operations should never get called by Bynar
            return Err(BynarError::from(format!(
                "{} could not have run this operation!",
                op_res.get_disk()
            )));
        }
    }
}

// check if the socket is readable/writable and send/recieve message if possible
fn send_and_recieve(
    s: &Socket,
    message_map: &mut HashMap<PathBuf, HashMap<PathBuf, Option<DiskOp>>>,
    message_queue: &mut VecDeque<(Operation, Option<String>, Option<u32>)>,
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
            let path = Path::new(mess.get_disk()).to_path_buf();
            //check if there was a previous request, and whether it was completed
            if let Some(disk_op) = get_map_op(&message_map, &path.to_path_buf())? {
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
                            disk_op
                        );
                        send_and_update(s, message_map, client_id, (mess, desc, op_id), &path)?;
                        trace!("Updated map {:?}", message_map);
                    }
                } else {
                    // we haven't gotten response from previous request yet, push request to back of queue
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
                    handle_operation_result(message_map, pool, outcome, config)?;
                }
                None => {
                    //Actually, this is a problem since Bynar only sends Add/SafeToRemove/Remove requests
                    error!("Message is not an OpOutcomeResult");
                    return Err(BynarError::from(format!(
                        "Message received is not an OpOutcomeResult"
                    )));
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
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("daemon")
                .help("Run Bynar as a daemon")
                .long("daemon")
                .required(false),
        )
        .arg(
            Arg::with_name("time")
                .help("Time in seconds between Bynar runs")
                .long("time")
                .default_value("5"),
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
        warn!(
            "Config directory {} doesn't exist. Creating",
            config_dir.display()
        );
        if let Err(e) = create_dir(config_dir) {
            error!(
                "Unable to create directory {}: {}",
                config_dir.display(),
                e.to_string()
            );
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
    let config: ConfigSettings = config.expect("Failed to load config");
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
    let s = match helpers::connect(
        &config.manager_host,
        &config.manager_port.to_string(),
        &public_key,
    ) {
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
    let mut message_map = create_msg_map().unwrap();
    'outer: loop {
        let now = Instant::now();
        match check_for_failed_disks(
            &config,
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
                                return;
                            }
                            let config: ConfigSettings =
                                config_file.expect("Failed to load config");
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
            send_and_recieve(
                &s,
                &mut message_map,
                &mut message_queue,
                &db_pool,
                &config,
                client_id.clone(),
            )
            .unwrap();
        }
        debug!("Request Map after looping {:?}", message_map);
        debug!("Message Queue after looping {:?}", message_queue);
    }
    debug!("Bynar exited successfully");
    notify_slack(
        &config,
        &format!("Bynar on host  {} has stopped", host_info.hostname),
    )
    .expect("Unable to connect to slack");
}
