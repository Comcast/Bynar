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

/// collect the hardware health information from redfish
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
    let controller_results = array_controllers
        .into_iter()
        .map(evaluate_storage)
        .collect();
    let enclosure_results = storage_enclosures
        .into_iter()
        .map(evaluate_storage)
        .collect();
    let disk_drive_results = disk_drives.into_iter().map(evaluate_storage).collect();
    let manager = redfish.get_manager_status()?;
    let manager_result = evaluate_manager(&manager);

    let thermal = redfish.get_thermal_status()?;
    let thermal_result = evaluate_thermals(&thermal);

    let power = redfish.get_power_status()?;
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

/// public wrapper for collect_redfish_info
pub fn check_hardware(config: &ConfigSettings) -> BynarResult<HardwareHealthSummary> {
    collect_redfish_info(&config)
}

/// Evaluate an input hardware of type Hardware + Status
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

/// Evaluate the ilo status
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

/// evaluate the power supply
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

/// evaluate the status of the fans and temperature
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

#[cfg(test)]
mod tests {
    use super::*;
    use libredfish::{common::*, manager::*, power::*, storage::*, thermal::*, *};
    //test the evaluate_storage function on each HardwareType
    #[test]
    fn test_evaluate_storage_arraycontroller() {
        // Test status ok
        let ac = ArrayController {
            adapter_type: "".to_string(),
            backup_power_source_status: "".to_string(),
            current_operating_mode: "".to_string(),
            encryption_crypto_officer_password_set: false,
            encryption_enabled: true,
            encryption_fw_locked: true,
            encryption_has_locked_volumes_missing_boot_password: true,
            encryption_mixed_volumes_enabled: true,
            encryption_standalone_mode_enabled: true,
            external_port_count: 0,
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "OK".to_string(),
                    state: "".to_string(),
                },
            },
            hardware_revision: "".to_string(),
            internal_port_count: 0,
            controller_type: "".to_string(),
        };
        assert!(evaluate_storage(ac).is_ok());
        // Test status not ok
        let ac = ArrayController {
            adapter_type: "".to_string(),
            backup_power_source_status: "".to_string(),
            current_operating_mode: "".to_string(),
            encryption_crypto_officer_password_set: false,
            encryption_enabled: true,
            encryption_fw_locked: true,
            encryption_has_locked_volumes_missing_boot_password: true,
            encryption_mixed_volumes_enabled: true,
            encryption_standalone_mode_enabled: true,
            external_port_count: 0,
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "Warning".to_string(),
                    state: "".to_string(),
                },
            },
            hardware_revision: "".to_string(),
            internal_port_count: 0,
            controller_type: "".to_string(),
        };
        assert!(!evaluate_storage(ac).is_ok());
    }
    #[test]
    fn test_evaluate_storage_diskdrive() {
        let dd = DiskDrive {
            block_size_bytes: 0,
            capacity_gb: 0,
            capacity_logical_blocks: 0,
            capacity_mi_b: 0,
            carrier_application_version: "".to_string(),
            carrier_authentication_status: "".to_string(),
            current_temperature_celsius: 0,
            disk_drive_status_reasons: Vec::new(),
            encrypted_drive: false,
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "OK".to_string(),
                    state: "".to_string(),
                },
            },
            interface_speed_mbps: 0,
            interface_type: "".to_string(),
            maximum_temperature_celsius: 0,
            media_type: "".to_string(),
            power_on_hours: None,
            rotational_speed_rpm: 0,
            ssd_endurance_utilization_percentage: None,
            drive_type: "".to_string(),
        };
        assert!(evaluate_storage(dd).is_ok());
        let dd = DiskDrive {
            block_size_bytes: 0,
            capacity_gb: 0,
            capacity_logical_blocks: 0,
            capacity_mi_b: 0,
            carrier_application_version: "".to_string(),
            carrier_authentication_status: "".to_string(),
            current_temperature_celsius: 0,
            disk_drive_status_reasons: Vec::new(),
            encrypted_drive: false,
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "Warning".to_string(),
                    state: "".to_string(),
                },
            },
            interface_speed_mbps: 0,
            interface_type: "".to_string(),
            maximum_temperature_celsius: 0,
            media_type: "".to_string(),
            power_on_hours: None,
            rotational_speed_rpm: 0,
            ssd_endurance_utilization_percentage: None,
            drive_type: "".to_string(),
        };
        assert!(!evaluate_storage(dd).is_ok());
    }
    #[test]
    fn test_evaluate_storage_smartarray() {
        let sa = SmartArray {
            adapter_type: "".to_string(),
            backup_power_source_status: "".to_string(),
            current_operating_mode: "".to_string(),
            encryption_crypto_officer_password_set: false,
            encryption_enabled: false,
            encryption_fw_locked: false,
            encryption_has_locked_volumes_missing_boot_password: false,
            encryption_mixed_volumes_enabled: false,
            encryption_standalone_mode_enabled: false,
            external_port_count: 0,
            hardware_revision: "".to_string(),
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "OK".to_string(),
                    state: "".to_string(),
                },
            },
            internal_port_count: 0,
            array_type: "".to_string(),
        };
        assert!(evaluate_storage(sa).is_ok());
        let sa = SmartArray {
            adapter_type: "".to_string(),
            backup_power_source_status: "".to_string(),
            current_operating_mode: "".to_string(),
            encryption_crypto_officer_password_set: false,
            encryption_enabled: false,
            encryption_fw_locked: false,
            encryption_has_locked_volumes_missing_boot_password: false,
            encryption_mixed_volumes_enabled: false,
            encryption_standalone_mode_enabled: false,
            external_port_count: 0,
            hardware_revision: "".to_string(),
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "Warning".to_string(),
                    state: "".to_string(),
                },
            },
            internal_port_count: 0,
            array_type: "".to_string(),
        };
        assert!(!evaluate_storage(sa).is_ok());
    }
    #[test]
    fn test_evaluate_storage_storageenclosure() {
        let se = StorageEnclosure {
            drive_bay_count: 0,
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "OK".to_string(),
                    state: "".to_string(),
                },
            },
            enclosure_type: "".to_string(),
        };
        assert!(evaluate_storage(se).is_ok());
        let se = StorageEnclosure {
            drive_bay_count: 0,
            hardware_common: HardwareCommon {
                odata: ODataLinks {
                    odata_context: "".to_string(),
                    odata_id: "".to_string(),
                    odata_type: "".to_string(),
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
                description: "".to_string(),
                id: "".to_string(),
                firmware_version: Firmware {
                    current: FirmwareCurrent {
                        version: "".to_string(),
                    },
                },
                location: "".to_string(),
                location_format: "".to_string(),
                model: "".to_string(),
                name: "test".to_string(),
                serial_number: "".to_string(),
                status: AllStatus {
                    health: "Warning".to_string(),
                    state: "".to_string(),
                },
            },
            enclosure_type: "".to_string(),
        };
        assert!(!evaluate_storage(se).is_ok());
    }

    #[test]
    // test the evaluate manager function
    fn test_evaluate_manager() {
        // test with status OK, should be good
        let selftest = OemHpIloselftestresult {
            notes: "".to_string(),
            self_test_name: "test".to_string(),
            status: "OK".to_string(), //OK and Informational are good
        };
        let stlist = vec![selftest];
        let mut manager = Manager {
            odata: ODataLinks {
                odata_context: "".to_string(),
                odata_id: "".to_string(),
                odata_type: "".to_string(),
                links: LinkType::SelfLink {
                    self_url: Href {
                        href: "".to_string(),
                    },
                },
            },
            actions: Action {
                manager_reset: ActionsManagerReset {
                    target: "".to_string(),
                },
            },
            available_actions: Vec::new(),
            command_shell: Commandshell {
                connect_types_supported: Vec::new(),
                enabled: true,
                max_concurrent_sessions: 1,
                service_enabled: true,
            },
            description: "".to_string(),
            ethernet_interfaces: ODataId {
                odata_id: "".to_string(),
            },
            firmware: Firmware {
                current: FirmwareCurrent {
                    version: "".to_string(),
                },
            },
            firmware_version: "".to_string(),
            graphical_console: Commandshell {
                connect_types_supported: Vec::new(),
                enabled: true,
                max_concurrent_sessions: 1,
                service_enabled: true,
            },
            id: "".to_string(),
            log_services: ODataId {
                odata_id: "".to_string(),
            },
            manager_type: "".to_string(),
            name: "test".to_string(),
            network_protocol: ODataId {
                odata_id: "".to_string(),
            },
            oem: manager::Oem {
                hp: manager::OemHp {
                    oem_type: HpType {
                        odata_type: "".to_string(),
                        hp_type: "".to_string(),
                    },
                    actions: OemHpAction {
                        hpi_lo_clear_rest_api_state: ActionsManagerReset {
                            target: "".to_string(),
                        },
                        hpi_lo_reset_to_factory_defaults: OemHpActionshpiloResetToFactoryDefault {
                            reset_type_redfish_allowable_values: Vec::new(),
                            target: "".to_string(),
                        },
                        hpi_lo_i_lo_functionality: ActionsManagerReset {
                            target: "".to_string(),
                        },
                    },
                    available_actions: Vec::new(),
                    clear_rest_api_status: "".to_string(),
                    federation_config: OemHpFederationconfig {
                        i_pv6_multicast_scope: "".to_string(),
                        multicast_announcement_interval: 0,
                        multicast_discovery: "".to_string(),
                        multicast_time_to_live: 0,
                        i_lo_federation_management: "".to_string(),
                    },
                    firmware: OemHpFirmware {
                        current: OemHpFirmwareCurrent {
                            date: "".to_string(),
                            debug_build: false,
                            major_version: 0,
                            minor_version: 0,
                            time: "".to_string(),
                            version_string: "".to_string(),
                        },
                    },
                    license: OemHpLicense {
                        license_key: "".to_string(),
                        license_string: "".to_string(),
                        license_type: "".to_string(),
                    },
                    required_login_fori_lorbsu: false,
                    serial_cli_speed: 0,
                    serial_cli_status: "".to_string(),
                    vsp_log_download_enabled: true,
                    i_lo_self_test_results: stlist,
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
            },
            serial_console: Commandshell {
                connect_types_supported: Vec::new(),
                enabled: true,
                max_concurrent_sessions: 1,
                service_enabled: true,
            },
            status: manager::Status {
                state: "".to_string(),
            },
            root_type: "".to_string(),
            uuid: "".to_string(),
            virtual_media: ODataId {
                odata_id: "".to_string(),
            },
        };
        assert!(
            evaluate_manager(&manager).is_empty(),
            "Should have returned an empty vector"
        );
        // test status is Informational, should be good
        manager.oem.hp.i_lo_self_test_results[0].status = "Informational".to_string();
        assert!(
            evaluate_manager(&manager).is_empty(),
            "Should have returned an empty vector"
        );
        // test status bad
        manager.oem.hp.i_lo_self_test_results[0].status = "Warning".to_string();
        let eval = evaluate_manager(&manager);
        assert!(!eval.is_empty(), "Should return a non-empty vector");
        assert!(eval.len() == 1, "Should return a vector length 1");
        if let Err(err_msg) = &eval[0] {
            assert_eq!(err_msg.to_string(), "Hp ilo error detected: ");
        } else {
            panic!("Failed to get an error message");
        }
    }

    #[test]
    // Test the evaluate thermal function
    fn test_evaluate_thermals() {
        let fan = Fan {
            current_reading: 0,
            fan_name: "test".to_string(),
            oem: FansOem {
                hp: FansOemHp {
                    fan_type: HpType {
                        odata_type: "".to_string(),
                        hp_type: "".to_string(),
                    },
                    location: "meow".to_string(),
                },
            },
            status: SomeStatus {
                health: None, //None becomes "OK"
                state: "".to_string(),
            },
            units: "C".to_string(),
        };
        let temp = Temperature {
            current_reading: 50,
            name: "test".to_string(),
            number: 0,
            lower_threshold_critical: None,
            lower_threshold_fatal: None,
            oem: TemperaturesOem {
                hp: TemperaturesOemHp {
                    temp_type: HpType {
                        odata_type: "".to_string(),
                        hp_type: "".to_string(),
                    },
                    location_xmm: 0,
                    location_ymm: 0,
                },
            },
            physical_context: "moop".to_string(),
            reading_celsius: 0,
            status: SomeStatus {
                health: None, //None becomes "OK"
                state: "".to_string(),
            },
            units: "C".to_string(),
            upper_threshold_critical: 70,
            upper_threshold_fatal: 100,
        };
        let flist = vec![fan];
        let tlist = vec![temp];
        let mut thermal = Thermal {
            odata: ODataLinks {
                odata_context: "".to_string(),
                odata_id: "".to_string(),
                odata_type: "".to_string(),
                links: LinkType::SelfLink {
                    self_url: Href {
                        href: "".to_string(),
                    },
                },
            },
            fans: flist,
            id: "".to_string(),
            name: "thermaltest".to_string(),
            temperatures: tlist,
            thermal_type: "test".to_string(),
        };
        assert!(
            evaluate_thermals(&thermal).is_empty(),
            "Should have returned an empty vector"
        );
        // still fine but use Some("OK") instead
        thermal.temperatures[0].status.health = Some("OK".to_string());
        thermal.fans[0].status.health = Some("OK".to_string());
        assert!(
            evaluate_thermals(&thermal).is_empty(),
            "Should have returned an empty vector"
        );
        // Bad Temperature
        thermal.temperatures[0].status.health = Some("NOT OK".to_string());
        let eval = evaluate_thermals(&thermal);
        assert!(!eval.is_empty(), "Should return a non-empty vector");
        assert!(eval.len() == 1, "Should return a vector length 1");
        if let Err(err_msg) = &eval[0] {
            assert_eq!(
                err_msg.to_string(),
                "Temperature reading for test is failing.  Location: moop"
            );
        } else {
            panic!("Failed to get an error message");
        }
        // Bad Fan
        thermal.temperatures[0].status.health = Some("OK".to_string());
        thermal.fans[0].status.health = Some("NT OK".to_string());
        let eval = evaluate_thermals(&thermal);
        assert!(!eval.is_empty(), "Should return a non-empty vector");
        assert!(eval.len() == 1, "Should return a vector length 1");
        if let Err(err_msg) = &eval[0] {
            assert_eq!(err_msg.to_string(), "Chassis fan test has failed");
        } else {
            panic!("Failed to get an error message");
        }
        // Bad Fan AND Temperature
        thermal.temperatures[0].status.health = Some("N OK".to_string());
        thermal.fans[0].status.health = Some("T OK".to_string());
        let eval = evaluate_thermals(&thermal);
        assert!(!eval.is_empty(), "Should return a non-empty vector");
        assert!(eval.len() == 2, "Should return a vector length 1");
        if let Err(err_msg) = &eval[0] {
            assert_eq!(err_msg.to_string(), "Chassis fan test has failed");
        } else {
            panic!("Failed to get an error message");
        }
        if let Err(err_msg) = &eval[1] {
            assert_eq!(
                err_msg.to_string(),
                "Temperature reading for test is failing.  Location: moop"
            );
        } else {
            panic!("Failed to get an error message");
        }
    }

    #[test]
    // Test the evaluate power function
    fn test_evaluate_power() {
        // power ok
        let powsup = Powersupply {
            firmware_version: "".to_string(),
            last_power_output_watts: 0,
            line_input_voltage: 0,
            line_input_voltage_type: "".to_string(),
            model: "".to_string(),
            name: "test".to_string(),
            oem: PowersuppliesOem {
                hp: PowersuppliesOemHp {
                    power_type: HpType {
                        odata_type: "".to_string(),
                        hp_type: "".to_string(),
                    },
                    average_power_output_watts: 0,
                    bay_number: 0,
                    hotplug_capable: true,
                    max_power_output_watts: 0,
                    mismatched: false,
                    power_supply_status: PowersuppliesOemHpPowersupplystatus {
                        state: "".to_string(),
                    },
                    i_pdu_capable: true,
                },
            },
            power_capacity_watts: 0,
            power_supply_type: "".to_string(),
            serial_number: "".to_string(),
            spare_part_number: "".to_string(),
            status: AllStatus {
                health: "OK".to_string(),
                state: "".to_string(),
            },
        };
        let pslist = vec![powsup];
        let mut pow = Power {
            odata: ODataLinks {
                odata_context: "".to_string(),
                odata_id: "".to_string(),
                odata_type: "".to_string(),
                links: LinkType::SelfLink {
                    self_url: Href {
                        href: "".to_string(),
                    },
                },
            },
            id: "".to_string(),
            name: "test".to_string(),
            oem: power::Oem {
                hp: power::OemHp {
                    oem_type: HpType {
                        odata_type: "".to_string(),
                        hp_type: "".to_string(),
                    },
                    snmp_power_threshold_alert: OemHpSnmppowerthresholdalert {
                        duration_in_min: 0,
                        threshold_watts: 0,
                        trigger: "".to_string(),
                    },
                    links: LinkType::SelfLink {
                        self_url: Href {
                            href: "".to_string(),
                        },
                    },
                },
            },
            power_capacity_watts: 0,
            power_consumed_watts: 0,
            power_control: Vec::new(),
            power_limit: PowercontrolPowerlimit {
                limit_in_watts: None,
            },
            power_metrics: PowercontrolPowermetric {
                average_consumed_watts: 0,
                interval_in_min: 0,
                max_consumed_watts: 0,
                min_consumed_watts: 0,
            },
            power_supplies: pslist,
            redundancy: Vec::new(),
            power_type: "".to_string(),
        };
        assert!(
            evaluate_power(&pow).is_empty(),
            "Should have returned an empty vector"
        );
        // Bad power
        pow.power_supplies[0].status.health = "NOT OK".to_string();
        let eval = evaluate_power(&pow);
        assert!(!eval.is_empty(), "Should return a non-empty vector");
        assert!(eval.len() == 1, "Should return a vector length 1");
        if let Err(err_msg) = &eval[0] {
            assert_eq!(err_msg.to_string(), "PSU serial #  has failed");
        } else {
            panic!("Failed to get an error message");
        }
    }
}
