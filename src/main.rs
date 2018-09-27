#![cfg_attr(test, feature(test, proc_macro_mod))]
/// Detect dead disks in a ceph cluster
/// 1. Detect dead disk
/// 2. Report dead disk to JIRA for repairs
/// 3. Test for resolution
/// 4. Put disk back into cluster
extern crate api;
#[macro_use]
extern crate clap;
extern crate helpers;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate protobuf;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate simplelog;
extern crate slack_hook;
extern crate zmq;

mod create_support_ticket;
mod in_progress;
mod test_disk;

use std::fs::{create_dir, read_to_string, File};
use std::path::{Path, PathBuf};

use self::test_disk::State;
use clap::{App, Arg};
use create_support_ticket::{create_support_ticket, ticket_resolved};
use helpers::{error::*, host_information::Host};
use simplelog::{CombinedLogger, Config, SharedLogger, TermLogger, WriteLogger};
use slack_hook::{PayloadBuilder, Slack};

#[derive(Clone, Debug, Deserialize)]
pub struct ConfigSettings {
    db_location: String,
    manager_host: String,
    manager_port: u16,
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

fn check_for_failed_disks(config_dir: &Path, simulate: bool) -> BynarResult<()> {
    let config: ConfigSettings =
        helpers::load_config(config_dir, "bynar.json")?;
    let host_info = Host::new()?;
    debug!("Gathered host info: {:?}", host_info);
    let public_key = get_public_key(&config, &host_info)?;
    let config_location = Path::new(&config.db_location);
    //Host information to use in ticket creation
    let mut description = format!("A disk on {} failed. Please replace.", host_info.hostname);
    let environment = format!(
        "Hostname: {}\nServer type: {}\nServer Serial: {}\nMachine Architecture: {}\nKernel: {}",
        host_info.hostname,
        host_info.server_type,
        host_info.serial_number,
        host_info.machine_architecture,
        host_info.kernel,
    );

    info!("Checking all drives");
    let conn =
        in_progress::connect_to_repair_database(&config_location)?;
    for result in test_disk::check_all_disks(&config_location)? {
        match result {
            Ok(state) => {
                info!("Disk status: {:?}", state);
                let mut dev_path = PathBuf::from("/dev");
                dev_path.push(state.disk.name);

                if state.state == State::WaitingForReplacement {
                    description.push_str(&format!("\nDisk path: {}", dev_path.display()));
                    if let Some(serial) = state.disk.serial_number {
                        description.push_str(&format!("\nDisk serial: {}", serial));
                    }
                    info!("Connecting to database to check if disk is in progress");
                    let in_progress = in_progress::is_disk_in_progress(&conn, &dev_path)?;
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
                                    //Ok to remove the disk
                                    if result {
                                        if config.slack_webhook.is_some() {
                                            let _ = notify_slack(
                                                &config,
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
                                            &config,
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
                            let ticket_id = create_support_ticket(
                                &config,
                                "Dead disk",
                                &description,
                                &environment,
                            )?;
                            debug!("Recording ticket id {} in database", ticket_id);
                            in_progress::record_new_repair_ticket(&conn, &ticket_id, &dev_path)?;
                        } else {
                            debug!("Device is already in the repair queue");
                        }
                    }
                // Handle the ones that ended up stuck in Fail
                } else if state.state == State::Fail {
                    error!("Disk {} ended in a Fail state", dev_path.display(),);
                } else {
                    // The rest should be State::Good ?
                }
            }
            Err(e) => {
                error!("check_all_disks failed with error: {:?}", e);
                return Err(BynarError::new(format!("check_all_disks failed with error: {:?}", e)));
            }
        };
    }
    Ok(())
}

fn add_repaired_disks(config_dir: &Path, simulate: bool) -> BynarResult<()> {
    let config: ConfigSettings =
        helpers::load_config(config_dir, "bynar.json")?;
    let host_info = Host::new()?;
    let config_location = Path::new(&config.db_location);
    let public_key = get_public_key(&config, &host_info)?;

    info!("Connecting to database to find repaired drives");
    let conn =
        in_progress::connect_to_repair_database(&config_location)?;
    info!("Getting outstanding repair tickets");
    let tickets = in_progress::get_outstanding_repair_tickets(&conn)?;
    info!("Checking for resolved repair tickets");
    for ticket in tickets {
        match ticket_resolved(&config, &ticket.ticket_id.to_string()) {
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
                        &Path::new(&ticket.disk_path),
                        None,
                        simulate,
                    ) {
                        Ok(_) => {
                            debug!("Disk added successfully");
                            match in_progress::resolve_ticket(&conn, &ticket.ticket_id) {
                                Ok(_) => {
                                    debug!("Database updated");
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to delete record for {}.  {:?}",
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
                    "Error gatting resolved ticket status for {}.  {:?}",
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
        ).arg(
            Arg::with_name("simulate")
                .help("Log messages but take no action")
                .long("simulate")
                .required(false),
        ).arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        ).get_matches();
    let level = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Info, //default
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    let mut loggers: Vec<Box<SharedLogger>> = vec![];
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

    match check_for_failed_disks(config_dir, simulate) {
        Err(e) => {
            error!("Check for failed disks failed with error: {}", e);
        }
        _ => {
            info!("Check for failed disks completed");
        }
    };
    match add_repaired_disks(config_dir, simulate) {
        Err(e) => {
            error!("Add repaired disks failed with error: {}", e);
        }
        _ => {
            info!("Add repaired disks completed");
        }
    };
}
