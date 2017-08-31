//! Disk checks are defined here.  To define a new
//! check create a struct and impl the DiskCheck trait.
//! To create a remediation should that check fail you
//! should also impl the DiskRemediation trait.
//!
extern crate blkid;
extern crate block_utils;
extern crate log;
extern crate petgraph;

use self::block_utils::{FilesystemType, RaidType};
use self::blkid::BlkId;
use self::petgraph::Graph;

use std::fmt::Debug;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::io::Result;
use std::path::PathBuf;
use std::process::Command;
use std::result::Result as StdResult;

/// A trait used to create a variety of hard drive checks
pub trait DiskCheck {
    /// Run a check against a dev path
    fn check(self, path: &PathBuf) -> Result<()>;
    /// If a DiskCheck fails then a repair can be used
    /// to try and repair it.
    fn repair(self, path: &PathBuf) -> Result<()>;
}

/// Checks are a DirectedGraph so that checks are run and remediations
/// are run to the point where you can't
/// go any further and then we conclude either we don't know what to do or
/// the disk is dead and we should flag it for replacement.
/// A repair flow can be setup like the one below:
///                                             -+-->can_i_repair -> Replace disk
///                                             +
///                       +------>is_filesystem_corrupted
///                       +                     +
///      +---->is_disk_writable                 +--->disk_is_repaired -> STOP
///      +            ^   +
///is_mounted         |   +----->disk_is_ok -> STOP
///      +            |
///      +--->unmount +
///
/// 1. First we check if the disk is mounted.  If it isn't then we proceed to
/// a write check.  If it is then we unmount and then proceed to a write check
/// 2. We then check if the disk is writeable.  If not then we check for filesystem
/// corruption.  If it is then we conclude the disk is ok and stop
/// 3. We then check if the filesystem is corrupted.  If it is then we proceed to
/// filesystem repair.
/// 4.
pub fn run_checks<T, E>(path: &PathBuf, checks: Graph<T, E>) -> Result<()>
where
    T: Copy + DiskCheck + Debug,
    E: Copy + DiskCheck + Debug,
{
    /*
    for check in checks {
        debug!("Running disk check {:?}", check);
        match check.check(path) {
            Ok(_) => {
                debug!("Check passed");
            }
            Err(e) => {
                error!("Check failed: {}.  Running repair", e);
                match check.repair(path) {
                    Ok(_) => {
                        debug!("Repair was successful!");
                    }
                    Err(e) => {
                        error!("Repair failed: {}", e);
                        //TODO: Escalate?
                    }
                }
            }
        }
    }
    */
    Ok(())
}

/// Check to see if the disk is writable.  If
/// not than maybe the disk is bad
#[derive(Debug)]
pub struct DiskWriteable;
impl DiskCheck for DiskWriteable {
    fn check(self, path: &PathBuf) -> Result<()> {
        let mut p = path.clone();
        p.push("disk_check");

        let mut file = File::create(p.as_path())?;
        file.write_all(b"Hello, world!")?;
        Ok(())
    }
    fn repair(self, path: &PathBuf) -> Result<()> {
        // Nothing to do here except log
        Ok(())
    }
}

#[derive(Debug)]
pub struct Fsck {
    fs_type: FilesystemType,
}

