extern crate api;
extern crate block_utils;
extern crate bytes;
#[macro_use]
extern crate clap;
extern crate gpt;
extern crate hashicorp_vault;
extern crate helpers;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate simplelog;
extern crate zmq;

mod backend;

use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::path::Path;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use api::service::{Disk, DiskType, Disks, Op, OpBoolResult, OpResult, Partition, PartitionInfo,
                   ResultType};
use backend::BackendType;
use block_utils::{Device, MediaType};
use clap::{App, Arg};
use gpt::header::read_header;
use gpt::partition::read_partitions;
use hashicorp_vault::client::VaultClient;
use protobuf::Message as ProtobufMsg;
use protobuf::RepeatedField;
use protobuf::parse_from_bytes;
use simplelog::{CombinedLogger, Config, TermLogger, WriteLogger};
use zmq::{Message, Socket};
use zmq::Result as ZmqResult;

#[derive(Clone, Debug, Deserialize)]
struct DiskManagerConfig {
    backend: BackendType,
    vault_token: Option<String>,
    vault_endpoint: Option<String>,
}

fn convert_media_to_disk_type(m: MediaType) -> DiskType {
    match m {
        MediaType::Loopback => DiskType::LOOPBACK,
        MediaType::LVM => DiskType::LVM,
        MediaType::MdRaid => DiskType::MDRAID,
        MediaType::NVME => DiskType::NVME,
        MediaType::Ram => DiskType::RAM,
        MediaType::Rotational => DiskType::ROTATIONAL,
        MediaType::SolidState => DiskType::SOLID_STATE,
        MediaType::Unknown => DiskType::UNKNOWN,
        MediaType::Virtual => DiskType::VIRTUAL,
    }
}

fn setup_curve(s: &mut Socket, config_dir: &Path, vault: bool) -> Result<()> {
    // will raise EINVAL if not linked against libsodium
    // The ubuntu package is linked so this shouldn't fail
    s.set_curve_server(true)?;
    let keypair = zmq::CurveKeyPair::new().map_err(|e| Error::new(ErrorKind::Other, e))?;
    if vault {
        //Connect to vault
        let config: DiskManagerConfig =
            helpers::load_config(&config_dir.to_string_lossy(), "disk-manager.json")?;
        if config.vault_token.is_none() || config.vault_endpoint.is_none() {
            error!("Vault support requested but vault_token or vault_endpoint aren't set");
            return Err(Error::new(
                ErrorKind::Other,
                "vault_token or vault_endpoint must be set for vault support".to_string(),
            ));
        }
        let endpoint = config.vault_endpoint.unwrap();
        let token = config.vault_token.unwrap();
        let hostname = {
            let mut f = File::open("/etc/hostname")?;
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            s
        };
        debug!(
            "Connecting to vault to save the public key to /bynar/{}.pem",
            hostname
        );
        let client = VaultClient::new(endpoint.as_str(), token)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        client
            .set_secret(format!("/bynar/{}.pem", hostname), keypair.public_key)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        s.set_curve_secretkey(&keypair.secret_key)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
    } else {
        debug!("Creating new curve keypair");
        s.set_curve_secretkey(&keypair.secret_key)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        let mut f = File::create(format!("{}/ecpubkey.pem", config_dir.display()))?;
        f.write(keypair.public_key.as_bytes())?;
    }
    debug!("Server mechanism: {:?}", s.get_mechanism());
    debug!("Curve server: {:?}", s.is_curve_server());

    Ok(())
}

/*
 Server that manages disks
 */
