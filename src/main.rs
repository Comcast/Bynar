
/// Detect dead disks in a ceph cluster
/// 1. Detect dead disk
/// 2. Report dead disk to JIRA for repairs
/// 3. Test for resolution
/// 4. Put disk back into cluster
#[macro_use]
extern crate clap;
#[macro_use]
extern crate json;
#[macro_use]
extern crate log;
extern crate simplelog;

mod ceph;
mod create_support_ticket;
mod host_information;
mod in_progress;
mod test_disk;

use std::path::PathBuf;

use clap::{Arg, App};
use simplelog::{Config, SimpleLogger};
use test_disk::run_checks;

fn main() {
    let matches = App::new("Ceph Disk Manager")
        .version(crate_version!())
        .author(crate_authors!())
        .about(
            "Detect dead hard drives, create a support ticket and watch for resolution",
        )
        .arg(
            Arg::with_name("existing_config")
                .default_value("/etc/ceph/ceph.conf")
                .help("Location of ceph.conf file for this cluster")
                .long("existing_config")
                .short("c")
                .takes_value(true),
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
    let _ = SimpleLogger::init(level, Config::default());

    println!("Testing /var/lib/ceph/osd/ceph-72");
    let f = match run_checks(&PathBuf::from("/var/lib/ceph/osd/ceph-72")) {
        Ok(o) => o,
        Err(e) => {
            println!("run_checks failed: {:?}", e);
        }
    };
    host_information::server_serial().unwrap();
}
