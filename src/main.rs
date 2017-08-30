
/// Detect dead disks in a ceph cluster
/// 1. Detect dead disk
/// 2. Report dead disk to JIRA for repairs
/// 3. Test for resolution
/// 4. Put disk back into cluster
extern crate clap;
#[macro_use]
extern crate log;
extern crate simplelog;

mod host_information;
mod in_progress;
mod test_disk;

use clap::{Arg, App};
use simplelog::{Config, SimpleLogger};

fn main() {
    let matches = App::new("Ceph migration")
        .version("1.0")
        .author("Chris Holcombe <xfactor973@gmail.com>")
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
        .arg(
            Arg::with_name("new_config")
                .default_value("/tmp/ceph.conf")
                .help("Location of ceph.conf file for the new cluster")
                .long("new_config")
                .required(true)
                .short("n")
                .takes_value(true),
        )
        .arg(Arg::with_name("v").short("v").multiple(true).help(
            "Sets the level of verbosity",
        ))
        .get_matches();
}
