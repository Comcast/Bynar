pub mod ceph;
//#[cfg(feature = "gluster")]
pub mod gluster;

use std::path::Path;
use std::str::FromStr;

use self::ceph::CephBackend;
use self::gluster::GlusterBackend;
use api::service::OpOutcome;
use helpers::error::*;
use serde_derive::*;

/// The outcome of a Backend Operation
pub enum OperationOutcome {
    /// Operation Succeeded
    Success,
    /// Skipped this disk for some reason (boot disk, cannot run operation on specific device, etc.)
    Skipped,
    /// The operation has already been done on the disk
    SkipRepeat,
}

impl From<OperationOutcome> for OpOutcome {
    fn from(out: OperationOutcome) -> OpOutcome {
        match out {
            OperationOutcome::Success => OpOutcome::Success,
            OperationOutcome::Skipped => OpOutcome::Skipped,
            OperationOutcome::SkipRepeat => OpOutcome::SkipRepeat,
        }
    }
}
/// Different distributed storage clusters have different ways of adding and removing
/// disks.  This will be consolidated here in trait impl's.
pub trait Backend {
    /// Add a disk to a cluster.
    /// For ceph that involves osd id provisioning, formatting, auth keys, etc
    /// For gluster or other services it might be much easier
    /// If simulate is passed no action should be taken
    /// An optional osd_id can be provided to ensure the osd is set to that
    fn add_disk(
        &self,
        device: &Path,
        id: Option<u64>,
        simulate: bool,
    ) -> BynarResult<OperationOutcome>;

    /// Remove a disk from a cluster
    /// If simulate is passed no action should be taken
    fn remove_disk(&self, device: &Path, simulate: bool) -> BynarResult<OperationOutcome>;

    /// Check if it's safe to remove a disk from a cluster
    /// If simulate is passed then this always returns true
    /// Take any actions needed with this call to figure out if a disk is safe
    /// to remove from the cluster.
    fn safe_to_remove(
        &self,
        device: &Path,
        simulate: bool,
    ) -> BynarResult<(OperationOutcome, bool)>;
}

/// The supported backend types
#[derive(Clone, Debug, Deserialize)]
pub enum BackendType {
    Ceph,
    Gluster,
}

impl FromStr for BackendType {
    type Err = BynarError;

    fn from_str(s: &str) -> BynarResult<Self> {
        let match_str = s.to_lowercase();
        match match_str.as_ref() {
            "ceph" => Ok(BackendType::Ceph),
            "gluster" => Ok(BackendType::Gluster),
            _ => Err(BynarError::new(format!("Unknown backend type: {}", s))),
        }
    }
}

/// Given a backendType, return a Backend.
pub fn load_backend(
    backend_type: &BackendType,
    config_dir: Option<&Path>,
) -> BynarResult<Box<dyn Backend>> {
    let backend: Box<dyn Backend> = match *backend_type {
        BackendType::Ceph => Box::new(CephBackend::new(config_dir)?),
        //#[cfg(feature = "gluster")]
        BackendType::Gluster => Box::new(GlusterBackend {}),
    };

    Ok(backend)
}