impl Fsck {
    fn check_ext(self) -> Result<()> {
        debug!("running e2fsck -n to check for errors");
        let status = Command::new("e2fsck").arg("-n").status()?;
        match status.code() {
            Some(code) => {
                match code {
                    //0 - No errors
                    0 => return Ok(()),
                    //4 - File system errors left uncorrected.  This requires repair
                    4 => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("e2fsck returned error code: {}", code),
                        ))
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("e2fsck returned error code: {}", code),
                        ))
                    }
                }
            }
            //Process terminated by signal
            None => {
                return Err(Error::new(
                    ErrorKind::Interrupted,
                    "e2fsck terminated by signal",
                ))
            }
        }
        Ok(())
    }
    fn repair_ext(self) -> Result<()> {
        //Run a noninteractive fix.  This will exit with return code 4
        //if it needs human intervention.
        debug!("running e2fsck -p for noninteractive repair");
        let status = Command::new("e2fsck").arg("-p").status()?;
        match status.code() {
            Some(code) => {
                match code {
                    //0 - No errors
                    0 => return Ok(()),
                    // 1 - File system errors corrected
                    1 => return Ok(()),
                    //2 - File system errors corrected, system should
                    //be rebooted
                    2 => return Ok(()),
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("e2fsck returned error code: {}", code),
                        ))
                    }
                }
            }
            //Process terminated by signal
            None => {
                return Err(Error::new(
                    ErrorKind::Interrupted,
                    "e2fsck terminated by signal",
                ))
            }
        }
        Ok(())
    }
    pub fn new(path: &PathBuf) -> StdResult<Self, String> {
        debug!("Probing device with blkid");
        let probe = BlkId::new(&path).map_err(|e| e.to_string())?;
        probe.do_probe().map_err(|e| e.to_string())?;
        let filesystem_type =
            FilesystemType::from_str(&probe.lookup_value("TYPE").map_err(|e| e.to_string())?);
        info!("Filesystem type: {:?}", filesystem_type);

        Ok(Fsck { fs_type: filesystem_type })
    }
}
impl DiskCheck for Fsck {
    // NOTE: filesystems should be unmounted before this is run
    fn check(self, path: &PathBuf) -> Result<()> {
        // 1. Check for the disk filesystem type
        // 2. Run check utility against it.
        match self.fs_type {
            FilesystemType::Btrfs => {}
            FilesystemType::Ext2 => {
                self.check_ext();
            }
            FilesystemType::Ext3 => {
                self.check_ext();
            }
            FilesystemType::Ext4 => {
                self.check_ext();
            }
            FilesystemType::Xfs => {
                //Any output that is produced when xfs_check is not run in verbose mode
                //indicates that the filesystem has an inconsistency.
                debug!("Running xfs_repair -n to check for corruption");
                let status = Command::new("xfs_repair").arg("-n").status()?;
                match status.code() {
                    Some(code) => {
                        match code {
                            0 => return Ok(()),
                            1 => {
                                return Err(Error::new(
                                    ErrorKind::Other,
                                    "Filesystem corruption detected",
                                ))
                            }
                            _ => {}
                        }
                    }
                    //Process terminated by signal
                    None => {
                        return Err(Error::new(
                            ErrorKind::Interrupted,
                            "xfs_repair terminated by signal",
                        ))
                    }
                }
            }
            FilesystemType::Zfs => {}
            FilesystemType::Unknown => {
                return Err(Error::new(ErrorKind::Other, "Unknown filesystem detected"))
            }
        };
        Ok(())
    }
    // If filesystem repair fails we have a choice of either trying to reformat
    // or marking the disk as bad and asking for a replacement
    fn repair(self, path: &PathBuf) -> Result<()> {
        // 1. Check for the disk filesystem type
        // 2. Run repair utility against it if available
        match self.fs_type {
            FilesystemType::Btrfs => {}
            FilesystemType::Ext2 => {
                self.repair_ext();
            }
            FilesystemType::Ext3 => {
                self.repair_ext();
            }
            FilesystemType::Ext4 => {
                self.repair_ext();
            }
            FilesystemType::Xfs => {
                debug!("Running xfs_repair");
                let status = Command::new("xfs_repair").status()?;
                match status.code() {
                    Some(code) => {
                        match code {
                            0 => return Ok(()),
                            _ => return Err(Error::new(ErrorKind::Other, "xfs_repair failed")),
                        }
                    }
                    //Process terminated by signal
                    None => {
                        return Err(Error::new(
                            ErrorKind::Interrupted,
                            "e2fsck terminated by signal",
                        ))
                    }
                };
            }
            FilesystemType::Zfs => {}
            FilesystemType::Unknown => {}
        }
        Ok(())
    }
}
