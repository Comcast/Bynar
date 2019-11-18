//! Disk checks are defined here.  To define a new check create a new
//! struct and then impl Transition for it.  The disks here use a state
//! machine to determine what is and is not possible.  To see the state
//! machine as a visual diagram run one of the unit tests and copy the
//! digraph output into a dot file and convert using
//! `dot -Tps example.dot -o example.ps` to postscript or
//! `dot -Tsvg example.dot -o example.svg` to svg.
//! See comments on the run() function for StateMachine and also
//! the comments under setup_state_machine() to learn more about how it works.
#[cfg(test)]
use mocktopus::*;

use crate::in_progress::{
    add_disk_detail, add_or_update_operation, get_devices_from_db, get_state,
    is_hardware_waiting_repair, save_state, HostDetailsMapping, OperationInfo,
};
use blkid::BlkId;
use block_utils::{
    format_block_device, get_device_info, mount_device, unmount_device, Device, DeviceState,DeviceType,
    Filesystem, FilesystemType, MediaType, ScsiDeviceType, ScsiInfo, Vendor,
};
use gpt::{disk, header::read_header, partition::read_partitions, partition::Partition};
use helpers::{error::*, host_information::Host};
use log::{debug, error, trace, warn};
use lvm::*;
#[cfg(test)]
use mocktopus::macros::*;
use petgraph::graphmap::GraphMap;
use petgraph::Directed;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager as ConnectionManager;
use std::collections::{BTreeMap, HashSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::FromStr;
use tempdir::TempDir;
use uuid::Uuid;

// Function pointer to the transition function
type TransitionFn =
    fn(State, &mut BlockDevice, &Option<(ScsiInfo, Option<ScsiInfo>)>, bool) -> State;

#[derive(Clone, Debug)]
pub struct BlockDevice {
    pub device: Device,
    pub dev_path: PathBuf,
    // None means disk is not in the database
    pub device_database_id: Option<u32>,
    pub mount_point: Option<PathBuf>,
    pub partitions: BTreeMap<u32, Partition>,
    pub scsi_info: ScsiInfo,
    pub state: State,
    pub storage_detail_id: u32,
    pub operation_id: Option<u32>,
}

impl BlockDevice {
    pub fn set_device_database_id(&mut self, device_database_id: u32) {
        self.device_database_id = Some(device_database_id);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Mutex;

    use blkid::BlkId;
    use lazy_static::lazy_static;
    use log::debug;
    use mocktopus::mocking::*;
    use simplelog::{Config, TermLogger};
    use tempdir::TempDir;
    use uuid::Uuid;

    lazy_static! {
        // This prevents all threads from getting the same loopback device
        static ref LOOP: Mutex<()> = Mutex::new(());
    }

    fn create_loop_device() -> PathBuf {
        let _shared = LOOP.lock().unwrap();
        // Find free loopback device
        let out = Command::new("losetup").args(&["-f"]).output().unwrap();
        // Assert we created the device
        assert_eq!(out.status.success(), true);

        let stdout = String::from_utf8_lossy(&out.stdout);
        let free_device = stdout.trim();

        // Create a loopback device for testing
        let random = rand::random::<u32>();
        let d = TempDir::new(&format!("bynar.{}", random)).expect("Temp dir creation failed");
        let file_path = d.path().join("file.img");

        let mut f = File::create(&file_path).expect("loop backing file creation failed");
        // Write 25MB to a file
        debug!("writing 25MB to {}", file_path.display());
        let buff = [0x00; 1024];
        for _ in 0..25600 {
            f.write(&buff)
                .expect("Failed to write to loop backing file");
        }
        f.sync_all().unwrap();

        // Setup a loopback device for testing filesystem corruption
        debug!("setting up {} device", free_device);
        Command::new("losetup")
            .args(&[free_device, &file_path.to_string_lossy()])
            .status()
            .unwrap();

        // Put an xfs filesystem down on it
        debug!("Putting xfs on to {}", free_device);
        Command::new("mkfs.xfs")
            .args(&[free_device])
            .status()
            .unwrap();

        PathBuf::from(free_device)
    }

    fn cleanup_loop_device(p: &Path) {
        // Cleanup
        Command::new("umount")
            .args(&[&p.to_string_lossy().into_owned()])
            .status()
            .unwrap();

        Command::new("losetup")
            .args(&["-d", &p.to_string_lossy()])
            .status()
            .unwrap();
    }

    #[test]
    fn test_state_machine_base() {
        TermLogger::new(log::LevelFilter::Debug, Config::default()).unwrap();

        // Mock smart to return Ok(true)
        super::run_smart_checks.mock_safe(|_| MockResult::Return(Ok(true)));

        let dev = create_loop_device();

        let blkid = BlkId::new(&dev).unwrap();
        blkid.do_probe().unwrap();
        let drive_uuid = blkid.lookup_value("UUID").unwrap();
        debug!("drive_uuid: {}", drive_uuid);

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();

        let d = super::BlockDevice {
            device: super::Device {
                id: Some(drive_id),
                name: dev.file_name().unwrap().to_str().unwrap().to_string(),
                media_type: super::MediaType::Rotational,
                capacity: 26214400,
                fs_type: super::FilesystemType::Xfs,
                serial_number: Some("123456".into()),
            },
            dev_path: PathBuf::from(""),
            device_database_id: None,
            mount_point: None,
            partitions: BTreeMap::new(),
            scsi_info: super::ScsiInfo::default(),
            state: super::State::Unscanned,
            storage_detail_id: 1,
            operation_id: None,
        };
        let mut s = super::StateMachine::new(d, None, true);
        s.setup_state_machine();
        s.print_graph();
        s.run();
        println!("final state: {}", s.block_device.state);
        cleanup_loop_device(&dev);
        assert_eq!(s.block_device.state, super::State::Good);
    }

    #[test]
    fn test_state_machine_bad_filesystem() {
        TermLogger::new(log::LevelFilter::Debug, Config::default()).unwrap();

        // Mock smart to return Ok(true)
        super::run_smart_checks.mock_safe(|_| MockResult::Return(Ok(true)));

        let dev = create_loop_device();
        let blkid = BlkId::new(&dev).unwrap();
        blkid.do_probe().unwrap();
        let drive_uuid = blkid.lookup_value("UUID").unwrap();
        debug!("drive_uuid: {}", drive_uuid);

        debug!("Corrupting the filesystem");
        // This is repairable by xfs_repair
        Command::new("xfs_db")
            .args(&[
                "-x",
                "-c",
                "blockget",
                "-c",
                "blocktrash",
                &dev.to_string_lossy().into_owned(),
            ])
            .status()
            .unwrap();

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();
        let d = super::BlockDevice {
            device: super::Device {
                id: Some(drive_id),
                name: dev.file_name().unwrap().to_str().unwrap().to_string(),
                media_type: super::MediaType::Rotational,
                capacity: 26214400,
                fs_type: super::FilesystemType::Xfs,
                serial_number: Some("123456".into()),
            },
            dev_path: PathBuf::from(""),
            device_database_id: None,
            mount_point: None,
            partitions: BTreeMap::new(),
            scsi_info: super::ScsiInfo::default(),
            state: super::State::Unscanned,
            storage_detail_id: 1,
            operation_id: None,
        };
        let mut s = super::StateMachine::new(d, None, true);
        s.setup_state_machine();
        s.print_graph();
        s.run();
        println!("final state: {}", s.block_device.state);

        cleanup_loop_device(&dev);
        assert_eq!(s.block_device.state, super::State::Good);
    }

    #[test]
    fn test_state_machine_replace_disk() {
        use helpers::error::*;
        // Smart passes, write fails,  check_filesystem fails, attemptRepair and reformat fails
        TermLogger::new(log::LevelFilter::Debug, Config::default()).unwrap();

        super::run_smart_checks.mock_safe(|_| MockResult::Return(Ok(true)));
        super::check_writable
            .mock_safe(|_| MockResult::Return(Err(BynarError::from("Mock Error"))));
        super::check_filesystem.mock_safe(|_, _| MockResult::Return(Ok(super::Fsck::Corrupt)));
        super::repair_filesystem
            .mock_safe(|_, _| MockResult::Return(Err(BynarError::from("Mock Error"))));

        // TODO: Can't mock outside dependencies.  Need a wrapper function or something
        super::format_device.mock_safe(|_| MockResult::Return(Err(BynarError::from("error"))));
        // That should leave the disk in WaitingForReplacement

        let dev = create_loop_device();

        let blkid = BlkId::new(&dev).unwrap();
        blkid.do_probe().unwrap();
        let drive_uuid = blkid.lookup_value("UUID").unwrap();
        debug!("drive_uuid: {}", drive_uuid);

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();

        let d = super::BlockDevice {
            device: super::Device {
                id: Some(drive_id),
                name: dev.file_name().unwrap().to_str().unwrap().to_string(),
                media_type: super::MediaType::Rotational,
                capacity: 26214400,
                fs_type: super::FilesystemType::Xfs,
                serial_number: Some("123456".into()),
            },
            dev_path: PathBuf::from(""),
            device_database_id: None,
            mount_point: None,
            partitions: BTreeMap::new(),
            scsi_info: super::ScsiInfo::default(),
            state: super::State::Unscanned,
            storage_detail_id: 1,
            operation_id: None,
        };
        let mut s = super::StateMachine::new(d, None, false);
        s.setup_state_machine();
        s.print_graph();
        s.run();
        println!("final state: {}", s.block_device.state);

        cleanup_loop_device(&dev);

        assert_eq!(s.block_device.state, super::State::WaitingForReplacement);
    }

    #[test]
    fn test_state_machine_replaced_disk() {
        TermLogger::new(log::LevelFilter::Debug, Config::default()).unwrap();
        super::run_smart_checks.mock_safe(|_| MockResult::Return(Ok(true)));

        let dev = create_loop_device();

        let blkid = BlkId::new(&dev).unwrap();
        blkid.do_probe().unwrap();
        let drive_uuid = blkid.lookup_value("UUID").unwrap();
        debug!("drive_uuid: {}", drive_uuid);

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();

        // Set the previous state to something other than Unscanned

        let d = super::BlockDevice {
            device: super::Device {
                id: Some(drive_id),
                name: dev.file_name().unwrap().to_str().unwrap().to_string(),
                media_type: super::MediaType::Rotational,
                capacity: 26214400,
                fs_type: super::FilesystemType::Xfs,
                serial_number: Some("123456".into()),
            },
            dev_path: PathBuf::from(""),
            device_database_id: None,
            mount_point: None,
            partitions: BTreeMap::new(),
            scsi_info: super::ScsiInfo::default(),
            state: super::State::Replaced,
            storage_detail_id: 1,
            operation_id: None,
        };
        // restore state?
        let mut s = super::StateMachine::new(d, None, true);
        s.setup_state_machine();
        s.print_graph();
        s.run();
        println!("final state: {}", s.block_device.state);
        assert_eq!(s.block_device.state, super::State::Good);
    }
}

trait Transition {
    // Transition from the current state to an ending state given an Event
    // database connection can be used to save and resume state
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        simulate: bool, // Pretend to transition and skip any side effects
    ) -> State;
}

impl Transition for AttemptRepair {
    // Take a Corrupt
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        simulate: bool,
    ) -> State {
        debug!("thread {} running AttemptRepair transition", process::id());
        // Disk filesystem is corrupted.  Attempt repairs.
        if !simulate {
            // keep ref to mountpoint.  check if filesystem unmounted (if not unmount first)
            // After running repair remount filesystem if unmounted
            if let Some(ref mnt) = device.mount_point {
                debug!("Attempt to unmount filesystem for repair");
                if let Err(e) = unmount_device(&mnt) {
                    error!("unmount {} failed: {}", mnt.display(), e);
                };
            }
            match repair_filesystem(&device.device.fs_type, &device.dev_path) {
                Ok(_) => {
                    // This requires root perms.  If the filesystem was previously mounted remount the filesystem
                    if let Some(ref mnt) = device.mount_point {
                        if let Err(e) = mount_device(&device.device, &mnt) {
                            error!("Remounting {} failed: {}", device.dev_path.display(), e);
                        }
                    }
                    to_state
                },
                Err(e) => {
                    error!("repair_filesystem failed on {:?}: {}", device, e);
                    // This requires root perms.  If the filesystem was previously mounted remount the filesystem
                    if let Some(ref mnt) = device.mount_point {
                        if !is_device_mounted(&device.dev_path){
                            // attempted to remount the filesystem
                            if let Err(e) = mount_device(&device.device, &mnt) {
                                error!("Remounting {} failed: {}", device.dev_path.display(), e);
                            }
                        }
                    }
                    State::Fail
                }
            }
        } else {
            to_state
        }
    }
}

