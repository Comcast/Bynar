/// This is built into a separate binary called bynar-client
//mod disk_manager;
use std::fs::{read_to_string, File};
use std::io::{Error, ErrorKind, Read, Write};
use std::path::Path;
use std::str::FromStr;

//use disk_manager::disk_manager;
use api::service::{
    Disk, Disks, JiraInfo, OpJiraTicketsResult, OpOutcome, OpOutcomeResult, ResultType,
};
use clap::{crate_authors, crate_version, App, Arg, ArgMatches, SubCommand};
use helpers::error::{BynarError, BynarResult};
use hostname::get_hostname;
use log::{debug, error, info, trace};
use simplelog::{CombinedLogger, Config, TermLogger, WriteLogger};
use zmq::Socket;

#[macro_use]
mod util;
/*
    CLI client to call functions over RPC
*/

fn add_disk(
    s: &Socket,
    path: &Path,
    id: Option<u64>,
    client_id: Vec<u8>,
    simulate: bool,
) -> BynarResult<OpOutcome> {
    helpers::add_disk_request(s, path, id, client_id, simulate)?;
    //loop until socket is readable, then get the response
    loop {
        let events = poll_events!(s, continue);
        // got response
        if events.contains(zmq::PollEvents::POLLIN) {
            let message = helpers::get_messages(s)?;
            let op_result = get_message!(OpOutcomeResult, &message)?;
            get_op_result!(op_result, add_disk);
        }
    }
}

fn list_disks(s: &Socket, client_id: Vec<u8>) -> BynarResult<Vec<Disk>> {
    helpers::list_disks_request(s, client_id)?;
    //loop until socket is readable, then get the response
    loop {
        let events = poll_events!(s, continue);
        // got response
        if events.contains(zmq::PollEvents::POLLIN) {
            let message = helpers::get_messages(s)?;
            let disks = get_message!(Disks, &message)?;
            let mut d: Vec<Disk> = Vec::new();
            for disk in disks.get_disk() {
                d.push(disk.clone());
            }
            println!("disk list: {:?}", d);
            return Ok(d);
        }
    }
}

fn remove_disk(
    s: &Socket,
    path: &Path,
    id: Option<u64>,
    client_id: Vec<u8>,
    simulate: bool,
) -> BynarResult<OpOutcome> {
    helpers::remove_disk_request(s, path, id, client_id, simulate)?;

    //loop until socket is readable, then get the response
    loop {
        let events = poll_events!(s, continue);
        // got response
        if events.contains(zmq::PollEvents::POLLIN) {
            let message = helpers::get_messages(s)?;
            let op_result = get_message!(OpOutcomeResult, &message)?;
            get_op_result!(op_result, remove_disk);
        }
    }
}

fn handle_add_disk(s: &Socket, matches: &ArgMatches<'_>, client_id: Vec<u8>) {
    let p = Path::new(matches.value_of("path").unwrap());
    info!("Adding disk: {}", p.display());
    let id = match matches.value_of("id") {
        Some(i) => Some(u64::from_str(&i).unwrap()),
        None => None,
    };
    let simulate = match matches.value_of("simulate") {
        Some(s) => bool::from_str(&s).unwrap(),
        None => false,
    };
    match add_disk(s, &p, id, client_id, simulate) {
        Ok(outcome) => match outcome {
            OpOutcome::Success => println!("Adding disk successful"),
            OpOutcome::Skipped => println!("Disk cannot be added, Skipping"),
            OpOutcome::SkipRepeat => println!("Disk already added, Skipping"),
        },
        Err(e) => {
            println!("Adding disk failed: {}", e);
        }
    };
}

fn handle_list_disks(s: &Socket, client_id: Vec<u8>) {
    info!("Listing disks");
    match list_disks(s, client_id) {
        Ok(disks) => {
            println!("Disk list: {:?}", disks);
        }
        Err(e) => {
            println!("Listing disks failed: {}", e);
        }
    };
}

fn handle_jira_tickets(s: &Socket, client_id: Vec<u8>) -> BynarResult<()> {
    trace!("handle_jira_tickets called");
    helpers::get_jira_tickets(s, client_id)?;
    //loop until socket is readable, then get the response
    loop {
        let events = poll_events!(s, continue);
        // got response
        if events.contains(zmq::PollEvents::POLLIN) {
            let message = helpers::get_messages(s)?;
            let tickets = get_message!(OpJiraTicketsResult, &message)?;
            match tickets.get_result() {
                ResultType::OK => {
                    debug!("got tickets successfully");
                    let proto_jira = tickets.get_tickets();
                    let mut _jira: Vec<JiraInfo> = Vec::new();
                    for JiraInfo in proto_jira {
                        debug!("get_ticket_id: {}", JiraInfo.get_ticket_id());
                        debug!("get_server_name: {}", JiraInfo.get_server_name());
                    }
                    return Ok(());
                }
                ResultType::ERR => {
                    if tickets.has_error_msg() {
                        let msg = tickets.get_error_msg();
                        error!("get jira tickets failed : {}", msg);
                        return Err(BynarError::from(tickets.get_error_msg()));
                    } else {
                        error!("Get jira tickets failed but error_msg not set");
                        return Err(BynarError::from(
                            "Get jira tickets failed but error_msg not set",
                        ));
                    }
                }
            }
        }
    }
}

