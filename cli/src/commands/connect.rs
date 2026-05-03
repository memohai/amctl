use std::net::TcpListener;

use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;

use crate::commands::adb::{run_adb, select_device};
use crate::config::{ResolvedSettings, set_key};
use crate::output::{CommandError, CommandResult};

const CONNECTION_HINT_PATH: &str =
    "/sdcard/Android/data/com.memohai.autofish/files/connection-hint.json";
const DEBUG_CONNECTION_HINT_PATH: &str =
    "/sdcard/Android/data/com.memohai.autofish.debug/files/connection-hint.json";
const TRANSPORT_USB_FORWARD: &str = "usb-forward";

pub struct UsbConnectOptions<'a> {
    pub device: Option<&'a str>,
    pub local_port: Option<u16>,
    pub print_only: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionHint {
    pub package_name: String,
    pub version_name: String,
    pub version_code: i64,
    pub service_port: u16,
    pub service_running: bool,
    pub updated_at: i64,
}

pub fn handle_usb_connect(
    settings: &ResolvedSettings,
    options: UsbConnectOptions<'_>,
) -> CommandResult {
    let device_serial = select_device(options.device)?;
    let (hint, hint_path) = read_connection_hint(&device_serial)?;
    if hint.service_port == 0 {
        return Err(CommandError::invalid_params(
            "invalid Autofish connection hint: servicePort must be between 1 and 65535",
        ));
    }
    if !hint.service_running {
        return Err(CommandError::invalid_params(
            "Autofish Service is not running; open the app and turn on Service, then retry",
        ));
    }

    let local_port = resolve_local_port(options.local_port, hint.service_port)?;
    let remote_url = format!("http://127.0.0.1:{local_port}");

    let execute_side_effects = should_execute_connect_side_effects(options.print_only);
    if execute_side_effects {
        run_adb(
            forward_args(&device_serial, local_port, hint.service_port),
            "adb forward failed",
        )?;
        verify_health(&remote_url)?;
        write_usb_config(
            settings,
            &device_serial,
            &remote_url,
            local_port,
            hint.service_port,
        )?;
    }

    Ok(json!({
        "deviceSerial": device_serial,
        "remoteUrl": remote_url,
        "transport": TRANSPORT_USB_FORWARD,
        "localPort": local_port,
        "devicePort": hint.service_port,
        "hintPath": hint_path,
        "hint": {
            "packageName": hint.package_name,
            "versionName": hint.version_name,
            "versionCode": hint.version_code,
            "serviceRunning": hint.service_running,
            "updatedAt": hint.updated_at,
        },
        "forwarded": execute_side_effects,
        "configWritten": execute_side_effects,
        "printOnly": options.print_only,
    }))
}

fn read_connection_hint(serial: &str) -> Result<(ConnectionHint, &'static str), CommandError> {
    let mut failures = Vec::new();
    for path in connection_hint_paths() {
        match read_connection_hint_at(serial, path) {
            Ok(hint) => return Ok((hint, path)),
            Err(err) => failures.push(json!({
                "path": path,
                "reason": hint_read_failure_reason(path, &err),
            })),
        }
    }
    Err(CommandError::invalid_params_with_details(
        "failed to read Autofish connection hint over adb. Open Autofish App and start or stop Service once, then retry. If the error persists, upgrade the Autofish App.",
        json!({
            "triedPaths": failures,
        }),
    ))
}

fn read_connection_hint_at(
    serial: &str,
    path: &'static str,
) -> Result<ConnectionHint, CommandError> {
    let output = run_adb(
        ["-s", serial, "shell", "cat", path],
        "failed to read Autofish connection hint over adb",
    )?;
    let raw = String::from_utf8_lossy(&output.stdout);
    parse_connection_hint(&raw)
}

fn hint_read_failure_reason(path: &str, error: &CommandError) -> String {
    let raw = error
        .raw
        .as_deref()
        .unwrap_or(error.message.as_str())
        .trim();
    let raw = raw
        .strip_prefix("failed to read Autofish connection hint over adb: ")
        .unwrap_or(raw);
    let cat_prefix = format!("cat: {path}: ");
    raw.strip_prefix(&cat_prefix).unwrap_or(raw).to_string()
}

fn connection_hint_paths() -> [&'static str; 2] {
    [CONNECTION_HINT_PATH, DEBUG_CONNECTION_HINT_PATH]
}

fn should_execute_connect_side_effects(print_only: bool) -> bool {
    !print_only
}

fn forward_args(serial: &str, local_port: u16, device_port: u16) -> Vec<String> {
    vec![
        "-s".to_string(),
        serial.to_string(),
        "forward".to_string(),
        format!("tcp:{local_port}"),
        format!("tcp:{device_port}"),
    ]
}

pub fn parse_connection_hint(raw: &str) -> Result<ConnectionHint, CommandError> {
    serde_json::from_str::<ConnectionHint>(raw.trim()).map_err(|e| {
        CommandError::invalid_params(format!(
            "invalid Autofish connection hint JSON: {e}. Open Autofish App and start or stop Service once, then retry."
        ))
    })
}