impl Transition for CheckForCorruption {
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        simulate: bool,
    ) -> State {
        debug!(
            "thread {} running CheckForCorruption transition",
            process::id()
        );
        if !simulate {
            match check_filesystem(&device.device.fs_type, &device.dev_path) {
                Ok(fsck) => match fsck {
                    // Writes are failing but fsck is ok?
                    // What else could be wrong?  The filesystem could be read only
                    // or ??
                    Fsck::Ok => State::Fail,
                    // The filesystem is corrupted.  Proceed to repair
                    Fsck::Corrupt => to_state,
                },
                Err(e) => {
                    error!("check_filesystem failed on {:?}: {}", device, e);
                    State::Fail
                }
            }
        } else {
            to_state
        }
    }
}

impl Transition for CheckReadOnly {
    fn transition(
        _to_state: State,
        _device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!("thread {} running CheckReadOnly transition", process::id());
        // Try again
        State::Fail
    }
}

impl Transition for CheckWearLeveling {
    fn transition(
        to_state: State,
        _device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!(
            "thread {} running CheckWearLeveling transition",
            process::id()
        );

        //TODO: How can we check wear leveling?
        to_state
    }
}

// Evaluate whether a scanned drive is good
impl Transition for Eval {
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!("thread {} running Eval transition", process::id());
        let blank = match is_disk_blank(&device.dev_path) {
            Ok(b) => b,
            Err(e) => {
                error!("Checking if disk is blank failed: {:?}", e);
                // What is the appropriate thing to conclude here?
                false
            }
        };
        debug!(
            "thread {} {} blank {}",
            process::id(),
            device.dev_path.display(),
            blank
        );
        if blank {
            debug!("thread {} Assuming blank disk is good", process::id());
            return to_state;
        }
        debug!("thread {} device: {:?}", process::id(), device);
        if device.device.fs_type == FilesystemType::Lvm {
            debug!("Checking LVM");
            match check_lvm(&device.dev_path) {
                Ok(_) => {
                    debug!("Return state {:?}", to_state);
                    return to_state;
                }
                Err(e) => {
                    error!("check_lvm failed: {:?}", e);
                    return State::Fail;
                }
            };
        }

