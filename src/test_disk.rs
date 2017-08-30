//! Disk checks are defined here.  To define a new
//! check create a struct and impl the DiskCheck trait.
//! To create a remediation should that check fail you
//! should also impl the DiskRemediation trait.
//!
extern crate block_utils;
extern crate log;

use self::block_utils::RaidType;
use std::fmt::Debug;
use std::path::PathBuf;

/// A trait used to create a variety of hard drive checks
pub trait DiskCheck {
    /// Run a check against a dev path
    fn check(self, path: &PathBuf) -> Result<(), String>;
    /// If a DiskCheck fails then a repair can be used
    /// to try and repair it.
    fn repair(self, path: &PathBuf) -> Result<(), String>;
}

pub fn run_checks<T: Copy + DiskCheck + Debug>(
    path: &PathBuf,
    checks: Vec<T>,
) -> Result<(), String> {
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
    Ok(())
}

/// Check to see if the disk is writable.  If
/// not than maybe the disk is bad
#[derive(Debug)]
pub struct DiskWriteable;
#[derive(Debug)]
pub struct Fsck;
impl DiskCheck for Fsck {
    fn check(self, path: &PathBuf) -> Result<(), String> {
        Ok(())
    }
    fn repair(self, path: &PathBuf) -> Result<(), String> {
        Ok(())
    }
}
