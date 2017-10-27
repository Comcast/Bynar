extern crate api;
#[macro_use]
extern crate clap;
extern crate helpers;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate simplelog;
extern crate zmq;

use std::path::Path;
use std::str::FromStr;

use api::service::Disk;
use clap::{Arg, ArgMatches, App, SubCommand};
use simplelog::{Config, SimpleLogger};
use zmq::Socket;
use zmq::Result as ZmqResult;
/*
    CLI client to call functions over RPC
*/

fn connect(host: &str, port: &str) -> ZmqResult<Socket> {
    debug!("Starting zmq sender with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ)?;
    assert!(
        requester
            .connect(&format!("tcp://{}:{}", host, port))
            .is_ok()
    );

    Ok(requester)
}

fn add_disk(s: &mut Socket, path: &Path, id: Option<u64>, simulate: bool) -> Result<(), String> {
    helpers::add_disk_request(s, path, id, simulate)?;
    Ok(())
}

fn list_disks(s: &mut Socket) -> Result<Vec<Disk>, String> {
    let disks = helpers::list_disks_request(s)?;
    println!("disk list: {:?}", disks);

    Ok(disks)
}

fn remove_disk(s: &mut Socket, path: &Path, id: Option<u64>, simulate: bool) -> Result<(), String> {
    helpers::remove_disk_request(s, path, id, simulate)?;
    Ok(())
}

fn handle_add_disk(s: &mut Socket, matches: &ArgMatches) {
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
    match add_disk(s, &p, id, simulate) {
        Ok(_) => {
            println!("Adding disk successful");
        }
        Err(e) => {
            println!("Adding disk failed: {}", e);
        }
    };
}

fn handle_list_disks(s: &mut Socket) {
    info!("Listing disks");
    match list_disks(s) {
        Ok(disks) => {
            println!("Disk list: {:?}", disks);
        }
        Err(e) => {
            println!("Listing disks failed: {}", e);
        }
    };
}

fn handle_remove_disk(s: &mut Socket, matches: &ArgMatches) {
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
    match remove_disk(s, &p, id, simulate) {
        Ok(_) => {
            println!("Removing disk successful");
        }
        Err(e) => {
            println!("Removing disk failed: {}", e);
        }
    }
}

fn get_vault_token(
    endpoint: &str,
    token: &str,
    key: &str,
) -> Result<String, ::hashicorp_vault::client::error::Result> {
    let client = VaultClient::new(host, token)?;
    let res = client.get_secret(key)?;
    res
}

fn get_cli_args<'a>() -> ArgMatches<'a> {
    App::new("Ceph Disk Manager Client")
        .version(crate_version!())
        .author(crate_authors!())
        .about(
            "Detect dead hard drives, create a support ticket and watch for resolution",
        )
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
        .subcommand(SubCommand::with_name("list").about(
            "List all disks on a server",
        ))
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
        .arg(Arg::with_name("v").short("v").multiple(true).help(
            "Sets the level of verbosity",
        ))
        .get_matches()
}

fn main() {
    let matches = get_cli_args();
    let level = match matches.occurrences_of("v") {
        0 => log::LogLevelFilter::Info, //default
        1 => log::LogLevelFilter::Debug,
        _ => log::LogLevelFilter::Trace,
    };
    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();
    let _ = SimpleLogger::init(level, Config::default());
    info!("Starting up");

    let mut s = match connect(host, port) {
        Ok(s) => s,
        Err(e) => {
            error!("Error connecting to socket: {:?}", e);
            return;
        }
    };
    if let Some(ref matches) = matches.subcommand_matches("add") {
        handle_add_disk(&mut s, matches);
    }
    if matches.subcommand_matches("list").is_some() {
        handle_list_disks(&mut s);
    }
    if let Some(ref matches) = matches.subcommand_matches("remove") {
        handle_remove_disk(&mut s, matches);
    }
}