        if device.mount_point.is_none() {
            debug!("Try mounting in EVAL");
            debug!(
                "thread {} Mounting device: {}",
                process::id(),
                device.dev_path.display()
            );
            let mnt_dir = match TempDir::new("bynar") {
                Ok(d) => d,
                Err(e) => {
                    error!("temp dir creation failed: {:?}", e);
                    return State::Fail;
                }
            };
            // This requires root perms
            if let Err(e) = mount_device(&device.device, &mnt_dir.path()) {
                error!("Mounting {} failed: {}", device.dev_path.display(), e);
                return State::MountFailed;
            }
            device.mount_point = Some(mnt_dir.into_path());
        }
        debug!("thread {} Checking if mount is writable", process::id());
        let mnt = &device
            .mount_point
            .as_ref()
            .expect("drive.mount_point is None but it cannot be");
        match check_writable(&mnt) {
            // Mount point is writeable, smart passed.  Good to go
            Ok(_) => {
                // clean up the mount we used
                if let Err(e) = unmount_device(&mnt) {
                    error!("unmount {} failed: {}", mnt.display(), e);
                };
                device.mount_point = None;
                to_state
            }
            Err(e) => {
                //Should proceed to error checking now
                error!("Error writing to disk: {:?}", e);
                State::WriteFailed
            }
        }
    }
}

impl Transition for MarkForReplacement {
    fn transition(
        to_state: State,
        _device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!(
            "thread {} running MarkForReplacement transition",
            process::id()
        );
        to_state
    }
}

impl Transition for Mount {
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!(
            "thread {} Mounting device: {}",
            process::id(),
            device.dev_path.display()
        );
        let mnt_dir = match TempDir::new("bynar") {
            Ok(d) => d,
            Err(e) => {
                error!("temp dir creation failed: {:?}", e);
                return State::Fail;
            }
        };
        if let Err(e) = mount_device(&device.device, &mnt_dir.path()) {
            error!("Mounting {} failed: {}", device.dev_path.display(), e);
            return State::Fail;
        }

        to_state
    }
}

impl Transition for NoOp {
    fn transition(
        to_state: State,
        _device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!("thread {} running NoOp transition", process::id());

        to_state
    }
}

impl Transition for Reformat {
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!("thread {} running Reformat transition", process::id());
        // Ensure we're not mounted before this it run
        if let Some(ref mnt) = device.mount_point {
            if let Err(e) = unmount_device(&mnt) {
                error!("unmount failed: {}", e);
            }
        }
        match format_device(&device.device) {
            Ok(_) => {
                // We need to update the UUID of the block device now.
                let blkid = BlkId::new(&device.dev_path).expect("blkid creation failed");
                blkid.do_probe().expect("blkid probe failed");
                let drive_uuid = blkid
                    .lookup_value("UUID")
                    .expect("blkid lookup uuid failed");
                debug!(
                    "thread {} drive_uuid: {}",
                    process::id(),
                    Uuid::parse_str(&drive_uuid)
                        .unwrap_or_else(|_| panic!("Invalid drive_uuid: {}", drive_uuid))
                );
                device.device.id = Some(
                    Uuid::parse_str(&drive_uuid)
                        .unwrap_or_else(|_| panic!("Invalid drive_uuid: {}", drive_uuid)),
                );

                to_state
            }
            Err(e) => {
                error!("Reformat failed: {}", e);
                State::Fail
            }
        }
    }
}

impl Transition for Remount {
    fn transition(
        to_state: State,
        _device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!("thread {} running Remount transition", process::id());
        // TODO: Investigate using libmount here
        match Command::new("mount").args(&["-o", "remount"]).output() {
            Ok(output) => {
                if output.status.success() {
                    to_state
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Remount failed: {}", stderr);
                    State::Fail
                }
            }
            Err(e) => {
                error!("Remount failed: {}", e);
                State::Fail
            }
        }
    }
}

impl Transition for Replace {
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        _scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!("thread {} running Replace transition", process::id());
        // So we know at this point that the disk has been replaced
        // We know the device we're working with
        // If it's being a raid device do we need to do anything there?

        // Check if the device has been replaced and the host can see it
        match get_device_info(&device.dev_path) {
            Ok(_) => {
                // Device seems to be present
                to_state
            }
            Err(e) => {
                error!(
                    "Unable to find device: {}. {:?}",
                    device.dev_path.display(),
                    e
                );
                State::Fail
            }
        }
    }
}

