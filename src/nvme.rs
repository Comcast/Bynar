extern crate serde_json;

use std::io::Result;
use std::process::Command;



/// Retrieve the error logs from the nvme device
pub fn get_error_log(dev: &Path) -> Result<String>{
    let out = Command::new("nvme").args(&["error-log", &dev.to_string_lossy(), "-o", "json"]).output()
    if !out.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let deserialized: Vec<String> = serde_json::from_str(&s)?;
    Ok(deserialized)
}

/// Retrieve the firmware logs from the nvme device
pub fn get_firmware_log(dev: &Path) -> Result<String>{
    let out = Command::new("nvme").args(&["fw-log", &dev.to_string_lossy(), "-o", "json"]).output()
    if !out.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let deserialized: Vec<String> = serde_json::from_str(&s)?;
    Ok(deserialized)
}

/// Retrieve the smart logs from the nvme device
pub fn get_smart_log(dev: &Path) -> Result<String>{
    let out = Command::new("nvme").args(&["smart-log", &dev.to_string_lossy(), "-o", "json"]).output()
    if !out.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let deserialized: Vec<String> = serde_json::from_str(&s)?;
    Ok(deserialized)
}

// Format an nvme block device
pub fn format(dev: &Path) -> Result<()>{
    let out = Command::new("nvme").args(&["format", &dev.to_string_lossy()]).output()
    if !out.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(())
}

pub fn list_nvme_namespaces(dev: &Path) -> Result<Vec<String>>{
    let out = Command::new("nvme").args(&["list-ns", &device.to_string_lossy(), "-o", "json"]).output()
    if !out.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let deserialized: Vec<String> = serde_json::from_str(&s)?;
    Ok(deserialized)

}

/// List the nvme controllers on the host
pub fn list_nvme_controllers() -> Result<Vec<String>>{
    let out = Command::new("nvme-list").args(&["-o", "json"]).output()
    if !out.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let deserialized: Vec<String> = serde_json::from_str(&s)?;
    Ok(deserialized)
}

/// List the nvme devices on the host
pub fn list_nvme_devices()->Result<Vec<String>>{
    let out = Command::new("nvme-list").args(&["-o", "json"]).output()
    if !out.status.success() {
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::NotFound,
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let deserialized: Vec<String> = serde_json::from_str(&s)?;
    Ok(deserialized)
}
