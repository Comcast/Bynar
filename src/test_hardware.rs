use crate::ConfigSettings;
use helpers::{error::BynarError, error::BynarResult};
use libredfish::{
    manager::Manager, power::Power, storage::ArrayController, storage::DiskDrive,
    storage::StorageEnclosure, thermal::Thermal, *,
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

    let controllers = get_array_controllers(&client, &redfish_config)?;
    let mut array_controllers: Vec<ArrayController> = Vec::new();
    let mut storage_enclosures: Vec<StorageEnclosure> = Vec::new();
    let mut disk_drives: Vec<DiskDrive> = Vec::new();
    for controller_id in 1..=controllers.total {
        array_controllers.push(get_array_controller(
            &client,
            &redfish_config,
            controller_id as u64,
        )?);
        // Grab all the enclosures attached to this array controller
        let enclosures = get_storage_enclosures(&client, &redfish_config, controller_id as u64)?;
        for enclosure_id in 1..=enclosures.total {
            storage_enclosures.push(get_storage_enclosure(
                &client,
                &redfish_config,
                controller_id as u64,
                enclosure_id as u64,
            )?);
        }
        //Grab all disks attached to this array controller
        let disks = get_physical_drives(&client, &redfish_config, controller_id as u64)?;
        for disk_id in 1..disks.total {
            disk_drives.push(get_physical_drive(
                &client,
                &redfish_config,
                disk_id as u64,
                controller_id as u64,
            )?);
        }
    }
    let controller_results = array_controllers
        .into_iter()
        .map(evaluate_array_controller)
        .collect();
    let enclosure_results = storage_enclosures
        .into_iter()
        .map(evaluate_enclosure)
        .collect();
    let disk_drive_results = disk_drives
        .into_iter()
        .map(evaluate_drive)
        .collect();
    let manager = get_manager_status(&client, &redfish_config)?;
    let manager_result = evaluate_manager(&manager);

    let thermal = get_thermal_status(&client, &redfish_config)?;
    let thermal_result = evaluate_thermals(&thermal);

    let power = get_power_status(&client, &redfish_config)?;
    let power_result = evaluate_power(&power);

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

fn evaluate_array_controller(controller: ArrayController) -> BynarResult<()> {
    if controller.status.health != "OK" {
        return Err(BynarError::HardwareError {
            name: controller.model,
            location: Some(controller.location),
            location_format: Some(controller.location_format),
            error: "Array controller has failed".to_string(),
            serial_number: Some(controller.serial_number),
        });
    }
    Ok(())
}

fn evaluate_drive(drive: DiskDrive) -> BynarResult<()> {
    if drive.status.health != "OK" {
        return Err(BynarError::HardwareError {
            name: drive.model,
            location: Some(drive.location),
            location_format: Some(drive.location_format),
            error: "Disk has failed".to_string(),
            serial_number: Some(drive.serial_number),
        });
    }
    Ok(())
}

fn evaluate_enclosure(enclosure: StorageEnclosure) -> BynarResult<()> {
    if enclosure.status.health != "OK" {
        return Err(BynarError::HardwareError {
            name: enclosure.model,
            location: Some(enclosure.location),
            location_format: Some(enclosure.location_format),
            error: "Storage enclosure has failed".to_string(),
            serial_number: Some(enclosure.serial_number),
        });
    }

    Ok(())
}

fn evaluate_manager(manager: &Manager) -> Vec<BynarResult<()>> {
    // Look through all the self test results
    // Check if this is an HP machine first?
    let mut results: Vec<BynarResult<()>> = Vec::new();

    for res in &manager.oem.hp.i_lo_self_test_results {
        if res.status != "OK" && res.status != "Informational" {
            // Found an error
            let err = format!("Hp ilo error detected: {}", res.notes);
            results.push(Err(BynarError::new(err)));
        }
    }

    results
}

fn evaluate_power(power: &Power) -> Vec<BynarResult<()>> {
    let mut results: Vec<BynarResult<()>> = Vec::new();

    for psu in &power.power_supplies {
        if psu.status.health != "OK" {
            // Power supply failed
            let err = format!("PSU serial # {} has failed", psu.serial_number);
            results.push(Err(BynarError::new(err)));
        }
    }

    results
}

fn evaluate_thermals(thermal: &Thermal) -> Vec<BynarResult<()>> {
    let mut results: Vec<BynarResult<()>> = Vec::new();
    for fan in &thermal.fans {
        if let Some(fan_health) = &fan.status.health {
            if fan_health != "OK" {
                // Fan failed
                let err = format!("Chassis fan {} has failed", fan.fan_name);
                results.push(Err(BynarError::new(err)));
            }
        }
    }
    for temp_reading in &thermal.temperatures {
        if let Some(temp_health) = &temp_reading.status.health {
            if temp_health != "OK" {
                // Too hot ?
                let err = format!(
                    "Temperature reading for {} is failing.  Location: {}",
                    temp_reading.name, temp_reading.physical_context
                );
                results.push(Err(BynarError::new(err)));
            }
        }
    }

    results
}
