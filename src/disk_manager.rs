#[macro_use]
mod util;
use serde_derive::*;

use std::fs::{create_dir, File};
use std::io::{Error, ErrorKind, Write};
use std::path::Path;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use api::service::{
    Disk, DiskType, Disks, Op, OpBoolResult, OpResult, Partition, PartitionInfo, ResultType,
};

mod backend;
use crate::backend::BackendType;
use crate::util::*;
use block_utils::{Device, MediaType};
use clap::{crate_authors, crate_version, App, Arg};
use gpt::{disk, header::read_header, partition::read_partitions};
use hashicorp_vault::client::VaultClient;
use helpers::error::*;
use hostname::get_hostname;
use log::{debug, error, info, trace, warn};
use protobuf::parse_from_bytes;
use protobuf::Message as ProtobufMsg;
use protobuf::RepeatedField;
use simplelog::{CombinedLogger, Config, SharedLogger, TermLogger, WriteLogger};
use zmq::{Message, Socket};

#[derive(Clone, Debug, Deserialize)]
struct DiskManagerConfig {
    backend: BackendType,
    vault_token: Option<String>,
    vault_endpoint: Option<String>,
}

fn convert_media_to_disk_type(m: &MediaType) -> DiskType {
    match *m {
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

fn setup_curve(s: &mut Socket, config_dir: &Path, vault: bool) -> BynarResult<()> {
    // will raise EINVAL if not linked against libsodium
    // The ubuntu package is linked so this shouldn't fail
    s.set_curve_server(true)?;
    let keypair = zmq::CurveKeyPair::new()?;
    let hostname =
        get_hostname().ok_or_else(|| Error::new(ErrorKind::Other, "hostname not found"))?;
    let key_file = config_dir.join(format!("{}.pem", hostname));
    if vault {
        //Connect to vault
        let config: DiskManagerConfig = helpers::load_config(&config_dir, "disk-manager.json")?;
        if config.vault_token.is_none() || config.vault_endpoint.is_none() {
            error!("Vault support requested but vault_token or vault_endpoint aren't set");
            return Err(BynarError::new(
                "vault_token or vault_endpoint must be set for vault support".to_string(),
            ));
        }
        let endpoint = config.vault_endpoint.unwrap();
        let token = config.vault_token.unwrap();
        debug!(
            "Connecting to vault to save the public key to /bynar/{}.pem",
            hostname
        );
        let client = VaultClient::new(endpoint.as_str(), token)?;
        client.set_secret(
            format!("{}/{}.pem", config_dir.display(), hostname),
            keypair.public_key,
        )?;
        s.set_curve_secretkey(&keypair.secret_key)?;
    } else {
        debug!("Creating new curve keypair");
        s.set_curve_secretkey(&keypair.secret_key)?;
        let mut f = File::create(key_file)?;
        f.write_all(keypair.public_key.as_bytes())?;
    }
    debug!("Server mechanism: {:?}", s.get_mechanism());
    debug!("Curve server: {:?}", s.is_curve_server());

    Ok(())
}

/*
Server that manages disks
*/
fn listen(
    backend_type: &backend::BackendType,
    config_dir: &Path,
    listen_address: &str,
    vault: bool,
) -> BynarResult<()> {
    debug!("Starting zmq listener with version({:?})", zmq::version());
    let context = zmq::Context::new();
    let mut responder = context.socket(zmq::REP)?;

    debug!("Listening on tcp://{}:5555", listen_address);
    // Fail to start if this fails
    setup_curve(&mut responder, config_dir, vault)?;
    assert!(responder
        .bind(&format!("tcp://{}:5555", listen_address))
        .is_ok());

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
                if !operation.has_disk() {
                    error!("Add operation must include disk field.  Ignoring request");
                    // We still have to respond with an error message
                    let mut result = OpResult::new();
                    result.set_result(ResultType::ERR);
                    result.set_error_msg(
                        "missing operation field in protocol. Ignoring request".to_string(),
                    );

                    let _ = respond_to_client(&result, &mut responder);
                    continue;
                }
                 nout_match!( add_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    id,
                    config_dir,
                ), "Add disk finished", "Add disk error: {:?}");
                /*match add_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    id,
                    config_dir,
                ) {
                    Ok(_) => {
                        info!("Add disk finished");
                    }
                    Err(e) => {
                        error!("Add disk error: {:?}", e);
                    }
                };*/
            }
            Op::AddPartition => {
                //
            }
            Op::List => {
                nout_match!(list_disks(&mut responder) , "List disks finished", "List disks error: {:?}");
                /*match list_disks(&mut responder) {
                    Ok(_) => {
                        info!("List disks finished");
                    }
                    Err(e) => {
                        error!("List disks error: {:?}", e);
                    }
                };*/
            }
            Op::Remove => {
                if !operation.has_disk() {
                    error!("Remove operation must include disk field.  Ignoring request");
                    continue;
                }
                let mut result = OpResult::new();
                match safe_to_remove(&Path::new(operation.get_disk()), &backend_type, config_dir) {
                    Ok(true) => {
                        nout_match!( remove_disk(
                            &mut responder,
                            operation.get_disk(),
                            &backend_type,
                            config_dir,
                        ) , "Remove disk finished", "Remove disk error: {:?}");
                        /*match remove_disk(
                            &mut responder,
                            operation.get_disk(),
                            &backend_type,
                            config_dir,
                        ) {
                            Ok(_) => {
                                info!("Remove disk finished");
                            }
                            Err(e) => {
                                error!("Remove disk error: {:?}", e);
                            }
                        };*/
                    }
                    Ok(false) => {
                        debug!("Disk is not safe to remove");
                        //Response to client
                        result.set_result(ResultType::ERR);
                        result.set_error_msg("Not safe to remove disk".to_string());
                        let _ = respond_to_client(&result, &mut responder);
                    }
                    Err(e) => {
                        error!("safe to remove failed: {:?}", e);
                        // Response to client
                        result.set_result(ResultType::ERR);
                        result.set_error_msg(e.to_string());
                        let _ = respond_to_client(&result, &mut responder);
                    }
                };
            }
            Op::SafeToRemove => {
                if !operation.has_disk() {
                    error!("SafeToRemove operation must include disk field.  Ignoring request");
                    continue;
                }
                nout_match!( safe_to_remove_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    config_dir,
                ), "Safe to remove disk finished", "Safe to remove error: {:?}");
                /*match safe_to_remove_disk(
                    &mut responder,
                    operation.get_disk(),
                    &backend_type,
                    config_dir,
                ) {
                    Ok(_) => {
                        info!("Safe to remove disk finished");
                    }
                    Err(e) => {
                        error!("Safe to remove error: {:?}", e);
                    }
                };*/
            }
        };
        thread::sleep(Duration::from_millis(10));
    }
}

