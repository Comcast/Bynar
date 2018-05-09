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
//extern crate blkid;
extern crate block_utils;
extern crate fstab;
extern crate libatasmart;
extern crate log;
extern crate tempdir;

use self::block_utils::{get_mountpoint, Device, FilesystemType, MediaType};
use self::tempdir::TempDir;

use std::fs::OpenOptions;
use std::io::{Error, ErrorKind};
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// After a disk is checked this Status is returned
#[derive(Debug)]
pub struct Status {
    /// Disk was corrupted
    pub corrupted: bool,
    /// This was able to repair it
    pub repaired: bool,
    /// Disk that was operated on
    pub device: Device,
    /// Osd that was operated on
    pub mount_path: PathBuf,
    /// If smart is supported this filed will be filled in
    pub smart_passed: Option<bool>,
}

pub fn check_all_disks() -> Result<Vec<Result<Status>>> {
    let mut results: Vec<Result<Status>> = Vec::new();
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
        results.push(run_checks(&mtab_device));
    }

    //TODO: Add nvme devices to block-utils

    Ok(results)
}

fn run_checks(device_info: &Device) -> Result<Status> {
    let mut disk_status = Status {
        corrupted: false,
        repaired: false,
        device: device_info.clone(),
        mount_path: PathBuf::from(""),
        smart_passed: None,
    };
    let dev_path = format!("/dev/{}", device_info.name);

    // Run a smart check on the base device without partition
    match run_smart_checks(&Path::new(&dev_path)) {
        Ok(result) => {
            disk_status.smart_passed = Some(result);
        }
        Err(e) => {
            error!("Smart test failed: {:?}", e);
        }
    };

    let device = Path::new(&dev_path);
    match get_mountpoint(&device) {
        Ok(mount_info) => {
            match mount_info {
                Some(s) => {
                    // mounted at s
                    info!("Device is mounted at: {:?}", s);
                    debug!("Checking if device exists: {:?}", device);
                    match device.exists() {
                        true => {
                            debug!("udev Probing device {:?}", device);
                            let info = block_utils::get_device_info(&device);
                            let corrupted = match check_writable(&s) {
                                Ok(_) => false,
                                Err(e) => {
                                    //Should proceed to error checking now
                                    error!("Error writing to disk: {:?}", e);
                                    disk_status.corrupted = true;
                                    true
                                }
                            };
                            if corrupted {
                                if let Ok(udev_info) = info {
                                    let check_result =
                                        check_filesystem(&udev_info.fs_type, &device);
                                    debug!("check_filesystem result: {:?}", check_result);
                                    let repair_result =
                                        repair_filesystem(&udev_info.fs_type, &device);
                                    debug!("repair_result result: {:?}", repair_result);
                                } else {
                                    error!(
                                        "Failed to gather udev info on {:?}. error: {:?}",
                                        device, info
                                    );
                                }
                            }
                        }
                        false => {
                            // mountpoint exists for device that does not exist.  Lets flag it
                            // so it gets checked out by a human
                            debug!(
                                "Device does not exist: {:?} but system thinks it is mounted",
                                device
                            );
                            disk_status.corrupted = true;
                        }
                    };
                }
                None => {
                    // It's not mounted.  Lets run an check/repair on it
                    debug!("Device is not mounted: {:?}", device);
                }
            };
        }
        Err(e) => {
            error!("Failed to determine if device is mounted.  {:?}", e);
        }
    };
    Ok(disk_status)
}

fn check_filesystem(filesystem_type: &FilesystemType, device: &Path) -> Result<()> {
    match filesystem_type {
        &FilesystemType::Ext2 => Ok(check_ext(device)?),
        &FilesystemType::Ext3 => Ok(check_ext(device)?),
        &FilesystemType::Ext4 => Ok(check_ext(device)?),
        &FilesystemType::Xfs => Ok(check_xfs(device)?),
        _ => Err(Error::new(ErrorKind::Other, "Unknown filesystem detected")),
    }
}

fn repair_filesystem(filesystem_type: &FilesystemType, device: &Path) -> Result<()> {
    match filesystem_type {
        &FilesystemType::Ext2 => Ok(repair_ext(device)?),
        &FilesystemType::Ext3 => Ok(repair_ext(device)?),
        &FilesystemType::Ext4 => Ok(repair_ext(device)?),
        &FilesystemType::Xfs => Ok(repair_xfs(device)?),
        _ => Err(Error::new(ErrorKind::Other, "Unknown filesystem detected")),
    }
}

fn check_writable(path: &Path) -> Result<()> {
    debug!("Checking if {:?} is writable", path);
    let temp_file = TempDir::new_in(path, "test")?;
    let mut file = OpenOptions::new().write(true).open(temp_file)?;
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
        Some(code) => match code {
            0 => return Ok(()),
            1 => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Filesystem corruption detected",
                ))
            }
            _ => {}
        },
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

fn repair_xfs(device: &Path) -> Result<()> {
    debug!("Running xfs_repair");
    let status = Command::new("xfs_repair").arg(device).status()?;
    match status.code() {
        Some(code) => match code {
            0 => return Ok(()),
            _ => return Err(Error::new(ErrorKind::Other, "xfs_repair failed")),
        },
        //Process terminated by signal
        None => {
            return Err(Error::new(
                ErrorKind::Interrupted,
                "e2fsck terminated by signal",
            ))
        }
    };
}

fn check_ext(device: &Path) -> Result<()> {
    debug!("running e2fsck -n to check for errors");
    let status = Command::new("e2fsck")
        .args(&["-n", &device.to_string_lossy()])
        .status()?;
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

// Run smart checks against the disk
fn run_smart_checks(device: &Path) -> Result<bool> {
    let mut smart = libatasmart::Disk::new(device).map_err(|e| Error::new(ErrorKind::Other, e))?;
    let status = smart
        .get_smart_status()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    Ok(status)
}
