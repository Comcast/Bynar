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

use api::service::Disk;
use clap::{Arg, App, SubCommand};
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

fn add_disk(s: &mut Socket, path: &Path) -> Result<(), String> {
    helpers::add_disk_request(s, path)?;
    Ok(())
}

//fn check_disks(s: &mut Socket) -> Result<RepairResponse, String> {
//    Ok(helpers::check_disk_request(s)?)
//}

fn list_disks(s: &mut Socket) -> Result<Vec<Disk>, String> {
    let disks = helpers::list_disks_request(s)?;
    println!("disk list: {:?}", disks);

    Ok(disks)
}

fn remove_disk(s: &mut Socket, path: &Path) -> Result<(), String> {
    helpers::remove_disk_request(s, path)?;
    Ok(())
}


fn main() {
    let matches = App::new("Ceph Disk Manager Client")
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
                ),
        )
        //.subcommand(SubCommand::with_name("check").about(
        //    "Check all disks on a server",
        //))
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
                ),
        )
        .arg(Arg::with_name("v").short("v").multiple(true).help(
            "Sets the level of verbosity",
        ))
        .get_matches();
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
        let p = Path::new(matches.value_of("path").unwrap());
        info!("Adding disk: {}", p.display());
        match add_disk(&mut s, &p) {
            Ok(_) => {
                println!("Adding disk successful");
            }
            Err(e) => {
                println!("Adding disk failed: {}", e);
            }
        };
    }
    /*
    if let Some(ref matches) = matches.subcommand_matches("check") {
        info!("Checking disks");
        match check_disks(&mut s) {
            Ok(response) => {
                println!("Checking disks completed");
                println!("Repair response: {:?}", response);
            }
            Err(e) => {
                println!("Checking disks failed: {}", e);
            }
        };
    }
    */
    if matches.subcommand_matches("list").is_some() {
        info!("Listing disks");
        match list_disks(&mut s) {
            Ok(disks) => {
                println!("Disk list: {:?}", disks);
            }
            Err(e) => {
                println!("Listing disks failed: {}", e);
            }
        };
    }
    if let Some(ref matches) = matches.subcommand_matches("remove") {
        let p = Path::new(matches.value_of("path").unwrap());
        info!("Adding disk: {}", p.display());
        match remove_disk(&mut s, &p) {
            Ok(_) => {
                println!("Removing disk successful");
            }
            Err(e) => {
                println!("Removing disk failed: {}", e);
            }
        }
    }
}
