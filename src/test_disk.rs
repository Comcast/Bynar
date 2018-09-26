//! Disk checks are defined here.  To define a new check create a new
//! struct and then impl Transition for it.  The disks here use a state
//! machine to determine what is and is not possible.  To see the state
//! machine as a visual diagram run one of the unit tests and copy the
//! digraph output into a dot file and convert using
//! `dot -Tps example.dot -o example.ps` to postscript or
//! `dot -Tsvg example.dot -o example.svg` to svg.
//! See comments on the run() function for StateMachine and also
//! the comments under setup_state_machine() to learn more about how it works.
extern crate blkid;
extern crate block_utils;
extern crate fstab;
extern crate gpt;
extern crate libatasmart;
extern crate log;
#[cfg(test)]
extern crate mocktopus;
extern crate petgraph;
extern crate rayon;
extern crate rusqlite;
extern crate tempdir;
extern crate uuid;

use in_progress;

use self::blkid::BlkId;
use self::block_utils::{
    format_block_device, get_mountpoint, is_mounted, mount_device, unmount_device, Device,
    Filesystem, FilesystemType, MediaType,
};
use self::gpt::{disk, header::read_header, partition::read_partitions};
use self::in_progress::*;
#[cfg(test)]
use self::mocktopus::macros::*;
use self::petgraph::graphmap::GraphMap;
use self::petgraph::Directed;
use self::rayon::prelude::*;
use self::rusqlite::Connection;
use self::tempdir::TempDir;
use self::uuid::Uuid;