fn respond_to_client(result: &OpResult, s: &mut Socket) -> BynarResult<()> {
    let encoded = result.write_to_bytes()?;
    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}

fn add_disk(
    s: &mut Socket,
    d: &str,
    backend: &BackendType,
    id: Option<u64>,
    config_dir: &Path,
) -> BynarResult<()> {
    let mut result = OpResult::new();
    let backend = match backend::load_backend(backend, Some(config_dir)) {
        Ok(backend) => backend,
        Err(e) => {
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());

            // Bail early.  We can't load the backend
            let _ = respond_to_client(&result, s);
            return Ok(());
        }
    };

    //Send back OpResult
    match backend.add_disk(&Path::new(d), id, false) {
        Ok(_) => {
            result.set_result(ResultType::OK);
        }
        Err(e) => {
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());
        }
    };
    let _ = respond_to_client(&result, s);

    Ok(())
}

fn get_disks() -> BynarResult<Vec<Disk>> {
    let mut disks: Vec<Disk> = Vec::new();
    debug!("Searching for block devices");
    let devices = block_utils::get_block_devices()?;

    debug!("Gathering udev info on block devices");
    // Gather info on all devices and skip Loopback devices
    let device_info: Vec<Device> = block_utils::get_all_device_info(devices.as_slice())?
        .into_iter()
        .collect();
    debug!("Device info found: {:?}", device_info);

    debug!("Gathering partition info");

    for device in device_info {
        let mut d = Disk::new();
        let dev_path = Path::new("/dev/").join(device.name);
        // This will skip partition_info if it fails to gather.  Blank disks will fail
        let p = get_partition_info(&dev_path).unwrap_or_else(|_| PartitionInfo::new());
        //Translate block_utils MediaType -> Protobuf DiskType
        d.set_field_type(convert_media_to_disk_type(&device.media_type));
        d.set_dev_path(dev_path.to_string_lossy().into_owned());
        d.set_partitions(p);
        if let Some(serial) = device.serial_number {
            d.set_serial_number(serial);
        }
        disks.push(d);
    }

    Ok(disks)
}

