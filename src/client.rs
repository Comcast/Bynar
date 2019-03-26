/// This is built into a separate binary called bynar-client
use std::fs::{read_to_string, File};
use std::path::Path;
use std::str::FromStr;

use api::service::Disk;
use clap::{crate_authors, crate_version, App, Arg, ArgMatches, SubCommand};
use helpers::error::BynarResult;
use hostname::get_hostname;
use log::{error, info};
use native_tls::{Identity, TlsAcceptor};
use simplelog::{CombinedLogger, Config, TermLogger, WriteLogger};
use websocket::client::sync::Client;
use websocket::stream::sync::{TcpStream, TlsStream};
/*
    CLI client to call functions over RPC
*/

fn add_disk(
    s: &mut Client<TlsStream<TcpStream>>,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> BynarResult<()> {
    helpers::add_disk_request(s, path, id, simulate)?;
    Ok(())
}

fn list_disks(s: &mut Client<TlsStream<TcpStream>>) -> BynarResult<Vec<Disk>> {
    let disks = helpers::list_disks_request(s)?;
    println!("disk list: {:?}", disks);

    Ok(disks)
}

fn remove_disk(
    s: &mut Client<TlsStream<TcpStream>>,
    path: &Path,
    id: Option<u64>,
    simulate: bool,
) -> BynarResult<()> {
    helpers::remove_disk_request(s, path, id, simulate)?;
    Ok(())
}

fn handle_add_disk(s: &mut Client<TlsStream<TcpStream>>, matches: &ArgMatches<'_>) {
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

fn handle_list_disks(s: &mut Client<TlsStream<TcpStream>>) {
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

fn handle_remove_disk(s: &mut Client<TlsStream<TcpStream>>, matches: &ArgMatches<'_>) {
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
    match remove_disk(s, &p, id, simulate) {
        Ok(_) => {
            println!("Removing disk successful");
        }
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
    let server_pubkey = read_to_string(matches.value_of("server_key").unwrap()).unwrap();

    let mut s = match helpers::connect(host, port, &server_pubkey) {
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