fn handle_remove_disk(s: &Socket, matches: &ArgMatches<'_>, client_id: Vec<u8>) {
    let p = Path::new(matches.value_of("path").unwrap());
    info!("Removing disk: {}", p.display());
    let id = match matches.value_of("id") {
        Some(i) => Some(u64::from_str(&i).unwrap()),
        None => None,
    };
    let simulate = match matches.value_of("simulate") {
        Some(s) => bool::from_str(&s).unwrap(),
        None => false,
    };
    match remove_disk(s, &p, id, client_id, simulate) {
        Ok(outcome) => match outcome {
            OpOutcome::Success => println!("Removing disk successful"),
            OpOutcome::Skipped => println!("Disk cannot be removed.  Skipping"),
            OpOutcome::SkipRepeat => println!("Disk already removed.  Skipping"),
        },
        Err(e) => {
            println!("Removing disk failed: {}", e);
        }
    }
}

fn get_cli_args(default_server_key: &str) -> ArgMatches<'_> {
    App::new("Ceph Disk Manager Client")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Manually make RPC calls to the disk-manager to list, add or remove disks")
        .arg(
            Arg::with_name("host")
                .default_value("localhost")
                .help("The host to call for service")
                .long("host")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .default_value("5555")
                .help("The port to call for service")
                .required(false)
                .short("p")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("server_key")
                .default_value(default_server_key)
                .help("The public key for the disk-manager service.")
                .required(false)
                .long("serverkey")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("Add a disk into the cluster")
                .arg(
                    Arg::with_name("path")
                        .help("The disk path to add: Ex: /dev/sda")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("id")
                        .help("An optional id to set for the disk.  Used for ceph osds")
                        .long("id")
                        .required(false)
                        .takes_value(true)
                        .validator(|v| match u64::from_str(&v) {
                            Ok(_) => Ok(()),
                            Err(_) => Err("id must be a valid u64".to_string()),
                        }),
                )
                .arg(
                    Arg::with_name("simulate")
                        .default_value("false")
                        .help("Simulate the operation")
                        .long("simulate")
                        .possible_values(&["false", "true"])
                        .required(false)
                        .takes_value(true),
                ),
        )
        .subcommand(SubCommand::with_name("list").about("List all disks on a server"))
        .subcommand(SubCommand::with_name("get_jira_tickets").about("get all tickets created"))
        .subcommand(
            SubCommand::with_name("remove")
                .about("Remove a disk from the cluster")
                .arg(
                    Arg::with_name("path")
                        .help("The disk path to add: Ex: /dev/sda")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("id")
                        .help("An optional id to set for the disk.  Used for ceph osds")
                        .long("id")
                        .required(false)
                        .takes_value(true)
                        .validator(|v| match u64::from_str(&v) {
                            Ok(_) => Ok(()),
                            Err(_) => Err("id must be a valid u64".to_string()),
                        }),
                )
                .arg(
                    Arg::with_name("simulate")
                        .default_value("false")
                        .help("Simulate the operation")
                        .long("simulate")
                        .possible_values(&["false", "true"])
                        .required(false)
                        .takes_value(true),
                ),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches()
}

fn main() {
    let server_key = format!(
        "/etc/bynar/{}.pem",
        get_hostname().unwrap_or_else(|| "ecpubkey".to_string())
    );
    let matches = get_cli_args(&server_key);
    let level = match matches.occurrences_of("v") {
        0 => log::LevelFilter::Info, //default
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();
    let _ = CombinedLogger::init(vec![
        TermLogger::new(level, Config::default()).unwrap(),
        WriteLogger::new(
            level,
            Config::default(),
            File::create("/var/log/bynar-client.log").unwrap(),
        ),
    ]);
    info!("Starting up");
    let mut server_pubkey = Vec::new();
    let mut keyfile = File::open(matches.value_of("server_key").unwrap()).unwrap();
    keyfile.read_to_end(&mut server_pubkey).unwrap();

    let s = match helpers::connect(host, port, &server_pubkey) {
        Ok(s) => s,
        Err(e) => {
            error!("Error connecting to socket: {:?}", e);
            return;
        }
    };
    let client_id: Vec<u8> = s.get_identity().unwrap();
    if let Some(ref matches) = matches.subcommand_matches("add") {
        handle_add_disk(&s, matches, client_id.clone());
    }
    if matches.subcommand_matches("list").is_some() {
        handle_list_disks(&s, client_id.clone());
    }
    if let Some(ref matches) = matches.subcommand_matches("remove") {
        handle_remove_disk(&s, matches, client_id.clone());
    }
    if let Some(ref _matches) = matches.subcommand_matches("get_jira_tickets") {
        match handle_jira_tickets(&s, client_id.clone()) {
            Ok(()) => {}
            Err(e) => println!("Get JIRA tickets failed {}", e),
        };
    }
}
