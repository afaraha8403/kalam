use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_devices() -> anyhow::Result<Vec<AudioDevice>> {
    let host = cpal::default_host();

    let default_device = host.default_input_device();
    let default_name = default_device.as_ref().and_then(|d| d.name().ok());

    // Collect so we get all devices (iterator consumption / platform quirks)
    let input_devices: Vec<cpal::Device> = host.input_devices()?.collect();
    log::info!("Enumerated {} input device(s)", input_devices.len());

    let mut devices = Vec::new();
    for (index, device) in input_devices.into_iter().enumerate() {
        let name = device
            .name()
            .unwrap_or_else(|_| format!("Input device {}", index + 1));
        let is_default = default_name
            .as_ref()
            .is_some_and(|default| default.trim() == name.trim());
        let id = if is_default {
            "default".to_string()
        } else {
            format!("device_{}", index)
        };

        devices.push(AudioDevice {
            id,
            name,
            is_default,
        });
    }

    if devices.is_empty() {
        if let Some(_default) = default_device {
            devices.push(AudioDevice {
                id: "default".to_string(),
                name: "Default Input Device".to_string(),
                is_default: true,
            });
        }
    }

    Ok(devices)
}

// Helper function to get a specific device by ID
#[allow(dead_code)]
pub fn get_device_by_id(device_id: &str) -> anyhow::Result<cpal::Device> {
    let host = cpal::default_host();

    if device_id == "default" {
        host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device available"))
    } else {
        // Try to find device by iterating
        let input_devices = host.input_devices()?;
        for (index, device) in input_devices.enumerate() {
            let expected_id = format!("device_{}", index);
            if expected_id == device_id {
                return Ok(device);
            }
        }
        // Fall back to default if not found
        host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("Input device '{}' not found", device_id))
    }
}

// Helper function to get a device by name
#[allow(dead_code)]
pub fn get_device_by_name(device_name: &str) -> anyhow::Result<cpal::Device> {
    let host = cpal::default_host();

    // Try to find device by name
    let input_devices = host.input_devices()?;
    for device in input_devices {
        if let Ok(name) = device.name() {
            if name == device_name {
                return Ok(device);
            }
        }
    }

    // Fall back to default if not found
    host.default_input_device()
        .ok_or_else(|| anyhow::anyhow!("Input device '{}' not found", device_name))
}