fn get_partition_info(dev_path: &Path) -> BynarResult<PartitionInfo> {
    let mut partition_info = PartitionInfo::new();
    let h = read_header(dev_path, disk::DEFAULT_SECTOR_SIZE)?;
    let partitions = read_partitions(dev_path, &h, disk::DEFAULT_SECTOR_SIZE)?;

    // Transform partitions to protobuf
    let proto_parts: Vec<Partition> = partitions
        .iter()
        .map(|part| {
            let mut p = Partition::new();
            p.set_uuid(part.1.part_guid.to_hyphenated().to_string());
            p.set_first_lba(part.1.first_lba);
            p.set_last_lba(part.1.last_lba);
            p.set_flags(part.1.flags);
            p.set_name(part.1.name.clone());
            p
        })
        .collect();
    partition_info.set_partition(RepeatedField::from_vec(proto_parts));
    Ok(partition_info)
}

fn list_disks(s: &mut Socket) -> BynarResult<()> {
    let disk_list: Vec<Disk> = get_disks()?;

    let mut disks = Disks::new();
    disks.set_disk(RepeatedField::from_vec(disk_list));
    debug!("Encoding disk list");
    let encoded = disks.write_to_bytes()?;

    let msg = Message::from_slice(&encoded)?;
    debug!("Responding to client with msg len: {}", msg.len());
    s.send_msg(msg, 0)?;
    Ok(())
}

fn remove_disk(
    s: &mut Socket,
    d: &str,
    backend: &BackendType,
    config_dir: &Path,
) -> BynarResult<()> {
    //Returns OpResult
    let mut result = OpResult::new();
    let backend = match backend::load_backend(backend, Some(config_dir)) {
        Ok(b) => b,
        Err(e) => {
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());

            // Bail early.  We can't load the backend
            let _ = respond_to_client(&result, s);
            return Ok(());
        }
    };
    match backend.remove_disk(&Path::new(d), false) {
        Ok(_) => {
            result.set_result(ResultType::OK);
        }
        Err(e) => {
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());
        }
    };
    let _ = respond_to_client(&result, s);
    Ok(())
}

fn safe_to_remove(d: &Path, backend: &BackendType, config_dir: &Path) -> BynarResult<bool> {
    let backend = backend::load_backend(backend, Some(config_dir))?;
    let safe = backend.safe_to_remove(d, false)?;

    Ok(safe)
}

fn safe_to_remove_disk(
    s: &mut Socket,
    d: &str,
    backend: &BackendType,
    config_dir: &Path,
) -> BynarResult<()> {
    debug!("Checking if {} is safe to remove", d);
    let mut result = OpBoolResult::new();
    match safe_to_remove(&Path::new(d), &backend, &config_dir) {
        Ok(val) => {
            debug!("Safe to remove: {}", val);
            result.set_result(ResultType::OK);
            result.set_value(val);
        }
        Err(e) => {
            debug!("Safe to remove err: {}", e);
            result.set_result(ResultType::ERR);
            result.set_error_msg(e.to_string());
            let encoded = result.write_to_bytes()?;
            let msg = Message::from_slice(&encoded)?;
            debug!("Responding to client with msg len: {}", msg.len());
            s.send_msg(msg, 0)?;
            return Err(BynarError::new(format!("safe to remove error: {}", e)));
        }
    };
    let encoded = result.write_to_bytes()?;
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
    info!("Starting up");

    //Sanity check
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
    let log = Path::new(matches.value_of("log").unwrap());
    let backend = BackendType::from_str(matches.value_of("backend").unwrap())
        .expect("unable to convert backend option to BackendType");
    let vault_support = {
        bool::from_str(matches.value_of("vault").unwrap())
            .expect("unable to convert vault option to bool")
    };
    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![];
    if let Some(term_logger) = TermLogger::new(level, Config::default()) {
        //systemd doesn't use a terminal
        loggers.push(term_logger);
    }
    loggers.push(WriteLogger::new(
        level,
        Config::default(),
        File::create(log).expect("log file creation failed"),
    ));
    let _ = CombinedLogger::init(loggers);
    match listen(
        &backend,
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
