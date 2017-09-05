//! Disk checks are defined here.  To define a new
//! check create a struct and impl the DiskCheck trait.
//! To create a remediation should that check fail you
//! should also impl the DiskRemediation trait.
//!
//!                            +------>disk_is_ok           +----->replace_disk
//!                            + no                         +no
//!       +---->is_filesystem_corrupted      +--------> can_i_repair
//!       + no                 + yes         + no      ^   + yes
//!is_disk_writable            +------>is_mounted      |   +----->repair_disk
//!       + yes                              + yes     +
//!       +----->disk_is_ok                  +---->unmoun
extern crate blkid;
extern crate block_utils;
extern crate log;

use self::block_utils::{get_mount_device, FilesystemType, RaidType};
use self::blkid::BlkId;

use std::fs::File;
use std::io::{Error, ErrorKind};
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_checks(path: &PathBuf) -> Result<()> {
    debug!("Probing device with blkid");
    let probe = BlkId::new(&path)?;
    probe.do_probe()?;
    let filesystem_type = FilesystemType::from_str(&probe.lookup_value("TYPE")?);
    info!("Filesystem type: {:?}", filesystem_type);

    let corrupted = match check_writable(path) {
        Ok(_) => false,
        Err(e) => {
            //Should proceed to error checking now
            error!("Error writing to disk: {:?}", e);
            true
        }
    };
    let device = get_mount_device(path)?;
    //if corrupted {}

    // NOTE: filesystems should be unmounted before this is run
    match filesystem_type {
        FilesystemType::Btrfs => {}
        FilesystemType::Ext2 => {
            check_ext()?;
        }
        FilesystemType::Ext3 => {
            check_ext()?;
        }
        FilesystemType::Ext4 => {
            check_ext()?;
        }
        FilesystemType::Xfs => {
            check_xfs(&device.unwrap())?;
        }
        FilesystemType::Zfs => {}
        FilesystemType::Unknown => {
            return Err(Error::new(ErrorKind::Other, "Unknown filesystem detected"))
        }
    }
    /*
    // 2. Run repair utility against it if available
    match filesystem_type {
        FilesystemType::Btrfs => {}
        FilesystemType::Ext2 => {
            repair_ext()?;
        }
        FilesystemType::Ext3 => {
            repair_ext()?;
        }
        FilesystemType::Ext4 => {
            repair_ext()?;
        }
        FilesystemType::Xfs => {
            repair_xfs()?;
        }
        FilesystemType::Zfs => {}
        FilesystemType::Unknown => {}
    };
    */
    Ok(())
}

fn check_writable(path: &Path) -> Result<()> {
    debug!("Checking if {:?} is writable", path);
    let mut file = File::create(format!("{}/check_disk", path.to_string_lossy()))?;
    file.write_all(b"Hello, world!")?;
    Ok(())
}

fn check_xfs(device: &Path) -> Result<()> {
    //Any output that is produced when xfs_check is not run in verbose mode
    //indicates that the filesystem has an inconsistency.
    debug!("Running xfs_repair -n to check for corruption");
    let status = Command::new("xfs_repair")
        .args(&vec!["-n", &device.to_string_lossy()])
        .status()?;
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
    Ok(())
}
fn repair_xfs() -> Result<()> {
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
fn check_ext() -> Result<()> {
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
}

fn repair_ext() -> Result<()> {
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
}