use std::collections::HashSet;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind};
use std::io::{Result, Write};
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    extern crate rand;
    extern crate simplelog;
    extern crate tempdir;

    use in_progress;

    use std::fs::{remove_file, File};
    use std::io::{Error, ErrorKind, Write};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Mutex;

    use self::tempdir::TempDir;
    use super::blkid::BlkId;
    use super::mocktopus::mocking::*;
    use super::uuid::Uuid;
    use simplelog::{Config, TermLogger};

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
        TermLogger::new(super::log::LevelFilter::Debug, Config::default()).unwrap();

        // Mock smart to return Ok(true)
        super::run_smart_checks.mock_safe(|_| MockResult::Return(Ok(true)));

        let dev = create_loop_device();

        let blkid = BlkId::new(&dev).unwrap();
        blkid.do_probe().unwrap();
        let drive_uuid = blkid.lookup_value("UUID").unwrap();
        debug!("drive_uuid: {}", drive_uuid);

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();
        let sql_dir = TempDir::new("bynar").unwrap();
        let db_path = sql_dir.path().join("base.sqlite3");
        //cleanup old
        let _ = remove_file(&db_path);
        let conn = super::connect_to_repair_database(&db_path).unwrap();

        let d = super::Device {
            id: Some(drive_id),
            name: dev.file_name().unwrap().to_str().unwrap().to_string(),
            media_type: super::MediaType::Rotational,
            capacity: 26214400,
            fs_type: super::FilesystemType::Xfs,
            serial_number: Some("123456".into()),
        };
        let mut s = super::StateMachine::new(d, conn, true);
        s.setup_state_machine();
        s.print_graph();
        s.restore_state().unwrap();
        s.run();
        println!("final state: {}", s.state);

        cleanup_loop_device(&dev);

        assert_eq!(s.state, super::State::Good);
    }

    #[test]
    fn test_state_machine_bad_filesystem() {
        TermLogger::new(super::log::LevelFilter::Debug, Config::default()).unwrap();

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
            ]).status()
            .unwrap();

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();
        let sql_dir = TempDir::new("bynar").unwrap();
        let db_path = sql_dir.path().join("bad_fs.sqlite3");
        //cleanup old
        let _ = remove_file(&db_path);
        let conn = super::connect_to_repair_database(&db_path).unwrap();
        let d = super::Device {
            id: Some(drive_id),
            name: dev.file_name().unwrap().to_str().unwrap().to_string(),
            media_type: super::MediaType::Rotational,
            capacity: 26214400,
            fs_type: super::FilesystemType::Xfs,
            serial_number: Some("123456".into()),
        };
        let mut s = super::StateMachine::new(d, conn, true);
        s.setup_state_machine();
        s.print_graph();
        s.restore_state().unwrap();
        s.run();
        println!("final state: {}", s.state);

        cleanup_loop_device(&dev);
        assert_eq!(s.state, super::State::Good);
    }

    #[test]
    fn test_state_machine_replace_disk() {
        // Smart passes, write fails,  check_filesystem fails, attemptRepair and reformat fails
        TermLogger::new(super::log::LevelFilter::Debug, Config::default()).unwrap();

        super::run_smart_checks.mock_safe(|_| MockResult::Return(Ok(true)));
        super::check_writable
            .mock_safe(|_| MockResult::Return(Err(Error::new(ErrorKind::Other, "Mock Error"))));
        super::check_filesystem.mock_safe(|_, _| MockResult::Return(Ok(super::Fsck::Corrupt)));
        super::repair_filesystem
            .mock_safe(|_, _| MockResult::Return(Err(Error::new(ErrorKind::Other, "Mock Error"))));

        // TODO: Can't mock outside dependencies.  Need a wrapper function or something
        super::format_device.mock_safe(|_| MockResult::Return(Err("error".to_string())));
        // That should leave the disk in WaitingForReplacement

        let dev = create_loop_device();

        let blkid = BlkId::new(&dev).unwrap();
        blkid.do_probe().unwrap();
        let drive_uuid = blkid.lookup_value("UUID").unwrap();
        debug!("drive_uuid: {}", drive_uuid);

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();
        let sql_dir = TempDir::new("bynar").unwrap();
        let db_path = sql_dir.path().join("replace_disk.sqlite3");
        //cleanup old
        let _ = remove_file(&db_path);
        let conn = super::connect_to_repair_database(&db_path).unwrap();

        let d = super::Device {
            id: Some(drive_id),
            name: dev.file_name().unwrap().to_str().unwrap().to_string(),
            media_type: super::MediaType::Rotational,
            capacity: 26214400,
            fs_type: super::FilesystemType::Xfs,
            serial_number: Some("123456".into()),
        };
        let mut s = super::StateMachine::new(d, conn, false);
        s.setup_state_machine();
        s.print_graph();
        s.restore_state().unwrap();
        s.run();
        println!("final state: {}", s.state);

        cleanup_loop_device(&dev);

        assert_eq!(s.state, super::State::WaitingForReplacement);
    }

    #[test]
    fn test_state_machine_replaced_disk() {
        TermLogger::new(super::log::LevelFilter::Debug, Config::default()).unwrap();
        super::run_smart_checks.mock_safe(|_| MockResult::Return(Ok(true)));

        let dev = create_loop_device();

        let blkid = BlkId::new(&dev).unwrap();
        blkid.do_probe().unwrap();
        let drive_uuid = blkid.lookup_value("UUID").unwrap();
        debug!("drive_uuid: {}", drive_uuid);

        let drive_id = Uuid::parse_str(&drive_uuid).unwrap();
        let sql_dir = TempDir::new("bynar").unwrap();
        let db_path = sql_dir.path().join("replaced_disk.sqlite3");
        //cleanup old
        let _ = remove_file(&db_path);
        let conn = super::connect_to_repair_database(&db_path).unwrap();

        // Set the previous state to something other than Unscanned
        in_progress::save_state(&conn, dev.as_path(), super::State::WaitingForReplacement).unwrap();

        let d = super::Device {
            id: Some(drive_id),
            name: dev.file_name().unwrap().to_str().unwrap().to_string(),
            media_type: super::MediaType::Rotational,
            capacity: 26214400,
            fs_type: super::FilesystemType::Xfs,
            serial_number: Some("123456".into()),
        };

        let mut s = super::StateMachine::new(d, conn, true);
        s.setup_state_machine();
        s.print_graph();
        s.restore_state().unwrap();
        s.run();
        println!("final state: {}", s.state);
        assert_eq!(s.state, super::State::Good);
    }

}