impl Transition for Scan {
    fn transition(
        to_state: State,
        device: &mut BlockDevice,
        scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>,
        _simulate: bool,
    ) -> State {
        debug!("thread {} running Scan transition", process::id());
        let raid_backed = is_raid_backed(&scsi_info);
        match (raid_backed.0, raid_backed.1) {
            (false, _) => match run_smart_checks(&Path::new(&device.dev_path)) {
                Ok(stat) => {
                    // If the device is a Disk, then end the state machine here. 
                    if device.device.device_type == DeviceType::Disk {
                        if stat {
                            debug!("Disk is healthy");
                            return State::Good;
                        }
                        else {
                            debug!("Disk Health Scan Failed");
                            return State::Fail;
                        }
                    }
                    to_state
                },
                Err(e) => {
                    error!("Smart test failed: {:?}", e);
                    State::Fail
                }
            },
            (_, Vendor::Hp) => {
                // is_raid_backed unpacks the Option so this should be safe
                match &scsi_info
                    .as_ref()
                    .expect("scsi_info is None but cannot be")
                    .0
                    .state
                {
                    Some(state) => {
                        debug!("thread {} scsi device state: {}", process::id(), state);
                        if *state == DeviceState::Running {
                            // If the device is a Disk, then end the state machine here. 
                            if device.device.device_type == DeviceType::Disk {
                                debug!("Disk is Healthy");
                                return State::Good;
                            }
                            to_state
                        } else {
                            State::Fail
                        }
                    }
                    None => {
                        // What can we conclude??
                        State::Fail
                    }
                }
            }
            (_, v) => {
                // Don't know how to deal with these yet
                warn!("Skipping {:?} raid backed disk scanning", v);
                to_state
            }
        }
    }
}

pub struct StateMachine {
    // A record of the transitions so they can be written as a dot graph
    // for later visual debugging
    dot_graph: Vec<(State, State, String)>,
    // Mapping of valid From -> To transitions
    graph: GraphMap<State, TransitionFn, Directed>,
    pub block_device: BlockDevice,
    // optional info of this device and optional scsi host information
    // used to determine whether this device is behind a raid controller
    pub scsi_info: Option<(ScsiInfo, Option<ScsiInfo>)>,
    simulate: bool,
}

impl fmt::Debug for StateMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.block_device)
    }
}

impl StateMachine {
    fn new(
        block_device: BlockDevice,
        scsi_info: Option<(ScsiInfo, Option<ScsiInfo>)>,
        simulate: bool,
    ) -> Self {
        StateMachine {
            dot_graph: Vec::new(),
            graph: GraphMap::new(),
            block_device,
            scsi_info,
            simulate,
        }
    }

    fn add_transition(
        &mut self,
        from_state: State,
        to_state: State,
        callback: TransitionFn,
        // Just for debugging dot graph creation
        transition_label: &str,
    ) {
        self.dot_graph
            .push((from_state, to_state, transition_label.to_string()));
        self.graph.add_edge(from_state, to_state, callback);
    }

