use crate::ConfigSettings;
use helpers::{error::BynarError, error::BynarResult, error::HardwareError};
use libredfish::{
    common::Status, manager::Manager, power::Power, storage::ArrayController, storage::DiskDrive,
    storage::Hardware, storage::StorageEnclosure, thermal::Thermal, *,
};
use log::debug;
use reqwest::Client;

/// Summary of all the hardware status information
pub struct HardwareHealthSummary {
    pub array_controllers: Vec<BynarResult<()>>,
    /// Physical disk drive status
    pub disk_drives: Vec<BynarResult<()>>,
    /// iLo status
    pub manager: Vec<BynarResult<()>>,
    /// Power supply status
    pub power: Vec<BynarResult<()>>,
    pub storage_enclosures: Vec<BynarResult<()>>,
    /// Fan status
    pub thermals: Vec<BynarResult<()>>,
}

fn collect_redfish_info(config: &ConfigSettings) -> BynarResult<HardwareHealthSummary> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;

    if config.redfish_ip.is_none() {
        debug!("Redfish ip address not specified.  Skipping checks");
        return Ok(HardwareHealthSummary {
            array_controllers: vec![],
            disk_drives: vec![],
            manager: vec![],
            power: vec![],
            storage_enclosures: vec![],
            thermals: vec![],
        });
    }
    let redfish_config = Config {
        user: config.redfish_username.clone(),
        password: config.redfish_password.clone(),
        endpoint: config.redfish_ip.clone().unwrap(),
        port: config.redfish_port,
    };
    let redfish = Redfish::new(client, redfish_config);
    let controllers = redfish.get_array_controllers()?; //get_array_controllers(&client, &redfish_config)?;
    let mut array_controllers: Vec<ArrayController> = Vec::new();
    let mut storage_enclosures: Vec<StorageEnclosure> = Vec::new();
    let mut disk_drives: Vec<DiskDrive> = Vec::new();
    for controller_id in 1..=controllers.mult_hardware.total {
        array_controllers.push(redfish.get_array_controller(controller_id as u64)?);
        // Grab all the enclosures attached to this array controller
        let enclosures = redfish.get_storage_enclosures(controller_id as u64)?;
        for enclosure_id in 1..=enclosures.mult_hardware.total {
            storage_enclosures
                .push(redfish.get_storage_enclosure(controller_id as u64, enclosure_id as u64)?);
        }
        //Grab all disks attached to this array controller
        let disks = redfish.get_physical_drives(controller_id as u64)?;
        for disk_id in 1..disks.mult_hardware.total {
            disk_drives.push(redfish.get_physical_drive(disk_id as u64, controller_id as u64)?);
        }
    }
    let controller_results = get_results!(array_controllers, evaluate_storage);
    let enclosure_results = get_results!(storage_enclosures, evaluate_storage);
    let disk_drive_results = get_results!(disk_drives, evaluate_storage);
    let manager_result = mult_results!(redfish, get_manager_status, evaluate_manager); //evaluate_manager(&manager);
    let thermal_result = mult_results!(redfish, get_thermal_status, evaluate_thermals); //evaluate_thermals(&thermal);
    let power_result = mult_results!(redfish, get_power_status, evaluate_power); //evaluate_power(&power);

    Ok(HardwareHealthSummary {
        array_controllers: controller_results,
        disk_drives: disk_drive_results,
        manager: manager_result,
        power: power_result,
        storage_enclosures: enclosure_results,
        thermals: thermal_result,
    })
}

pub fn check_hardware(config: &ConfigSettings) -> BynarResult<HardwareHealthSummary> {
    collect_redfish_info(&config)
}

fn evaluate_storage<T>(hardware: T) -> BynarResult<()>
where
    T: Hardware + Status,
{
    if hardware.health() != "OK" {
        return Err(BynarError::HardwareError(HardwareError {
            name: hardware.model(),
            location: Some(hardware.location()),
            location_format: Some(hardware.location_format()),
            error: format!("{:?} has failed", hardware.get_type()),
            serial_number: Some(hardware.serial_number()),
        }));
    }
    Ok(())
}

fn evaluate_manager(manager: &Manager) -> Vec<BynarResult<()>> {
    // Look through all the self test results
    // Check if this is an HP machine first?
    let mut results: Vec<BynarResult<()>> = Vec::new();
    eval!(
        results,
        &manager.oem.hp.i_lo_self_test_results,
        "Informational",
        "Hp ilo error detected {}",
        notes
    );

    results
}

fn evaluate_power(power: &Power) -> Vec<BynarResult<()>> {
    let mut results: Vec<BynarResult<()>> = Vec::new();
    eval!(
        results,
        &power.power_supplies,
        "PSU serial # {} has failed",
        serial_number
    );

    results
}

fn evaluate_thermals(thermal: &Thermal) -> Vec<BynarResult<()>> {
    let mut results: Vec<BynarResult<()>> = Vec::new();
    eval!(
        results,
        &thermal.fans,
        "Chassis fan {} has failed",
        fan_name
    );
    eval!(
        results,
        &thermal.temperatures,
        "Temperature reading for {} is failing.  Location: {}",
        name,
        physical_context
    );
    results
}