trait Transition {
    // Transition from the current state to an ending state given an Event
    // database connection can be used to save and resume state
    fn transition(
        to_state: &State,
        device: &mut Device,
        db_conn: &Connection,
        simulate: bool, // Pretend to transition and skip any side effects
    ) -> State;
}

impl Transition for AttemptRepair {
    // Take a Corrupt
    fn transition(
        to_state: &State,
        device: &mut Device,
        _db_conn: &Connection,
        simulate: bool,
    ) -> State {
        debug!("running AttemptRepair transition");
        // Disk filesystem is corrupted.  Attempt repairs.
        let tmp = format!("/dev/{}", device.name);
        let dev_path = Path::new(&tmp);
        if !simulate {
            match repair_filesystem(&device.fs_type, &dev_path) {
                Ok(_) => *to_state,
                Err(e) => {
                    error!("repair_filesystem failed on {:?}: {}", device, e);
                    State::Fail
                }
            }
        } else {
            *to_state
        }
    }
}

impl Transition for CheckForCorruption {
    fn transition(
        to_state: &State,
        device: &mut Device,
        _db_conn: &Connection,
        simulate: bool,
    ) -> State {
        debug!("running CheckForCorruption transition");
        if !simulate {
            let tmp = format!("/dev/{}", device.name);
            let dev_path = Path::new(&tmp);
            match check_filesystem(&device.fs_type, &dev_path) {
                Ok(fsck) => match fsck {
                    // Writes are failing but fsck is ok?
                    // What else could be wrong?  The filesystem could be read only
                    // or ??
                    Fsck::Ok => State::Fail,
                    // The filesystem is corrupted.  Proceed to repair
                    Fsck::Corrupt => *to_state,
                },
                Err(e) => {
                    error!("check_filesystem failed on {:?}: {}", device, e);
                    State::Fail
                }
            }
        } else {
            *to_state
        }
    }
}

impl Transition for CheckReadOnly {
    fn transition(
        _to_state: &State,
        device: &mut Device,
        _db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running CheckReadOnly transition");
        let tmp = format!("/dev/{}", device.name);
        let dev_path = Path::new(&tmp);

        // Try again
        State::Fail
    }
}

impl Transition for CheckWearLeveling {
    fn transition(
        to_state: &State,
        _device: &mut Device,
        _db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running CheckWearLeveling transition");

        //TODO: How can we check wear leveling?
        *to_state
    }
}

// Evaluate whether a scanned drive is good
impl Transition for Eval {
    fn transition(
        to_state: &State,
        device: &mut Device,
        db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running Eval transition");
        let tmp = format!("/dev/{}", device.name);
        let dev_path = Path::new(&tmp);

        let mnt_dir: TempDir;
        if !is_mounted(&dev_path).unwrap_or(false) {
            debug!("Mounting device: {}", dev_path.display());
            mnt_dir = match TempDir::new("bynar") {
                Ok(d) => d,
                Err(e) => {
                    error!("temp dir creation failed: {:?}", e);
                    return State::Fail;
                }
            };
            // This requires root perms
            if let Err(e) = mount_device(&device, &mnt_dir.path().to_string_lossy()) {
                error!("Mounting {} failed: {}", dev_path.display(), e);
                return State::MountFailed;
            }
        }

        debug!("Getting mountpoint info for {}", dev_path.display());
        match get_mountpoint(&dev_path) {
            Ok(mount_info) => match mount_info {
                Some(info) => {
                    debug!("mount info: {:?}", info);
                    if let Err(e) = save_mount_location(&db_conn, &dev_path, &info) {
                        error!(
                            "save mount location failed for {}: {:?}",
                            dev_path.display(),
                            e
                        );
                        return State::Fail;
                    }

                    debug!("Checking if mount is writable");
                    match check_writable(&info) {
                        // Mount point is writeable, smart passed.  Good to go
                        Ok(_) => *to_state,
                        Err(e) => {
                            //Should proceed to error checking now
                            error!("Error writing to disk: {:?}", e);
                            State::WriteFailed
                        }
                    }
                }
                None => {
                    // Device isn't mounted.  Mount in temp location and check?
                    // what if it doesn't have a filesystem.

                    // This shouldn't happen because !is_mounted above
                    // took care of it
                    error!("Device is not mounted");
                    State::NotMounted
                }
            },
            Err(e) => {
                error!(
                    "Error getting mountpoint for {}: {:?}",
                    dev_path.display(),
                    e
                );
                State::Fail
            }
        }
    }
}

