extern crate api;
#[macro_use]
extern crate clap;
extern crate helpers;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate rayon;
extern crate simplelog;
extern crate zmq;

use std::io::Result as IOResult;
use std::io::Read;
use std::fs::File;

use api::service::Disk;
use clap::{Arg, App};
use helpers::{connect, list_disks_request};
use rayon::prelude::*;
use simplelog::{Config, SimpleLogger};

fn get_host_list(host_file: &str) -> IOResult<Vec<String>> {
    let mut host_list: Vec<String> = Vec::new();
    let mut f = File::open(host_file)?;
    let mut buff = String::new();
    f.read_to_string(&mut buff)?;

    for l in buff.lines() {
        host_list.push(l.to_string());
    }
    Ok(host_list)
}

fn parallel_deploy(host_list: Vec<String>, port: &str) -> Result<(), String> {
    let sorted_disks = get_disk_list(host_list, port)?;

    println!("Sorted hosts: {:?}", sorted_disks);
    Ok(())
}

// Returns a Vec of (Hostname, Vec<Disk>).  The results will be sorted by hostname
fn get_disk_list(host_list: Vec<String>, port: &str) -> Result<Vec<(String, Vec<Disk>)>, String> {
    let mut disks: Vec<(String, Vec<Disk>)> = vec![];
    // Collect all disks in parallel
    host_list
        .par_iter()
        .map(|host| {
            let mut s = match connect(host, port) {
                Ok(s) => s,
                Err(e) => {
                    error!("Unable to connect to host {}, error: {:?}", host, e);
                    //return Err(format!("Unable to connect to {}", host));
                    let disks: Vec<Disk> = vec![];
                    return (host.to_string(), disks);
                }
            };
            let host_disks = match list_disks_request(&mut s) {
                Ok(disks) => disks,
                Err(e) => {
                    error!("Unable to list disks on host {}, error: {:?}", host, e);
                    let disks: Vec<Disk> = vec![];
                    return (host.to_string(), disks);
                }
            };
            (host.clone().to_string(), host_disks)
        })
        .collect_into(&mut disks);
    // Sort by hostname
    disks.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    Ok(disks)
}

fn main() {
    let matches = App::new("Parallel OSD deployment")
        .version(crate_version!())
        .author(crate_authors!())
        .about(
            "Deploy osds in parallel for when you've got better things to do with your time",
        )
        .arg(
            Arg::with_name("host_list")
                .help("A file with 1 host per line to deploy to")
                .long("hostlist")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("port")
                .default_value("5555")
                .help("The tcp port that disk-manager is listening on")
                .long("port")
                .required(false),
        )
        .arg(
            Arg::with_name("simulate")
                .help("Log messages but take no action")
                .long("simulate")
                .required(false),
        )
        .arg(
            Arg::with_name("wait")
                .default_value("600")
                .help("Max time to wait before killing a connection in secs")
                .long("wait")
                .required(false),
        )
        .arg(Arg::with_name("v").short("v").multiple(true).help(
            r#"Sets the level of verbosity.  Because of the parallel nature of this program the "
            "debug setting might be extremely noisy"#,
        ))
        .get_matches();
    let level = match matches.occurrences_of("v") {
        0 => log::LogLevelFilter::Info, //default
        1 => log::LogLevelFilter::Debug,
        _ => log::LogLevelFilter::Trace,
    };
    let _ = SimpleLogger::init(level, Config::default());
    info!("Starting up");
    let hosts = match get_host_list(matches.value_of("host_list").unwrap()) {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to read host list: {:?}", e);
            return;
        }
    };
    match parallel_deploy(hosts, matches.value_of("port").unwrap()) {
        Ok(_) => {
            info!("Successfully deployed all osds");
        }
        Err(e) => {
            error!("Failed to deploy all osds: {:?}", e);
            return;
        }
    };
}
