pub mod ceph;
//#[cfg(feature = "gluster")]
//pub mod gluster;

use std::io::Result;
use std::path::Path;
use std::result::Result as StdResult;
use std::str::FromStr;

use self::ceph::CephBackend;

/// Different distributed storage clusters have different ways of adding and removing
/// disks.  This will be consolidated here in trait impl's.
pub trait Backend {
    /// Add a disk to a cluster.
    /// For ceph that involves osd id provisioning, formatting, auth keys, etc
    /// For gluster or other services it might be much easier
    /// If simulate is passed no action should be taken
    /// An optional osd_id can be provided to ensure the osd is set to that
    fn add_disk(&self, device: &Path, id: Option<u64>, simulate: bool) -> Result<()>;

    /// Remove a disk from a cluster
    /// If simulate is passed no action should be taken
    fn remove_disk(&self, device: &Path, simulate: bool) -> Result<()>;
}

/// The supported backend types
#[derive(Clone, Debug, Deserialize)]
pub enum BackendType {
    Ceph,
    //Gluster
}

impl FromStr for BackendType {
    type Err = String;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        let match_str = s.to_lowercase();
        match match_str.as_ref() {
            "ceph" => Ok(BackendType::Ceph),
            _ => Err(format!("Unknown backend type: {}", s)),
        }
    }
}

/// Given a backend path, return a 'd Backend.
pub fn load_backend(
    backend_type: &BackendType,
    config_dir: Option<&Path>,
) -> StdResult<Box<Backend>, String> {
    let backend: Box<Backend> = match backend_type {
        &BackendType::Ceph => Box::new(CephBackend::new(config_dir).map_err(|e| e.to_string())?),
        //#[cfg(feature = "gluster")]
        //&BackendType::Gluster => Box::new(GlusterBackend::new(config_dir)
        //.map_err(|e| e.to_string())?),
        //e => return Err(format!("Unknown backend: {:?}", e)),
    };

    Ok(backend)
}