fn listen(
    backend_type: backend::BackendType,
    config_dir: &Path,
    listen_address: &str,
    vault: bool,
) -> ZmqResult<()> {
    debug!("Starting zmq listener with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let mut responder = context.socket(zmq::REP)?;

    debug!("Listening on tcp://{}:5555", listen_address);
    // Fail to start if this fails
    setup_curve(&mut responder, config_dir, vault).unwrap();
    assert!(
        responder
            .bind(&format!("tcp://{}:5555", listen_address))
            .is_ok()
    );

    loop {
        let msg = responder.recv_bytes(0)?;
        debug!("Got msg len: {}", msg.len());
        trace!("Parsing msg {:?} as hex", msg);
        let operation = match parse_from_bytes::<api::service::Operation>(&msg) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to parse_from_bytes {:?}.  Ignoring request", e);
                continue;
            }
        };

        debug!("Operation requested: {:?}", operation.get_Op_type());
        match operation.get_Op_type() {
            Op::Add => {
                let id = if operation.has_osd_id() {
                    Some(operation.get_osd_id())
                } else {
                    None
                };
                let journal = if operation.has_osd_journal() {
                    Some(operation.get_osd_journal())
                } else {
                    None
                };
                let journal_partition = if operation.has_osd_journal_partition() {
                    Some(operation.get_osd_journal_partition())
                } else {
                    None
                };
                if !operation.has_disk() {
                    error!("Add operation must include disk field.  Ignoring request");
                    continue;
                }
                match add_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    id,
                    journal,
                    journal_partition,
                    config_dir,
                ) {
                    Ok(_) => {
                        info!("Add disk successful");
                    }
                    Err(e) => {
                        error!("Add disk error: {:?}", e);
                    }
                };
            }
            Op::AddPartition => {
                //
            }
            Op::List => {
                match list_disks(&mut responder) {
                    Ok(_) => {
                        info!("List disks successful");
                    }
                    Err(e) => {
                        error!("List disks error: {:?}", e);
                    }
                };
            }
            Op::Remove => {
                if !operation.has_disk() {
                    error!("Remove operation must include disk field.  Ignoring request");
                    continue;
                }
                match remove_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    config_dir,
                ) {
                    Ok(_) => {
                        info!("Remove disk successful");
                    }
                    Err(e) => {
                        error!("Remove disk error: {:?}", e);
                    }
                };
            }
            Op::SafeToRemove => {
                if !operation.has_disk() {
                    error!("SafeToRemove operation must include disk field.  Ignoring request");
                    continue;
                }
                match safe_to_remove_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    config_dir,
                ) {
                    Ok(_) => {
                        info!("Remove disk successful");
                    }
                    Err(e) => {
                        error!("Remove disk error: {:?}", e);
                    }
                };
            }
        };
        thread::sleep(Duration::from_millis(10));
    }
}

fn add_disk(
    s: &mut Socket,
    d: &str,
    backend: &BackendType,
    id: Option<u64>,
    journal: Option<&str>,
    journal_partition: Option<u32>,
    config_dir: &Path,
) -> Result<()> {
    let backend = backend::load_backend(backend, Some(config_dir))
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let mut result = OpResult::new();

    //Send back OpResult
    match backend.add_disk(&Path::new(d), id, journal, journal_partition, false) {
        Ok(_) => {
            result.set_result(ResultType::OK);
        }
        Err(e) => {
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());
        }
    }
    let encoded = result
        .write_to_bytes()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;

    Ok(())
}

fn get_disks() -> Result<Vec<Disk>> {
    let mut disks: Vec<Disk> = Vec::new();
    debug!("Searching for block devices");
    let devices = block_utils::get_block_devices().map_err(|e| Error::new(ErrorKind::Other, e))?;

    debug!("Gathering udev info on block devices");
    // Gather info on all devices and skip Loopback devices
    let device_info: Vec<Device> = block_utils::get_all_device_info(devices.as_slice())
        .map_err(|e| Error::new(ErrorKind::Other, e))?
        .into_iter()
        .collect();
    debug!("Device info found: {:?}", device_info);

    debug!("Gathering partition info");

    for device in device_info {
        let mut d = Disk::new();
        let dev_path = format!("/dev/{}", device.name);
        // This will skip partition_info if it fails to gather.  Blank disks will fail
        let p = get_partition_info(&dev_path).unwrap_or(PartitionInfo::new());
        //Translate block_utils MediaType -> Protobuf DiskType
        d.set_field_type(convert_media_to_disk_type(device.media_type));
        d.set_dev_path(dev_path);
        d.set_partitions(p);
        if let Some(serial) = device.serial_number {
            d.set_serial_number(serial);
        }
        disks.push(d);
    }

    Ok(disks)
}