impl Transition for MarkForReplacement {
    fn transition(
        to_state: &State,
        device: &mut Device,
        db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running MarkForReplacement transition");
        let tmp = format!("/dev/{}", device.name);
        let dev_path = Path::new(&tmp);
        match is_disk_in_progress(&db_conn, &dev_path) {
            Ok(_in_progress) => {
                //if in_progress {
                // This is already in waiting for replacement
                *to_state
                //} else {
                // TODO: Does this make sense?
                //*to_state
                //}
            }
            Err(e) => {
                error!(
                    "Error getting disk progress for {}: {:?}",
                    dev_path.display(),
                    e
                );
                State::Fail
            }
        }
    }
}

impl Transition for Mount {
    fn transition(
        to_state: &State,
        device: &mut Device,
        _db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running mount transition");

        let tmp = format!("/dev/{}", device.name);
        let dev_path = Path::new(&tmp);
        let mnt_dir: TempDir;

        debug!("Mounting device: {}", dev_path.display());
        mnt_dir = match TempDir::new("bynar") {
            Ok(d) => d,
            Err(e) => {
                error!("temp dir creation failed: {:?}", e);
                return State::Fail;
            }
        };
        if let Err(e) = mount_device(&device, &mnt_dir.path().to_string_lossy()) {
            error!("Mounting {} failed: {}", dev_path.display(), e);
            return State::Fail;
        }

        *to_state
    }
}

impl Transition for NoOp {
    fn transition(
        to_state: &State,
        _device: &mut Device,
        _db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running NoOp transition");

        *to_state
    }
}