fn resolve_local_port(requested: Option<u16>, device_port: u16) -> Result<u16, CommandError> {
    if let Some(port) = requested {
        if port == 0 {
            return Err(CommandError::invalid_params(
                "--local-port must be between 1 and 65535",
            ));
        }
        if !is_local_port_available(port) {
            return Err(CommandError::invalid_params(format!(
                "local port {port} is already in use"
            )));
        }
        return Ok(port);
    }
    if is_local_port_available(device_port) {
        return Ok(device_port);
    }
    pick_free_local_port()
}

fn is_local_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn pick_free_local_port() -> Result<u16, CommandError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| CommandError::internal(format!("failed to find a free local port: {e}")))?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|e| CommandError::internal(format!("failed to read local port: {e}")))
}

fn verify_health(remote_url: &str) -> Result<(), CommandError> {
    let url = format!("{}/health", remote_url.trim_end_matches('/'));
    let response = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| CommandError::internal(format!("failed to build HTTP client: {e}")))?
        .get(url)
        .send()
        .map_err(|e| {
            CommandError::internal(format!(
                "adb forward was created, but Autofish /health check failed: {e}"
            ))
        })?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(CommandError::internal(format!(
            "adb forward was created, but Autofish /health returned HTTP {}",
            response.status()
        )))
    }
}

fn write_usb_config(
    settings: &ResolvedSettings,
    serial: &str,
    remote_url: &str,
    local_port: u16,
    device_port: u16,
) -> Result<(), CommandError> {
    set_key(&settings.config_path, "remote.url", remote_url)
        .map_err(|e| CommandError::internal(e.to_string()))?;
    set_key(
        &settings.config_path,
        "connection.transport",
        TRANSPORT_USB_FORWARD,
    )
    .map_err(|e| CommandError::internal(e.to_string()))?;
    set_key(&settings.config_path, "connection.usb.device", serial)
        .map_err(|e| CommandError::internal(e.to_string()))?;
    set_key(
        &settings.config_path,
        "connection.usb.local_port",
        &local_port.to_string(),
    )
    .map_err(|e| CommandError::internal(e.to_string()))?;
    set_key(
        &settings.config_path,
        "connection.usb.device_port",
        &device_port.to_string(),
    )
    .map_err(|e| CommandError::internal(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connection_hint_json() {
        let hint = parse_connection_hint(
            r#"{"packageName":"com.memohai.autofish","versionName":"0.4.0","versionCode":10,"servicePort":8081,"serviceRunning":true,"updatedAt":123}"#,
        )
        .expect("valid hint");
        assert_eq!(hint.service_port, 8081);
        assert!(hint.service_running);
        assert_eq!(hint.package_name, "com.memohai.autofish");
    }

    #[test]
    fn rejects_invalid_connection_hint_json() {
        let err = parse_connection_hint("not json").unwrap_err();
        assert!(
            err.message
                .contains("invalid Autofish connection hint JSON")
        );
    }

    #[test]
    fn includes_release_and_debug_hint_paths() {
        assert_eq!(
            connection_hint_paths(),
            [
                "/sdcard/Android/data/com.memohai.autofish/files/connection-hint.json",
                "/sdcard/Android/data/com.memohai.autofish.debug/files/connection-hint.json"
            ]
        );
    }

    #[test]
    fn hint_read_failure_keeps_message_short_and_paths_in_details() {
        let failures = connection_hint_paths()
            .into_iter()
            .map(|path| {
                json!({
                    "path": path,
                    "message": "No such file or directory",
                    "raw": null,
                })
            })
            .collect::<Vec<_>>();
        let err = CommandError::invalid_params_with_details(
            "failed to read Autofish connection hint over adb. Open Autofish App and start or stop Service once, then retry. If the error persists, upgrade the Autofish App.",
            json!({ "triedPaths": failures }),
        );

        assert!(!err.message.contains("/sdcard/Android/data"));
        assert_eq!(
            err.details
                .as_ref()
                .and_then(|details| details.get("triedPaths"))
                .and_then(|paths| paths.as_array())
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn hint_read_failure_reason_removes_repeated_context() {
        let err = CommandError::internal(format!(
            "failed to read Autofish connection hint over adb: cat: {CONNECTION_HINT_PATH}: No such file or directory"
        ));

        assert_eq!(
            hint_read_failure_reason(CONNECTION_HINT_PATH, &err),
            "No such file or directory"
        );
    }

    #[test]
    fn builds_adb_forward_args() {
        assert_eq!(
            forward_args("RFCX123456", 18081, 8081),
            vec!["-s", "RFCX123456", "forward", "tcp:18081", "tcp:8081"]
        );
    }

    #[test]
    fn print_only_disables_forward_and_config_side_effects() {
        assert!(!should_execute_connect_side_effects(true));
        assert!(should_execute_connect_side_effects(false));
    }
}