    // Run all transitions until we can't go any further and return
    fn run(&mut self) {
        // Start at the current state the disk is at and work our way down the graph
        debug!(
            "thread {} {} starting state: {}",
            process::id(),
            self.block_device.dev_path.display(),
            self.block_device.state
        );
        if self.block_device.state == State::Good{
            debug!("Starting state is Good, replacing with Unscanned");
            self.block_device.state = State::Unscanned;
        }
        'outer: loop {
            // Gather all the possible edges from this current State
            let edges: Vec<(State, State, &TransitionFn)> =
                self.graph.edges(self.block_device.state).collect();
            // Some states have multiple paths they could go down.
            // If the state transition returns State::Fail try the next path
            let beginning_state = self.block_device.state;
            for e in edges {
                debug!(
                    "thread {} Attempting {} to {} transition",
                    process::id(),
                    &e.0,
                    &e.1
                );
                let state = e.2(e.1, &mut self.block_device, &self.scsi_info, self.simulate);
                match state {
                    State::Fail => {
                        debug!(
                            "thread {} Transition failed. Trying next transition",
                            process::id()
                        );
                        continue;
                    }
                    State::WaitingForReplacement => {
                        // TODO: Is this the only state we shouldn't advance further from?
                        debug!(
                            "thread {} state==State::WaitingForReplacement",
                            process::id()
                        );
                        self.block_device.state = state;
                        break 'outer;
                    }
                    State::Good => {
                        debug!("thread {} state==State::Good", process::id());
                        self.block_device.state = state;
                        break 'outer;
                    }
                    _ => {}
                }
                // transition succeeded.  Save state and go around the loop again
                // This won't detect if the transitions return something unexpected
                if state == e.1 {
                    debug!("thread {} state==e.1 {}=={}", process::id(), state, e.1);
                    self.block_device.state = state;
                    break;
                }
            }
            // At this point we should've advanced further.  If not then we're stuck in an infinite loop
            // Note this won't detect more complicated infinite loops where it changes through 2 states
            // before ending back around again.
            if self.block_device.state == beginning_state {
                // We're stuck in an infinite loop we can't advance further from
                debug!(
                    "thread {} Breaking loop: stuck in same state {}=={}",
                    process::id(),
                    self.block_device.state,
                    beginning_state
                );
                break 'outer;
            }
        }
    }

    #[allow(dead_code)]
    fn print_graph(&self) {
        // FIXME: Too simple.  Doesn't label the transitions
        // Walk the graph and create a Dot
        let mut states = HashSet::new();
        println!("digraph state_machine{{");
        for n in &self.dot_graph {
            states.insert(n.0);
            states.insert(n.1);
            println!("\t{:?} -> {:?}[label=\"{}\"];", n.0, n.1, n.2);
        }
        for n in states {
            println!("\t{:?}[label=\"{:?}\"];", n, n);
        }
        //for edge in self.graph.all_edges() {
        //println!("\t{:?} -> {:?}[label=\"\"];", edge.0, edge.1);
        //}
        println!("}}");
    }

    // Add all the transition states here
    fn setup_state_machine(&mut self) {
        // GraphMap will run the transitions in the order they're added here
        // If Unscanned has 2 edges it will run the first added one first
        // and then the second one.  To deal with this the
        // states are ordered from most to least ideal outcome.
        self.add_transition(State::Unscanned, State::Scanned, Scan::transition, "Scan");
        self.add_transition(State::Unscanned, State::Fail, Scan::transition, "Scan");
        self.add_transition(
            State::NotMounted,
            State::Mounted,
            Mount::transition,
            "Mount",
        );
        self.add_transition(
            State::NotMounted,
            State::MountFailed,
            Mount::transition,
            "Mount",
        );
        self.add_transition(
            State::MountFailed,
            State::Corrupt,
            CheckForCorruption::transition,
            "CheckForCorruption",
        );

        self.add_transition(State::Scanned, State::Good, Eval::transition, "Eval");
        self.add_transition(State::Scanned, State::NotMounted, Eval::transition, "Eval");
        self.add_transition(State::Scanned, State::WriteFailed, Eval::transition, "Eval");
        self.add_transition(
            State::Scanned,
            State::WornOut,
            CheckWearLeveling::transition,
            "CheckWearLeveling",
        );

        self.add_transition(State::Mounted, State::Scanned, NoOp::transition, "NoOp");
        self.add_transition(
            State::ReadOnly,
            State::Mounted,
            Remount::transition,
            "Remount",
        );
        self.add_transition(
            State::ReadOnly,
            State::MountFailed,
            Remount::transition,
            "Remount",
        );

        self.add_transition(
            State::Corrupt,
            State::Repaired,
            AttemptRepair::transition,
            "AttemptRepair",
        );
        self.add_transition(
            State::Corrupt,
            State::RepairFailed,
            NoOp::transition,
            "NoOp",
        );

        self.add_transition(
            State::RepairFailed,
            State::Reformatted,
            Reformat::transition,
            "Reformat",
        );
        self.add_transition(
            State::RepairFailed,
            State::ReformatFailed,
            NoOp::transition,
            "NoOp",
        );

        self.add_transition(
            State::ReformatFailed,
            State::WaitingForReplacement,
            NoOp::transition,
            "NoOp",
        );

        self.add_transition(
            State::Reformatted,
            State::Unscanned,
            NoOp::transition,
            "NoOp",
        );

        self.add_transition(
            State::WornOut,
            State::WaitingForReplacement,
            MarkForReplacement::transition,
            "MarkForReplacement",
        );

        self.add_transition(State::Repaired, State::Unscanned, NoOp::transition, "NoOp");
        self.add_transition(
            State::WaitingForReplacement,
            State::Replaced,
            Replace::transition,
            "Replace",
        );
        self.add_transition(State::Replaced, State::Unscanned, NoOp::transition, "NoOp");

        self.add_transition(
            State::WriteFailed,
            State::ReadOnly,
            CheckReadOnly::transition,
            "CheckReadOnly",
        );
        // Fsck can either conclude here that everything is fine or the filesystem is corrupt
        self.add_transition(
            State::WriteFailed,
            State::Corrupt,
            CheckForCorruption::transition,
            "CheckForCorruption",
        );
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum State {
    // If the disk is in the corrupted state repairs are attempted
    Corrupt,
    Fail,
    Good,
    Mounted,
    MountFailed,
    // Should be mounted but isn't
    NotMounted,
    // Device is mounted read only
    ReadOnly,
    // Tried to reformat but failed
    ReformatFailed,
    Reformatted,
    // Tried to repair corruption and failed
    RepairFailed,
    Repaired,
    Replaced,
    Scanned,
    Unscanned,
    // The disk could not be repaired and needs to be replaced
    WaitingForReplacement,
    WornOut,
    // Write test failed
    WriteFailed,
}

impl FromStr for State {
    type Err = BynarError;

    fn from_str(s: &str) -> BynarResult<Self> {
        match s {
            "corrupt" => Ok(State::Corrupt),
            "fail" => Ok(State::Fail),
            "good" => Ok(State::Good),
            "mounted" => Ok(State::Mounted),
            "mount_failed" => Ok(State::MountFailed),
            "readonly" => Ok(State::ReadOnly),
            "reformatted" => Ok(State::Reformatted),
            "reformat_failed" => Ok(State::ReformatFailed),
            "repaired" => Ok(State::Repaired),
            "repair_failed" => Ok(State::RepairFailed),
            "replaced" => Ok(State::Replaced),
            "scanned" => Ok(State::Scanned),
            "unscanned" => Ok(State::Unscanned),
            "waiting_for_replacement" => Ok(State::WaitingForReplacement),
            "worn_out" => Ok(State::WornOut),
            _ => Err(BynarError::new(format!("Unknown state: {}", s))),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Corrupt => write!(f, "corrupt"),
            State::Fail => write!(f, "fail"),
            State::Good => write!(f, "good"),
            State::Mounted => write!(f, "mounted"),
            State::MountFailed => write!(f, "mount_failed"),
            State::NotMounted => write!(f, "not_mounted"),
            State::ReadOnly => write!(f, "readonly"),
            State::RepairFailed => write!(f, "repair_failed"),
            State::ReformatFailed => write!(f, "reformat_failed"),
            State::Reformatted => write!(f, "reformatted"),
            State::Repaired => write!(f, "repaired"),
            State::Replaced => write!(f, "replaced"),
            State::Scanned => write!(f, "scanned"),
            State::Unscanned => write!(f, "unscanned"),
            State::WaitingForReplacement => write!(f, "waiting_for_replacement"),
            State::WriteFailed => write!(f, "write_failed"),
            State::WornOut => write!(f, "worn_out"),
        }
    }
}

// Transitions
#[derive(Debug)]
struct AttemptRepair;

#[derive(Debug)]
struct CheckForCorruption;

#[derive(Debug)]
struct CheckWearLeveling;

#[derive(Debug)]
struct CheckReadOnly;

#[derive(Debug)]
struct Eval;

#[derive(Debug)]
struct MarkForReplacement;

#[derive(Debug)]
struct Mount;

#[derive(Debug)]
struct NoOp;

#[derive(Debug)]
struct Remount;

#[derive(Debug)]
struct Replace;

#[derive(Debug)]
struct Reformat;

#[derive(Debug)]
struct Scan;
// Transitions

enum Fsck {
    Ok,
    Corrupt,
}

fn filter_disks(devices: &[PathBuf], storage_detail_id: u32) -> BynarResult<Vec<BlockDevice>> {
    // Gather info on all devices and skip Loopback devices

    // This hides issues where it can't look up a device.  It silently disappears here
    let udev_devices = block_utils::get_all_device_info(&devices)?;
    let block_devices: Vec<BlockDevice> = udev_devices
        .into_iter()
        .map(|d| {
            let dev_path = Path::new("/dev").join(&d.name);
            debug!("inspecting disk: {}", dev_path.display());
            let mut mount_point = None;
            let partitions =
                if let Ok(disk_header) = read_header(&dev_path, disk::DEFAULT_SECTOR_SIZE) {
                    read_partitions(&dev_path, &disk_header, disk::DEFAULT_SECTOR_SIZE)
                        .unwrap_or_else(|_| BTreeMap::new())
                } else {
                    BTreeMap::new()
                };
            if let Ok(Some(mount)) = block_utils::get_mountpoint(&dev_path) {
                debug!("device mount: {}", mount.display());
                mount_point = Some(mount);
            }

            BlockDevice {
                device: d.clone(),
                dev_path,
                // None means disk is not in the database
                device_database_id: None,
                mount_point,
                partitions,
                scsi_info: ScsiInfo::default(),
                state: State::Unscanned,
                storage_detail_id,
                operation_id: None,
            }
        })
        .collect();

    let filtered_devices: Vec<BlockDevice> = block_devices
        .into_iter()
        // Get rid of loopback devices
        .filter(|b| !(b.device.media_type == MediaType::Loopback))
        // Get rid of lvm devices
        .filter(|b| !(b.device.media_type == MediaType::LVM))
        // Get rid of cd/dvd rom devices
        .filter(|b| !(b.device.name.starts_with("sr")))
        // Get rid of ram devices
        .filter(|b| !(b.device.media_type == MediaType::Ram))
        // Get rid of root disk
        .filter(|b| !(b.mount_point == Some(Path::new("/").to_path_buf())))
        // Get rid of /boot
        .filter(|b| !(b.mount_point == Some(Path::new("/boot").to_path_buf())))
        // Get rid of /boot/efi
        .filter(|b| !(b.mount_point == Some(Path::new("/boot/efi").to_path_buf())))
        .filter(|b| {
            for p in b.partitions.iter().enumerate() {
                let partition_path = Path::new("/dev").join(format!(
                    "{name}{num}",
                    name = b.device.name,
                    num = p.0 + 1
                ));
                debug!("partition_path: {}", partition_path.display());
                if let Ok(Some(mount)) = block_utils::get_mountpoint(&partition_path) {
                    debug!("partition mount: {}", mount.display());
                    // Found the root filesystem disk
                    if mount == Path::new("/") {
                        debug!("Found root disk. Skipping");
                        return false;
                    }
                    if mount == Path::new("/boot") {
                        debug!("Found /boot partition.  Skipping");
                        return false;
                    }
                    if mount == Path::new("/boot/efi") {
                        debug!("Found /boot/efi partition. Skipping");
                        return false;
                    }
                }
            }
            true
        })
        .collect();
    debug!("Filtered disks {:?}", filtered_devices);
    Ok(filtered_devices)
}

// Add in any disks that the database knew about that linux can no longer find
fn add_previous_devices(
    devices: &mut Vec<BlockDevice>,
    pool: &Pool<ConnectionManager>,
    host_mapping: &HostDetailsMapping,
) -> BynarResult<()> {
    // Sometimes failed devices are removed from sys/udev and we can no
    // longer find them. This will dig up previously known devices
    let previously_known_devices = get_devices_from_db(&pool, host_mapping.storage_detail_id)?;
    // Add back in missing devices here
    for (dev_id, device_name, device_path) in previously_known_devices {
        if !devices.iter().any(|b| b.dev_path == device_path) {
            // Ok so if the host doesn't know about the device
            // but the database does, what do we do about this?
            // we can't check anything because there's nothing to check
            // do we just mark it for replacement?
            let awaiting_repair = is_hardware_waiting_repair(
                &pool,
                host_mapping.storage_detail_id,
                &device_name,
                None,
            )?;
            debug!(
                "{} awaiting repair: {}",
                device_path.display(),
                awaiting_repair
            );
            // So this never trips because the database thinks this disk is still good
            if !awaiting_repair {
                let b = BlockDevice {
                    device: block_utils::Device {
                        id: None,
                        name: device_path
                            .file_name()
                            .ok_or_else(|| {
                                BynarError::new(format!(
                                    "device {} missing filename",
                                    device_path.display()
                                ))
                            })?
                            .to_string_lossy()
                            .into_owned(),
                        media_type: block_utils::MediaType::Unknown,
                        device_type: block_utils::DeviceType::Unknown,
                        capacity: 0,
                        fs_type: block_utils::FilesystemType::Unknown,
                        serial_number: None,
                    },
                    dev_path: device_path,
                    device_database_id: Some(dev_id),
                    mount_point: None,
                    partitions: BTreeMap::new(),
                    scsi_info: ScsiInfo::default(),
                    state: State::WaitingForReplacement,
                    storage_detail_id: host_mapping.storage_detail_id,
                    operation_id: None,
                };
                save_state(pool, &b, State::WaitingForReplacement)?;
                devices.push(b);
            }
        }
    }

    Ok(())
}

/// Retrives a list of disks, and sets up a state machine on each of them.
/// Retrives previous state and runs through the state machine and preserves
/// the final state in the database before returning a vector of StateMachine
pub fn check_all_disks(
    host_info: &Host,
    pool: &Pool<ConnectionManager>,
    host_mapping: &HostDetailsMapping,
) -> BynarResult<Vec<BynarResult<StateMachine>>> {
    // Udev will only show the disks that are currently attached to the tree
    // It will fail to show disks that have died and disconnected but are still
    // shown as mounted in /etc/mtab
    let mut devices = block_utils::get_block_devices()?;
    let scsi_info = block_utils::sort_scsi_info(&host_info.scsi_info);

    // Gather info on all the currently mounted devices
    let mut mtab_devices: Vec<PathBuf> = block_utils::get_mounted_devices()?
        .iter()
        .map(|d| PathBuf::from("/dev/").join(&d.name))
        .collect();

    // Remove any mtab_devices that udev already knows about leaving only ones
    // that udev doesn't know about, ie broken mounted devices
    mtab_devices.retain(|mtab_device| !devices.iter().any(|device| mtab_device == device));
    devices.extend_from_slice(&mtab_devices);

    // Gather info on all devices and skip Loopback devices
    let mut device_info = filter_disks(&devices, host_mapping.storage_detail_id)?;

    add_previous_devices(&mut device_info, &pool, &host_mapping)?;
    // add the filtered devices to the database.
    // A mutable ref is needed so that the device_database_id can be set
    for mut dev in device_info.iter_mut() {
        add_disk_detail(pool, &mut dev)?;
        // add operation for tracking
        let device_db_id = match dev.device_database_id {
            None => 0,
            Some(i) => i,
        };
        let mut op_info = OperationInfo::new(host_mapping.entry_id, device_db_id);
        add_or_update_operation(pool, &mut op_info)?;

        // store the operation_id in BlockDevice struct
        dev.operation_id = op_info.operation_id;
    }
    //TODO: Add nvme devices to block-utils

    // Create 1 state machine per Device
    // TODO: This could be evaulated in parallel but LVM usage is preventing this
    // There's a bug in LVM that segfaults if more than 1 is started at the same
    // time.
    let mut disk_states: Vec<BynarResult<StateMachine>> = Vec::new();
    for device in device_info {
        let scsi_info = scsi_info
            .iter()
            .find(|r| {
                if let Some(ref dev_name) = r.0.block_device {
                    if let Some(file_name) = dev_name.file_name() {
                        if file_name == OsStr::new(&device.device.name) {
                            return true;
                        }
                    } else {
                        return false;
                    }
                }
                false
            })
            .and_then(|r| Some(r.clone()));
        debug!("thread {} scsi_info: {:?}", process::id(), scsi_info);
        debug!("thread {} device: {:?}", process::id(), device);
        let mut s = StateMachine::new(device, scsi_info, false);
        s.setup_state_machine();
        s.block_device.state = get_state(pool, &s.block_device)?;
        s.run();
        // Save the state to database after state machine finishes its run
        save_state(pool, &s.block_device, s.block_device.state)?;
        disk_states.push(Ok(s));
    }

    Ok(disk_states)
}

#[cfg_attr(test, mockable)]
fn check_filesystem(filesystem_type: &FilesystemType, device: &Path) -> BynarResult<Fsck> {
    match *filesystem_type {
        FilesystemType::Ext2 => check_ext(device),
        FilesystemType::Ext3 => check_ext(device),
        FilesystemType::Ext4 => check_ext(device),
        FilesystemType::Lvm => check_lvm(device),
        FilesystemType::Xfs => check_xfs(device),
        _ => Err(BynarError::from("Unknown filesystem detected")),
    }
}

#[cfg_attr(test, mockable)]
fn repair_filesystem(filesystem_type: &FilesystemType, device: &Path) -> BynarResult<()> {
    match *filesystem_type {
        FilesystemType::Ext2 => {
            repair_ext(device)?;
            Ok(())
        }
        FilesystemType::Ext3 => {
            repair_ext(device)?;
            Ok(())
        }
        FilesystemType::Ext4 => {
            repair_ext(device)?;
            Ok(())
        }
        FilesystemType::Xfs => {
            repair_xfs(device)?;
            Ok(())
        }
        _ => Err(BynarError::from("Unknown filesystem detected")),
    }
}

#[cfg_attr(test, mockable)]
fn check_writable(path: &Path) -> BynarResult<()> {
    debug!(
        "thread {} Checking if {:?} is writable",
        process::id(),
        path
    );
    let temp_path = TempDir::new_in(path, "bynar")?;
    let file_path = temp_path.path().join("write_test");
    debug!("thread {} Creating: {}", process::id(), file_path.display());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)?;
    file.write_all(b"Hello, world!")?;
    Ok(())
}