impl Transition for Reformat {
    fn transition(
        to_state: &State,
        device: &mut Device,
        _db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running Reformat transition");
        let tmp = format!("/dev/{}", device.name);
        let dev_path = Path::new(&tmp);

        // Ensure we're not mounted before this it run
        match get_mountpoint(&dev_path) {
            Ok(info) => {
                if info.is_some() {
                    // Must unmount
                    if let Err(e) = unmount_device(&info.unwrap().to_string_lossy()) {
                        error!("unmount failed: {}", e);
                    }
                }
            }
            Err(e) => {
                // Fail to get mountpoint.  Prob ok?
                error!("get_mountpoint failed: {}", e);
            }
        };
        match format_device(&device) {
            Ok(_) => {
                // We need to update the UUID of the block device now.
                let blkid = BlkId::new(&dev_path).expect("blkid creation failed");
                blkid.do_probe().expect("blkid probe failed");
                let drive_uuid = blkid
                    .lookup_value("UUID")
                    .expect("blkid lookup uuid failed");
                debug!("drive_uuid: {}", Uuid::parse_str(&drive_uuid).unwrap());
                device.id = Some(Uuid::parse_str(&drive_uuid).unwrap());

                *to_state
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
        to_state: &State,
        _device: &mut Device,
        _db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running Remount transition");
        // TODO: Investigate using libmount here
        match Command::new("mount").args(&["-o", "remount"]).output() {
            Ok(output) => {
                if output.status.success() {
                    *to_state
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
        to_state: &State,
        _device: &mut Device,
        _db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running Replace transition");
        // So we know at this point that the disk has been replaced
        // We know the device we're working with

        *to_state
    }
}

impl Transition for Scan {
    fn transition(
        to_state: &State,
        device: &mut Device,
        db_conn: &Connection,
        _simulate: bool,
    ) -> State {
        debug!("running Scan transition");

        // TODO: Handle devices that live behind a raid controller
        let dev_path = format!("/dev/{}", device.name);
        // Run a smart check on the base device without partition
        match run_smart_checks(&Path::new(&dev_path)) {
            Ok(_) => match save_smart_results(&db_conn, &Path::new(&dev_path), true) {
                Ok(_) => *to_state,
                Err(e) => {
                    error!("Save smart results failed {:?}", e);
                    State::Fail
                }
            },
            Err(e) => {
                error!("Smart test failed: {:?}", e);
                match save_smart_results(&db_conn, &Path::new(&dev_path), false) {
                    Ok(_) => State::Fail,
                    Err(e) => {
                        error!("Save smart results failed {:?}", e);
                        State::Fail
                    }
                }
            }
        }
    }
}

pub struct StateMachine {
    // A record of the transitions so they can be written as a dot graph
    // for later visual debugging
    dot_graph: Vec<(State, State, String)>,
    // Mapping of valid From -> To transitions
    graph: GraphMap<
        State,
        fn(to_state: &State, device: &mut Device, db_conn: &Connection, simulate: bool) -> State,
        Directed,
    >,
    pub state: State,
    pub disk: Device,
    pub db_conn: Connection,
    simulate: bool,
}

impl fmt::Debug for StateMachine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.state)
    }
}

impl StateMachine {
    fn new(disk: Device, db_conn: Connection, simulate: bool) -> Self {
        StateMachine {
            dot_graph: Vec::new(),
            graph: GraphMap::new(),
            state: State::Unscanned,
            disk,
            db_conn,
            simulate,
        }
    }

    fn add_transition(
        &mut self,
        from_state: State,
        to_state: State,
        callback: fn(to_state: &State, device: &mut Device, db_conn: &Connection, simulate: bool)
            -> State,
        // Just for debugging dot graph creation
        transition_label: &str,
    ) {
        self.dot_graph
            .push((from_state, to_state, transition_label.to_string()));
        self.graph.add_edge(from_state, to_state, callback);
    }

    // Restore the state of this machine from the database if it was previously saved
    // otherwise do nothing and start over at Unscanned
    fn restore_state(&mut self) -> ::std::result::Result<(), rusqlite::Error> {
        let tmp = format!("/dev/{}", self.disk.name);
        let dev_path = Path::new(&tmp);
        if let Some(s) = get_state(&self.db_conn, &dev_path)? {
            self.state = s;
        }

        Ok(())
    }

    // Run all transitions until we can't go any further and return
    fn run(&mut self) {
        // Start at the current state the disk is at and work our way down the graph
        debug!("Starting state: {}", self.state);
        let tmp = format!("/dev/{}", self.disk.name);
        let dev_path = Path::new(&tmp);
        'outer: loop {
            // Gather all the possible edges from this current State
            let edges: Vec<(
                State,
                State,
                &fn(to_state: &State, device: &mut Device, db_conn: &Connection, simulate: bool)
                    -> State,
            )> = self.graph.edges(self.state).collect();
            // Some states have multiple paths they could go down.
            // If the state transition returns State::Fail try the next path
            let beginning_state = self.state;
            for e in edges {
                debug!("Attempting {} to {} transition", &e.0, &e.1);
                let state = e.2(&e.1, &mut self.disk, &self.db_conn, self.simulate);
                if state == State::Fail {
                    // Try the next transition if there is one
                    debug!("Fail. Trying next transition");
                    continue;
                }
                if state == State::WaitingForReplacement {
                    // TODO: Is this the only state we shouldn't advance further from?
                    debug!("state==State::WaitingForReplacement");
                    self.state = state;
                    save_state(&self.db_conn, &dev_path, self.state).expect("save_state failed");
                    break 'outer;
                } else if state == State::Good {
                    debug!("state==State::Good");
                    self.state = state;
                    save_state(&self.db_conn, &dev_path, self.state).expect("save_state failed");
                    break 'outer;
                }
                // transition succeeded.  Save state and go around the loop again
                // This won't detect if the transitions return something unexpected
                if state == e.1 {
                    debug!("state==e.1 {}=={}", state, e.1);
                    self.state = state;
                    save_state(&self.db_conn, &dev_path, self.state).expect("save_state failed");
                    break;
                }
            }
            // At this point we should've advanced further.  If not then we're stuck in an infinite loop
            // Note this won't detect more complicated infinite loops where it changes through 2 states
            // before ending back around again.
            if self.state == beginning_state {
                // We're stuck in an infinite loop we can't advance further from
                debug!("Breaking loop: {}=={}", self.state, beginning_state);
                break 'outer;
            }
        }
    }

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
        //self.add_transition(State::Scanned, State::NotMounted, Scan::transition);
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

        self.add_transition(State::Repaired, State::Good, NoOp::transition, "NoOp");
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
    type Err = String;

    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
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
            _ => Err(format!("Unknown state: {}", s)),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

pub fn check_all_disks(db: &Path) -> Result<Vec<Result<StateMachine>>> {
    // Udev will only show the disks that are currently attached to the tree
    // It will fail to show disks that have died and disconnected but are still
    // shown as mounted in /etc/mtab
    let devices = block_utils::get_block_devices().map_err(|e| Error::new(ErrorKind::Other, e))?;

    // Gather info on all devices and skip Loopback devices
    let device_info: Vec<Device> = block_utils::get_all_device_info(devices.as_slice())
        .map_err(|e| Error::new(ErrorKind::Other, e))?
        .into_iter()
        // Get rid of loopback devices
        .filter(|d| !(d.media_type == MediaType::Loopback))
        // Get rid of lvm devices
        .filter(|d| !(d.media_type == MediaType::LVM))
        // Get rid of ram devices
        .filter(|d| !(d.media_type == MediaType::Ram))
        // Get rid of disks that aren't Ceph
        .filter(|d| {
            let dev_path = Path::new("/dev").join(&d.name);
            debug!("inspecting disk: {}", dev_path.display());
            let disk_header = match read_header(&dev_path, disk::DEFAULT_SECTOR_SIZE){
                Ok(h) => h,
                Err(e) => {
                    error!("Unable to read disk header: {}  Skipping", e);
                    return false;
                }
            };
            let partitions = match read_partitions(&dev_path, &disk_header, disk::DEFAULT_SECTOR_SIZE){
                Ok(p) => p,
                Err(e) => {
                    error!("Unable to read disk partitions: {}  Skipping", e);
                    return false;
                }
            };
            for p in partitions {
                if p.part_type_guid.os != "Ceph" {
                    debug!("Skipping non ceph disk: {}", dev_path.display());
                    return false;
                }
            }
            true
        })
        .collect();

    // Gather info on all the currently mounted devices
    let mut mtab_devices: Vec<Device> = block_utils::get_mounted_devices()?;

    // Remove any mtab_devices that udev already knows about leaving only ones
    // that udev doesn't know about, ie broken mounted devices
    mtab_devices.retain(|mtab_device| {
        !device_info
            .iter()
            .any(|udev_device| mtab_device.name.contains(&udev_device.name))
    });

    // Check any devices that udev doesn't know about that are still mounted
    for mtab_device in mtab_devices {
        //results.push(run_checks(&mtab_device));
    }

    //TODO: Add nvme devices to block-utils

    // Create 1 state machine per Device and evaulate all devices in parallel
    let disk_states: Vec<Result<StateMachine>> = device_info
        .clone()
        .into_par_iter()
        .map(|device| {
            // Lookup the disk and see if it's in progress.  If so then
            // set the state to WaitingOnReplacement.
            // Resume where we left off
            let conn =
                connect_to_repair_database(db).map_err(|e| Error::new(ErrorKind::Other, e))?;
            let mut s = StateMachine::new(device, conn, false);
            s.setup_state_machine();
            s.restore_state()
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
            s.run();
            // Possibly serialize the state here to the database to resume later
            if s.state == State::WaitingForReplacement {
                info!("Connecting to database to check if disk is in progress");
                let disk_path = Path::new("/dev").join(&s.disk.name);
                let conn =
                    connect_to_repair_database(db).map_err(|e| Error::new(ErrorKind::Other, e))?;
                let in_progress = is_disk_in_progress(&conn, &disk_path)
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;
            }
            Ok(s)
        }).collect();

    Ok(disk_states)
}

#[cfg_attr(test, mockable)]
fn check_filesystem(filesystem_type: &FilesystemType, device: &Path) -> Result<Fsck> {
    match *filesystem_type {
        FilesystemType::Ext2 => check_ext(device),
        FilesystemType::Ext3 => check_ext(device),
        FilesystemType::Ext4 => check_ext(device),
        FilesystemType::Xfs => check_xfs(device),
        _ => Err(Error::new(ErrorKind::Other, "Unknown filesystem detected")),
    }
}

#[cfg_attr(test, mockable)]
fn repair_filesystem(filesystem_type: &FilesystemType, device: &Path) -> Result<()> {
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
        _ => Err(Error::new(ErrorKind::Other, "Unknown filesystem detected")),
    }
}

#[cfg_attr(test, mockable)]
fn check_writable(path: &Path) -> Result<()> {
    debug!("Checking if {:?} is writable", path);
    let temp_path = TempDir::new_in(path, "bynar")?;
    let file_path = temp_path.path().join("write_test");
    debug!("Creating: {}", file_path.display());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)?;
    file.write_all(b"Hello, world!")?;
    Ok(())
}

