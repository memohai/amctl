use std::ffi::OsStr;
use std::process::{Command, Output};

use crate::core::error_code::ErrorCode;
use crate::output::CommandError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    pub state: String,
}

pub fn select_device(requested: Option<&str>) -> Result<String, CommandError> {
    let devices = list_adb_devices()?;
    if let Some(serial) = requested {
        let device = devices.iter().find(|device| device.serial == serial);
        match device {
            Some(device) if device.state == "device" => return Ok(serial.to_string()),
            Some(device) => {
                return Err(CommandError::invalid_params(format!(
                    "ADB device {serial} is not ready (state: {})",
                    device.state
                )));
            }
            None => {
                return Err(CommandError::invalid_params(format!(
                    "ADB device {serial} was not found; connected devices: {}",
                    format_device_list(&devices)
                )));
            }
        }
    }

    let ready = devices
        .iter()
        .filter(|device| device.state == "device")
        .collect::<Vec<_>>();
    match ready.len() {
        0 => Err(CommandError::invalid_params(
            "no ready ADB device found; run `adb devices`, connect a device, or pass --device",
        )),
        1 => Ok(ready[0].serial.clone()),
        _ => Err(CommandError::invalid_params(format!(
            "multiple ADB devices found; pass --device with one of: {}",
            ready
                .iter()
                .map(|device| device.serial.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub fn run_adb<I, S>(args: I, context: &str) -> Result<Output, CommandError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("adb")
        .args(args)
        .output()
        .map_err(|e| adb_command_error(context, e))?;
    if !output.status.success() {
        return Err(command_status_error(context, &output));
    }
    Ok(output)
}

pub fn parse_adb_devices(output: &str) -> Vec<Device> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("List of devices") {
                return None;
            }
            let mut parts = line.split_whitespace();
            let serial = parts.next()?;
            let state = parts.next()?;
            Some(Device {
                serial: serial.to_string(),
                state: state.to_string(),
            })
        })
        .collect()
}

fn list_adb_devices() -> Result<Vec<Device>, CommandError> {
    let output = run_adb(["devices"], "adb devices failed")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_adb_devices(&stdout))
}

fn format_device_list(devices: &[Device]) -> String {
    if devices.is_empty() {
        return "<none>".to_string();
    }
    devices
        .iter()
        .map(|device| format!("{} ({})", device.serial, device.state))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn adb_command_error(context: &str, error: std::io::Error) -> CommandError {
    CommandError {
        code: ErrorCode::InternalError,
        message: format!(
            "{context}: {error}. Make sure Android Platform Tools are installed and `adb` is on PATH."
        ),
        retryable: false,
        status: None,
        raw: None,
        details: None,
    }
}

pub fn command_status_error(context: &str, output: &Output) -> CommandError {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    CommandError {
        code: ErrorCode::InternalError,
        message: if stderr.is_empty() {
            context.to_string()
        } else {
            format!("{context}: {stderr}")
        },
        retryable: false,
        status: None,
        raw: Some(if stdout.is_empty() { stderr } else { stdout }),
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_adb_devices_output() {
        let devices = parse_adb_devices(
            "List of devices attached\nRFCX123456\tdevice\nemulator-5554\toffline\n\n",
        )
        .into_iter()
        .map(|device| (device.serial, device.state))
        .collect::<Vec<_>>();
        assert_eq!(
            devices,
            vec![
                ("RFCX123456".to_string(), "device".to_string()),
                ("emulator-5554".to_string(), "offline".to_string())
            ]
        );
    }
}
