///#![cfg_attr(test, feature(test, proc_macro_mod))]
/// Detect dead disks in a ceph cluster
/// 1. Detect dead disk
/// 2. Report dead disk to JIRA for repairs
/// 3. Test for resolution
/// 4. Put disk back into cluster
#[macro_use]
extern crate serde_derive;

mod create_support_ticket;
mod in_progress;
mod test_disk;
mod test_hardware;

use crate::create_support_ticket::{create_support_ticket, ticket_resolved};
use crate::in_progress::*;
use crate::test_disk::State;
use clap::{crate_authors, crate_version, App, Arg};
use helpers::{error::*, host_information::Host};
use log::{debug, error, info, warn};
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager as ConnectionManager;
use simplelog::{CombinedLogger, Config, SharedLogger, TermLogger, WriteLogger};
use slack_hook::{PayloadBuilder, Slack};
use std::fs::{create_dir, read_to_string, File};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigSettings {
    db_location: String,
    manager_host: String,
    manager_port: u16,
    /// Redfish Ip address or dns name ( Usually iLo where redfish is listening)
    redfish_ip: Option<String>,
    /// Redfish credentials
    redfish_username: Option<String>,
    /// Redfish credentials
    redfish_password: Option<String>,
    /// The port redfish is listening on
    redfish_port: Option<u16>,
    slack_webhook: Option<String>,
    slack_channel: Option<String>,
    slack_botname: Option<String>,
    vault_endpoint: Option<String>,
    vault_token: Option<String>,
    pub jira_user: String,
    pub jira_password: String,
    pub jira_host: String,
    pub jira_issue_type: String,
    pub jira_priority: String,
    pub jira_project_id: String,
    pub jira_ticket_assignee: String,
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

fn notify_slack(config: &ConfigSettings, msg: &str) -> BynarResult<()> {
    let c = config.clone();
    let slack = Slack::new(c.slack_webhook.unwrap().as_ref())?;
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

fn get_public_key(config: &ConfigSettings, host_info: &Host) -> BynarResult<String> {
    // If vault_endpoint and token are set we should get the key from vault
    // Otherwise we need to know where the public_key is located?
    if config.vault_endpoint.is_some() && config.vault_token.is_some() {
        let key = helpers::get_vault_token(
            config.vault_endpoint.clone().unwrap().as_ref(),
            config.vault_token.clone().unwrap().as_ref(),
            &host_info.hostname,
        )?;
        Ok(key)
    } else {
        let p = Path::new("/etc")
            .join("bynar")
            .join(format!("{}.pem", host_info.hostname));
        if !p.exists() {
            error!("{} does not exist", p.display());
        }
        let key = read_to_string(p)?;
        Ok(key)
    }
}

fn check_for_failed_disks(
    config: &ConfigSettings,
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
    for result in test_disk::check_all_disks(&host_info, pool, host_mapping)? {
        match result {
            Ok(state_machine) => {
                info!(
                    "Disk status: /dev/{} {:?}",
                    state_machine.block_device.device.name, state_machine
                );
                let mut dev_path = PathBuf::from("/dev");
                let dev_name = state_machine.block_device.device.name.clone();

                dev_path.push(state_machine.block_device.device.name);

                if state_machine.block_device.state == State::WaitingForReplacement {
                    description.push_str(&format!("\nDisk path: {}", dev_path.display()));
                    if let Some(serial) = state_machine.block_device.device.serial_number {
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
                    info!("Connecting to database to check if disk is in progress");
                    let in_progress = in_progress::is_hardware_waiting_repair(
                        pool,
                        host_mapping.storage_detail_id,
                        &dev_name, None
                    )?;
                    if !simulate {
                        if !in_progress {
                            debug!("Asking disk-manager if it's safe to remove disk");
                            // CALL RPC
                            let mut socket = helpers::connect(
                                &config.manager_host,
                                &config.manager_port.to_string(),
                                &public_key,
                            )?;
                            match helpers::safe_to_remove_request(&mut socket, &dev_path) {
                                Ok(result) => {
                                    debug!("safe to remove: {}", result);
                                    //Ok to remove the disk
                                    if result {
                                        if config.slack_webhook.is_some() {
                                            let _ = notify_slack(
                                                config,
                                                &format!(
                                                    "Removing disk: {} on host: {}",
                                                    dev_path.display(),
                                                    host_info.hostname
                                                ),
                                            );
                                        }
                                        match helpers::remove_disk_request(
                                            &mut socket,
                                            &dev_path,
                                            None,
                                            false,
                                        ) {
                                            Ok(_) => {
                                                debug!("Disk removal successful");
                                            }
                                            Err(e) => {
                                                error!("Disk removal failed: {}", e);
                                            }
                                        };
                                    } else if config.slack_webhook.is_some() {
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
                                }
                                Err(err) => {
                                    //Not ok to remove the disk but we need to
                                    if config.slack_webhook.is_some() {
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
                                }
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
                            let mut operation_detail =
                                OperationDetail::new(op_id, OperationType::WaitingForReplacement);
                            operation_detail.set_tracking_id(ticket_id);
                            add_or_update_operation_detail(pool, &mut operation_detail)?;
                        } else {
                            debug!("Device is already in the repair queue");
                        }
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
    }
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
                BynarError::HardwareError { ref name, ref serial_number, .. } => {
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

fn add_repaired_disks(
    config: &ConfigSettings,
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    storage_detail_id: u32,
    simulate: bool,
) -> BynarResult<()> {
    let public_key = get_public_key(&config, &host_info)?;

    info!("Getting outstanding repair tickets");
    let tickets = in_progress::get_outstanding_repair_tickets(&pool, storage_detail_id)?;
    debug!("outstanding tickets: {:?}", tickets);
    info!("Checking for resolved repair tickets");
    for ticket in tickets {
        match ticket_resolved(config, &ticket.ticket_id.to_string()) {
            Ok(resolved) => {
                if resolved {
                    //CALL RPC
                    debug!("Connecting to disk-manager");
                    let mut socket = helpers::connect(
                        &config.manager_host,
                        &config.manager_port.to_string(),
                        &public_key,
                    )?;
                    match helpers::add_disk_request(
                        &mut socket,
                        &Path::new(&ticket.device_path),
                        None,
                        simulate,
                    ) {
                        Ok(_) => {
                            debug!("Disk added successfully. Updating database record");
                            match in_progress::resolve_ticket_in_db(pool, &ticket.ticket_id) {
                                Ok(_) => {
                                    debug!("Database updated");
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to resolve ticket {}.  {:?}",
                                        ticket.ticket_id, e
                                    );
                                }
                            };
                        }
                        Err(e) => {
                            error!("Failed to add disk: {:?}", e);
                        }
                    };
                }
            }
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
        .get_matches();
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
        File::create("/var/log/bynar.log").expect("/var/log/bynar.log creation failed"),
    ));
    let _ = CombinedLogger::init(loggers);
    info!("Starting up");

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
    let simulate = matches.is_present("simulate");
    let h_info = Host::new();
    if h_info.is_err() {
        error!("Failed to gather host information");
        //gracefully exit
        return;
    }
    let host_info = h_info.expect("Failed to gather host information");
    debug!("Gathered host info: {:?}", host_info);
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

    match check_for_failed_disks(
        &config,
        &host_info,
        &db_pool,
        &host_details_mapping,
        simulate,
    ) {
        Err(e) => {
            error!("Check for failed disks failed with error: {}", e);
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
        }
        _ => {
            info!("Check for failed hardware completed");
        }
    };
    match add_repaired_disks(
        &config,
        &host_info,
        &db_pool,
        host_details_mapping.storage_detail_id,
        simulate,
    ) {
        Err(e) => {
            error!("Add repaired disks failed with error: {}", e);
        }
        _ => {
            info!("Add repaired disks completed");
        }
    };
}