fn check_xfs(device: &Path) -> Result<Fsck> {
    //Any output that is produced when xfs_check is not run in verbose mode
    //indicates that the filesystem has an inconsistency.
    debug!("Running xfs_repair -n to check for corruption");
    let status = Command::new("xfs_repair")
        .args(&vec!["-n", &device.to_string_lossy()])
        .status()?;
    match status.code() {
        Some(code) => match code {
            0 => Ok(Fsck::Ok),
            1 => Ok(Fsck::Corrupt),
            _ => Err(Error::new(
                ErrorKind::Other,
                format!("xfs_repair failed with code: {}", code).as_ref(),
            )),
        },
        //Process terminated by signal
        None => Err(Error::new(
            ErrorKind::Interrupted,
            "xfs_repair terminated by signal",
        )),
    }
}

fn repair_xfs(device: &Path) -> Result<()> {
    debug!("Running xfs_repair");
    let status = Command::new("xfs_repair").arg(device).status()?;
    match status.code() {
        Some(code) => match code {
            0 => Ok(()),
            _ => Err(Error::new(ErrorKind::Other, "xfs_repair failed")),
        },
        //Process terminated by signal
        None => Err(Error::new(
            ErrorKind::Interrupted,
            "e2fsck terminated by signal",
        )),
    }
}

fn check_ext(device: &Path) -> Result<Fsck> {
    debug!("running e2fsck -n to check for errors");
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
                _ => Err(Error::new(
                    ErrorKind::Other,
                    format!("e2fsck returned error code: {}", code),
                )),
            }
        }
        //Process terminated by signal
        None => Err(Error::new(
            ErrorKind::Interrupted,
            "e2fsck terminated by signal",
        )),
    }
}

fn repair_ext(device: &Path) -> Result<()> {
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
                _ => Err(Error::new(
                    ErrorKind::Other,
                    format!("e2fsck returned error code: {}", code),
                )),
            }
        }
        //Process terminated by signal
        None => Err(Error::new(
            ErrorKind::Interrupted,
            "e2fsck terminated by signal",
        )),
    }
}

// Run smart checks against the disk
#[cfg_attr(test, mockable)]
fn run_smart_checks(device: &Path) -> Result<bool> {
    let mut smart = libatasmart::Disk::new(device).map_err(|e| Error::new(ErrorKind::Other, e))?;
    let status = smart
        .get_smart_status()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    Ok(status)
}

#[cfg_attr(test, mockable)]
fn format_device(device: &Device) -> ::std::result::Result<(), String> {
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