// TODO: How do you tell if an lvm device is functioning properly?
fn check_lvm(device: &Path) -> BynarResult<Fsck> {
    // lv display should show whether lvm can even access the device
    // do a write test against the device
    debug!("thread {} Checking lvm for corruption", process::id());
    let lvm = Lvm::new(None)?;
    lvm.scan()?;
    // This might fail if the lvm on the disk is corrupt
    if let Ok(vol_names) = lvm.get_volume_group_names() {
        debug!("thread {} lvm volume names: {:?}", process::id(), vol_names);
        for v in vol_names {
            let vg = lvm.vg_open(&v, &OpenMode::Read)?;
            let physical_vols = vg.list_pvs()?;
            trace!(
                "thread {} lvm physical volumes: {:?}",
                process::id(),
                physical_vols
            );
            for p in physical_vols {
                trace!("thread {} physical volume: {}", process::id(), p.get_name());
                if device == Path::new(&p.get_name()) {
                    return Ok(Fsck::Ok);
                }
            }
        }
    }
    Ok(Fsck::Ok)
}

fn check_xfs(device: &Path) -> BynarResult<Fsck> {
    //Any output that is produced when xfs_check is not run in verbose mode
    //indicates that the filesystem has an inconsistency.
    debug!(
        "thread {} Running xfs_repair -n to check for corruption",
        process::id()
    );
    let status = Command::new("xfs_repair")
        .args(&vec!["-n", &device.to_string_lossy()])
        .status()?;
    match status.code() {
        Some(code) => match code {
            0 => Ok(Fsck::Ok),
            1 => Ok(Fsck::Corrupt),
            _ => Err(BynarError::new(format!(
                "xfs_repair failed with code: {}",
                code
            ))),
        },
        //Process terminated by signal
        None => Err(BynarError::from("xfs_repair terminated by signal")),
    }
}