fn get_partition_info(dev_path: &str) -> Result<PartitionInfo> {
    let mut partition_info = PartitionInfo::new();
    let h = read_header(dev_path)?;
    let partitions = read_partitions(dev_path, &h)?;

    // Transform partitions to protobuf
    let proto_parts: Vec<Partition> = partitions
        .iter()
        .map(|part| {
            let mut p = Partition::new();
            p.set_uuid(part.part_guid.hyphenated().to_string());
            p.set_first_lba(part.first_LBA);
            p.set_last_lba(part.last_LBA);
            p.set_flags(part.flags);
            p.set_name(part.name.clone());
            p
        })
        .collect();
    partition_info.set_partition(RepeatedField::from_vec(proto_parts));
    Ok(partition_info)
}

fn list_disks(s: &mut Socket) -> Result<()> {
    let disk_list: Vec<Disk> = get_disks().map_err(|e| Error::new(ErrorKind::Other, e))?;

    let mut disks = Disks::new();
    disks.set_disk(RepeatedField::from_vec(disk_list));
    debug!("Encoding disk list");
    let encoded = disks
        .write_to_bytes()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}

fn remove_disk(s: &mut Socket, d: &str, backend: &BackendType, config_dir: &Path) -> Result<()> {
    //Returns OpResult
    let backend = backend::load_backend(backend, Some(config_dir))
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let mut result = OpResult::new();
    match backend.remove_disk(&Path::new(d), false) {
        Ok(_) => {
            result.set_result(ResultType::OK);
        }
        Err(e) => {
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());
        }
    };
    let encoded = result
        .write_to_bytes()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}

fn safe_to_remove_disk(
    s: &mut Socket,
    d: &str,
    backend: &BackendType,
    config_dir: &Path,
) -> Result<()> {
    let backend = backend::load_backend(backend, Some(config_dir))
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let mut result = OpBoolResult::new();
    match backend.safe_to_remove(&Path::new(d), false) {
        Ok(val) => {
            result.set_result(ResultType::OK);
            result.set_value(val);
        }
        Err(e) => {
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());
        }
    };
    let encoded = result
        .write_to_bytes()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}

fn main() {
    let matches = App::new("Disk Manager")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Detect dead hard drives, create a support ticket and watch for resolution")
        .arg(
            Arg::with_name("backend")
                .default_value("ceph")
                .help("Backend cluster type to manage disks for")
                .long("backend")
                .possible_values(&["ceph"])
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("listen")
                .default_value("*")
                .help("Address to listen on.  Default is all interfaces")
                .long("listenaddress")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("configdir")
                .default_value("/etc/bynar")
                .help("The directory where all config files can be found")
                .long("configdir")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("log")
                .default_value("/var/log/bynar-disk-manager.log")
                .help("Default log file location")
                .long("logfile")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("vault")
                .default_value("false")
                .help("Enable vault support. Remember to set the vault_token and vault_endpoint")
                .long("vault")
                .possible_values(&["true", "false"])
                .takes_value(true)
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
    let config_dir = Path::new(matches.value_of("configdir").unwrap());
    let backend = BackendType::from_str(matches.value_of("backend").unwrap()).unwrap();
    let vault_support = {
        let b = bool::from_str(matches.value_of("vault").unwrap()).unwrap();
        b
    };
    let _ = CombinedLogger::init(vec![
        TermLogger::new(level, Config::default()).unwrap(),
        WriteLogger::new(
            level,
            Config::default(),
            File::create(matches.value_of("log").unwrap()).unwrap(),
        ),
    ]);
    match listen(
        backend,
        config_dir,
        matches.value_of("listen").unwrap(),
        vault_support,
    ) {
        Ok(_) => {
            println!("Finished");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    };
}