fn repair_xfs(device: &Path) -> BynarResult<()> {
    debug!("thread {} Running xfs_repair", process::id());
    let status = Command::new("xfs_repair").arg(device).status()?;
    match status.code() {
        Some(code) => match code {
            0 => Ok(()),
            _ => Err(BynarError::from("xfs_repair failed")),
        },
        //Process terminated by signal
        None => Err(BynarError::from("e2fsck terminated by signal")),
    }
}

fn check_ext(device: &Path) -> BynarResult<Fsck> {
    debug!(
        "thread {} running e2fsck -n to check for errors",
        process::id()
    );
    let status = Command::new("e2fsck")
        .args(&["-n", &device.to_string_lossy()])
        .status()?;
    match status.code() {
        Some(code) => {
            match code {
                //0 - No errors
                0 => Ok(Fsck::Ok),
                //4 - File system errors left uncorrected.  This requires repair
                4 => Ok(Fsck::Corrupt),
                _ => Err(BynarError::new(format!(
                    "e2fsck returned error code: {}",
                    code
                ))),
            }
        }
        //Process terminated by signal
        None => Err(BynarError::from("e2fsck terminated by signal")),
    }
}

fn repair_ext(device: &Path) -> BynarResult<()> {
    //Run a noninteractive fix.  This will exit with return code 4
    //if it needs human intervention.
    debug!("running e2fsck -p for noninteractive repair");
    let status = Command::new("e2fsck")
        .args(&["-p", &device.to_string_lossy()])
        .status()?;
    match status.code() {
        Some(code) => {
            match code {
                //0 - No errors
                0 => Ok(()),
                // 1 - File system errors corrected
                1 => Ok(()),
                //2 - File system errors corrected, system should
                //be rebooted
                2 => Ok(()),
                _ => Err(BynarError::new(format!(
                    "e2fsck returned error code: {}",
                    code
                ))),
            }
        }
        //Process terminated by signal
        None => Err(BynarError::from("e2fsck terminated by signal")),
    }
}

// Run the smartctl checks against the disk if libata fails
#[cfg_attr(test, mockable)]
fn run_smartctl_check(device: &Path) -> BynarResult<bool> {
    // Enable Smart Scan
    let out = Command::new("smartctl")
        .args(&["-s", "on", &device.to_string_lossy()])
        .output()?;
    let status = match out.status.code() {
        Some(code) => match code {
            // no errors, smart enabled
            0 => {
                let out = Command::new("smartctl")
                    .args(&["-H", &device.to_string_lossy()])
                    .output()?; //Run overall health scan
                match out.status.code() {
                    Some(code) => match code {
                        // no errors, health scan successful
                        0 => true,
                        _ => false,
                    },
                    //Process terminated by signal
                    None => return Err(BynarError::from("smartctl terminated by signal")),
                }
            }
            // could not enable smart checks
            _ => return Err(BynarError::from("smartctl could not enable smart checks")),
        },
        //Process terminated by signal
        None => return Err(BynarError::from("smartctl terminated by signal")),
    };
    Ok(status)
}

// Run smart checks against the disk
#[cfg_attr(test, mockable)]
fn run_smart_checks(device: &Path) -> BynarResult<bool> {
    let status: bool = match libatasmart::Disk::new(device) {
        Ok(mut smart) => {
            match smart.get_smart_status() {
                Ok(stat) => stat,
                Err(e) => {
                    error!("Error {:?} Run SmartMonTools", e);
                    // If ata smart fails, run smartmontools
                    return run_smartctl_check(device);
                }
            }
        }
        Err(e) => {
            error!("Error {:?} Run SmartMonTools", e);
            // If ata smart fails, run smartmontools
            return run_smartctl_check(device);
        }
    };

    Ok(status)
}

#[cfg_attr(test, mockable)]
fn format_device(device: &Device) -> BynarResult<()> {
    let tmp = format!("/dev/{}", device.name);
    let dev_path = Path::new(&tmp);
    let mut fs = Filesystem::new(device.fs_type.to_str());
    if let Filesystem::Xfs { force, .. } = &mut fs {
        // XFS needs to be forced to overwrite
        *force = true;
    };
    format_block_device(&dev_path, &fs)?;

    Ok(())
}

fn is_device_mounted(dev_path: &Path) -> bool {
    // First check if the device itself is mounted
    if let Ok(Some(mount)) = block_utils::get_mountpoint(&dev_path) {
        debug!("thread {} device mount: {}", process::id(), mount.display());
        return true;
    }

    // Then check if any of the partitions it contains are mounted
    let disk_header = match read_header(&dev_path, disk::DEFAULT_SECTOR_SIZE) {
        Ok(h) => h,
        Err(e) => {
            warn!("thread {} Unable to read disk header: {}", process::id(), e);
            return false;
        }
    };

    let partitions = match read_partitions(&dev_path, &disk_header, disk::DEFAULT_SECTOR_SIZE) {
        Ok(p) => p,
        Err(e) => {
            warn!(
                "thread {} Unable to read disk partitions: {}",
                process::id(),
                e
            );
            return false;
        }
    };

    for p in partitions.iter().enumerate() {
        let tmp = format!("{name}{num}", name = dev_path.display(), num = p.0 + 1);
        let partition_path = Path::new(&tmp);
        debug!(
            "thread {} partition_path: {}",
            process::id(),
            partition_path.display()
        );
        if let Ok(Some(mount)) = block_utils::get_mountpoint(&partition_path) {
            debug!(
                "thread {} partition mount: {}",
                process::id(),
                mount.display()
            );
            return true;
        }
    }

    // Unable to tell if it's mounted
    false
}

// While difficult to say for certain this runs through
// a few tests and makes a best guess if the disk is
// blank
fn is_disk_blank(dev: &Path) -> BynarResult<bool> {
    debug!("thread {} Initializing lvm", process::id());
    let lvm = Lvm::new(None)?;
    lvm.scan()?;
    // This might fail if the lvm on the disk is corrupt
    if let Ok(vol_names) = lvm.get_volume_group_names() {
        debug!("thread {} lvm volume names: {:?}", process::id(), vol_names);
        for v in vol_names {
            let vg = lvm.vg_open(&v, &OpenMode::Read)?;
            let physical_vols = vg.list_pvs()?;
            trace!(
                "thread {} lvm physical volumes: {:?}",
                process::id(),
                physical_vols
            );
            for p in physical_vols {
                trace!("thread {} physical volume: {}", process::id(), p.get_name());
                if dev == Path::new(&p.get_name()) {
                    return Ok(false);
                }
            }
        }
    }

    debug!(
        "thread {} Attempting to read gpt disk header",
        process::id()
    );
    if read_header(&dev, disk::DEFAULT_SECTOR_SIZE).is_ok() {
        // We found a gpt header
        return Ok(false);
    }
    debug!("thread {} Checking if disk is mounted", process::id());
    if is_device_mounted(&dev) {
        return Ok(false);
    }
    let device = get_device_info(dev)?;
    debug!(
        "thread {} Mounting device: {}",
        process::id(),
        dev.display()
    );
    let mnt_dir = TempDir::new("bynar")?;
    match mount_device(&device, &mnt_dir.path()) {
        Ok(_) => return Ok(false),
        Err(e) => {
            debug!(
                "thread {} Mounting {} failed: {}",
                process::id(),
                dev.display(),
                e
            );
            //If the partition is EMPTY, it should be mountable, which means if it ISN'T mountable its probably corrupt (and not blank)
            return Ok(false)
        }
    }

    // Best guess is it's blank
    Ok(true)
}

fn is_raid_backed(scsi_info: &Option<(ScsiInfo, Option<ScsiInfo>)>) -> (bool, Vendor) {
    if let Some(scsi_info) = scsi_info {
        if let Some(ref dev_host) = scsi_info.1 {
            if dev_host.scsi_type == ScsiDeviceType::StorageArray
                || dev_host.scsi_type == ScsiDeviceType::Enclosure
            {
                //This device sits behind a raid controller
                match dev_host.vendor {
                    Vendor::Hp => {
                        debug!("thread {} HP raid device found", process::id());
                        return (true, Vendor::Hp);
                    }
                    _ => {
                        // Don't know how to access these yet.
                        warn!(
                            "thread {} Unable to inspect {:?} raid types yet",
                            process::id(),
                            dev_host.vendor
                        );
                        return (false, dev_host.vendor.clone());
                    }
                }
            }
        }
    }
    (false, Vendor::None)
}
