use crate::cli::{Cli, Commands, OutputFormat};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ENV_AF_CONFIG: &str = "AF_CONFIG";
const ENV_AF_OUTPUT: &str = "AF_OUTPUT";
const ENV_AF_DB: &str = "AF_DB";
const ENV_AF_URL: &str = "AF_URL";
const ENV_AF_TOKEN: &str = "AF_TOKEN";
const ENV_AF_ARTIFACT_DIR: &str = "AF_ARTIFACT_DIR";
const ENV_AF_SCREEN_FILE: &str = "AF_SCREEN_FILE";
const ENV_AF_SCREENSHOT_FILE: &str = "AF_SCREENSHOT_FILE";
const ENV_AF_PAGE_DIR: &str = "AF_PAGE_DIR";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileConfig {
    #[serde(default)]
    pub output: OutputSection,
    #[serde(default)]
    pub remote: RemoteSection,
    #[serde(default)]
    pub connection: ConnectionSection,
    #[serde(default)]
    pub memory: MemorySection,
    #[serde(default)]
    pub artifacts: ArtifactSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputSection {
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteSection {
    pub url: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectionSection {
    pub transport: Option<String>,
    #[serde(default)]
    pub usb: UsbConnectionSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsbConnectionSection {
    pub device: Option<String>,
    pub local_port: Option<u16>,
    pub device_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemorySection {
    pub db: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactSection {
    pub dir: Option<String>,
    pub screen_file: Option<String>,
    pub screenshot_file: Option<String>,
    pub page_dir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    Cli,
    Env,
    File,
    Default,
    Unset,
}

impl ConfigSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ConfigSource::Cli => "cli",
            ConfigSource::Env => "env",
            ConfigSource::File => "file",
            ConfigSource::Default => "default",
            ConfigSource::Unset => "unset",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedSettings {
    pub config_path: PathBuf,
    pub output: OutputFormat,
    pub output_source: ConfigSource,
    pub memory_db: PathBuf,
    pub memory_db_source: ConfigSource,
    pub remote_url: Option<String>,
    pub remote_url_source: Option<ConfigSource>,
    pub remote_token: Option<String>,
    pub remote_token_source: Option<ConfigSource>,
    pub connection_transport: Option<String>,
    pub connection_transport_source: Option<ConfigSource>,
    pub connection_usb_device: Option<String>,
    pub connection_usb_device_source: Option<ConfigSource>,
    pub connection_usb_local_port: Option<u16>,
    pub connection_usb_local_port_source: Option<ConfigSource>,
    pub connection_usb_device_port: Option<u16>,
    pub connection_usb_device_port_source: Option<ConfigSource>,
    pub artifact_dir: PathBuf,
    pub artifact_dir_source: ConfigSource,
    pub screen_file: Option<PathBuf>,
    pub screen_file_source: Option<ConfigSource>,
    pub screenshot_file: Option<PathBuf>,
    pub screenshot_file_source: Option<ConfigSource>,
    pub page_dir: PathBuf,
    pub page_dir_source: ConfigSource,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub source: String,
}

pub fn default_config_path() -> PathBuf {
    if let Ok(path) = env::var(ENV_AF_CONFIG) {
        return PathBuf::from(path);
    }
    let base = if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from(".config")
    };
    base.join("af").join("config.toml")
}

fn load_file_config(path: &Path) -> anyhow::Result<FileConfig> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file at {}", path.display()))?;
    toml::from_str(&raw)
        .with_context(|| format!("failed to parse config file at {}", path.display()))
}

pub fn resolve_settings(cli: &Cli) -> anyhow::Result<ResolvedSettings> {
    let config_path = cli.config.clone().unwrap_or_else(default_config_path);
    let file_config = load_file_config(&config_path)?;

    let (output, output_source) = resolve_output(cli, &file_config)?;
    let (memory_db, memory_db_source) = resolve_path(
        cli.memory_db.as_ref().cloned(),
        env::var_os(ENV_AF_DB).map(PathBuf::from),
        file_config.memory.db.as_ref().map(PathBuf::from),
        PathBuf::from("af.db"),
    );
    let (artifact_dir, artifact_dir_source) = resolve_path(
        None,
        env::var_os(ENV_AF_ARTIFACT_DIR).map(PathBuf::from),
        file_config.artifacts.dir.as_ref().map(PathBuf::from),
        default_artifact_dir(&config_path),
    );
    let (screen_file, screen_file_source) = resolve_optional_path(
        env::var_os(ENV_AF_SCREEN_FILE).map(PathBuf::from),
        file_config
            .artifacts
            .screen_file
            .as_ref()
            .map(PathBuf::from),
    );
    let (screenshot_file, screenshot_file_source) = resolve_optional_path(
        env::var_os(ENV_AF_SCREENSHOT_FILE).map(PathBuf::from),
        file_config
            .artifacts
            .screenshot_file
            .as_ref()
            .map(PathBuf::from),
    );
    let (page_dir, page_dir_source) = resolve_path(
        None,
        env::var_os(ENV_AF_PAGE_DIR).map(PathBuf::from),
        file_config.artifacts.page_dir.as_ref().map(PathBuf::from),
        artifact_dir.join("page"),
    );

    let (remote_url, remote_url_source, remote_token, remote_token_source) = match &cli.command {
        Commands::Health { remote } => {
            let (remote_url, remote_url_source) = resolve_optional_string(
                remote.url.clone(),
                env::var(ENV_AF_URL).ok(),
                file_config.remote.url.clone(),
            );
            (remote_url, remote_url_source, None, None)
        }
        Commands::Act { remote, .. }
        | Commands::Observe { remote, .. }
        | Commands::Verify { remote, .. }
        | Commands::Recover { remote, .. } => {
            let (remote_url, remote_url_source) = resolve_optional_string(
                remote.url.clone(),
                env::var(ENV_AF_URL).ok(),
                file_config.remote.url.clone(),
            );
            let (remote_token, remote_token_source) = resolve_optional_string(
                remote.token.clone(),
                env::var(ENV_AF_TOKEN).ok(),
                file_config.remote.token.clone(),
            );
            (
                remote_url,
                remote_url_source,
                remote_token,
                remote_token_source,
            )
        }
        _ => {
            let (remote_url, remote_url_source) = resolve_optional_string(
                None,
                env::var(ENV_AF_URL).ok(),
                file_config.remote.url.clone(),
            );
            let (remote_token, remote_token_source) = resolve_optional_string(
                None,
                env::var(ENV_AF_TOKEN).ok(),
                file_config.remote.token.clone(),
            );
            (
                remote_url,
                remote_url_source,
                remote_token,
                remote_token_source,
            )
        }
    };
    let (connection_transport, connection_transport_source) =
        resolve_optional_string(None, None, file_config.connection.transport.clone());
    let (connection_usb_device, connection_usb_device_source) =
        resolve_optional_string(None, None, file_config.connection.usb.device.clone());
    let (connection_usb_local_port, connection_usb_local_port_source) =
        resolve_optional_u16(None, file_config.connection.usb.local_port);
    let (connection_usb_device_port, connection_usb_device_port_source) =
        resolve_optional_u16(None, file_config.connection.usb.device_port);

    Ok(ResolvedSettings {
        config_path,
        output,
        output_source,
        memory_db,
        memory_db_source,
        remote_url,
        remote_url_source,
        remote_token,
        remote_token_source,
        connection_transport,
        connection_transport_source,
        connection_usb_device,
        connection_usb_device_source,
        connection_usb_local_port,
        connection_usb_local_port_source,
        connection_usb_device_port,
        connection_usb_device_port_source,
        artifact_dir,
        artifact_dir_source,
        screen_file,
        screen_file_source,
        screenshot_file,
        screenshot_file_source,
        page_dir,
        page_dir_source,
    })
}

fn resolve_output(
    cli: &Cli,
    file_config: &FileConfig,
) -> anyhow::Result<(OutputFormat, ConfigSource)> {
    if let Some(value) = cli.output {
        return Ok((value, ConfigSource::Cli));
    }
    if let Ok(value) = env::var(ENV_AF_OUTPUT) {
        return parse_output_format(&value).map(|v| (v, ConfigSource::Env));
    }
    if let Some(value) = &file_config.output.default {
        return parse_output_format(value).map(|v| (v, ConfigSource::File));
    }
    Ok((OutputFormat::Text, ConfigSource::Default))
}

fn parse_output_format(value: &str) -> anyhow::Result<OutputFormat> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => bail!("invalid output format '{value}', expected text or json"),
    }
}

fn resolve_path(
    cli_value: Option<PathBuf>,
    env_value: Option<PathBuf>,
    file_value: Option<PathBuf>,
    default_value: PathBuf,
) -> (PathBuf, ConfigSource) {
    if let Some(value) = cli_value {
        return (value, ConfigSource::Cli);
    }
    if let Some(value) = env_value {
        return (value, ConfigSource::Env);
    }
    if let Some(value) = file_value {
        return (value, ConfigSource::File);
    }
    (default_value, ConfigSource::Default)
}

fn resolve_optional_path(
    env_value: Option<PathBuf>,
    file_value: Option<PathBuf>,
) -> (Option<PathBuf>, Option<ConfigSource>) {
    if let Some(value) = env_value {
        return (Some(value), Some(ConfigSource::Env));
    }
    if let Some(value) = file_value {
        return (Some(value), Some(ConfigSource::File));
    }
    (None, None)
}

fn resolve_optional_string(
    cli_value: Option<String>,
    env_value: Option<String>,
    file_value: Option<String>,
) -> (Option<String>, Option<ConfigSource>) {
    if let Some(value) = cli_value {
        return (Some(value), Some(ConfigSource::Cli));
    }
    if let Some(value) = env_value {
        return (Some(value), Some(ConfigSource::Env));
    }
    if let Some(value) = file_value {
        return (Some(value), Some(ConfigSource::File));
    }
    (None, None)
}

fn resolve_optional_u16(
    cli_value: Option<u16>,
    file_value: Option<u16>,
) -> (Option<u16>, Option<ConfigSource>) {
    if let Some(value) = cli_value {
        return (Some(value), Some(ConfigSource::Cli));
    }
    if let Some(value) = file_value {
        return (Some(value), Some(ConfigSource::File));
    }
    (None, None)
}

pub fn require_url(settings: &ResolvedSettings) -> anyhow::Result<&str> {
    settings
        .remote_url
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("missing URL: pass --url, set AF_URL, or configure remote.url")
        })
}

pub fn require_token(settings: &ResolvedSettings) -> anyhow::Result<&str> {
    settings
        .remote_token
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("missing token: pass --token, set AF_TOKEN, or configure remote.token")
        })
}

fn default_artifact_dir(config_path: &Path) -> PathBuf {
    let base = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("artifacts")
}

pub fn list_entries(settings: &ResolvedSettings) -> Vec<ConfigEntry> {
    let mut out = Vec::new();
    for key in known_keys() {
        if let Some(entry) = get_entry(settings, key) {
            out.push(entry);
        }
    }
    out
}

pub fn get_entry(settings: &ResolvedSettings, key: &str) -> Option<ConfigEntry> {
    if !is_known_key(key) {
        return None;
    }
    let value =
        effective_value(settings, key).unwrap_or((serde_json::Value::Null, ConfigSource::Unset));
    Some(ConfigEntry {
        key: key.to_string(),
        value: value.0,
        source: value.1.as_str().to_string(),
    })
}

fn effective_value(
    settings: &ResolvedSettings,
    key: &str,
) -> Option<(serde_json::Value, ConfigSource)> {
    let value = match key {
        "output.default" => effective_output(settings),
        "remote.url" => {
            effective_optional_string(settings.remote_url.as_deref(), settings.remote_url_source)
        }
        "remote.token" => effective_optional_string(
            settings.remote_token.as_deref(),
            settings.remote_token_source,
        ),
        "connection.transport" => effective_optional_string(
            settings.connection_transport.as_deref(),
            settings.connection_transport_source,
        ),
        "connection.usb.device" => effective_optional_string(
            settings.connection_usb_device.as_deref(),
            settings.connection_usb_device_source,
        ),
        "connection.usb.local_port" => effective_optional_u16(
            settings.connection_usb_local_port,
            settings.connection_usb_local_port_source,
        ),
        "connection.usb.device_port" => effective_optional_u16(
            settings.connection_usb_device_port,
            settings.connection_usb_device_port_source,
        ),
        "memory.db" => Some((
            serde_json::Value::String(settings.memory_db.display().to_string()),
            settings.memory_db_source,
        )),
        "artifacts.dir" => Some((
            serde_json::Value::String(settings.artifact_dir.display().to_string()),
            settings.artifact_dir_source,
        )),
        "artifacts.screen_file" => {
            effective_optional_path(settings.screen_file.as_deref(), settings.screen_file_source)
        }
        "artifacts.screenshot_file" => effective_optional_path(
            settings.screenshot_file.as_deref(),
            settings.screenshot_file_source,
        ),
        "artifacts.page_dir" => Some((
            serde_json::Value::String(settings.page_dir.display().to_string()),
            settings.page_dir_source,
        )),
        _ => None,
    }?;
    Some((config_value_for_output(key, value.0), value.1))
}

fn effective_output(settings: &ResolvedSettings) -> Option<(serde_json::Value, ConfigSource)> {
    if settings.config_path == PathBuf::new() {
        return None;
    }
    Some((
        serde_json::Value::String(settings.output.as_str().to_string()),
        settings.output_source,
    ))
}

fn effective_optional_string(
    value: Option<&str>,
    source: Option<ConfigSource>,
) -> Option<(serde_json::Value, ConfigSource)> {
    Some(match (value, source) {
        (Some(value), Some(source)) => (serde_json::Value::String(value.to_string()), source),
        _ => (serde_json::Value::Null, ConfigSource::Unset),
    })
}

fn effective_optional_path(
    value: Option<&Path>,
    source: Option<ConfigSource>,
) -> Option<(serde_json::Value, ConfigSource)> {
    Some(match (value, source) {
        (Some(value), Some(source)) => (
            serde_json::Value::String(value.display().to_string()),
            source,
        ),
        _ => (serde_json::Value::Null, ConfigSource::Unset),
    })
}

fn effective_optional_u16(
    value: Option<u16>,
    source: Option<ConfigSource>,
) -> Option<(serde_json::Value, ConfigSource)> {
    Some(match (value, source) {
        (Some(value), Some(source)) => (serde_json::json!(value), source),
        _ => (serde_json::Value::Null, ConfigSource::Unset),
    })
}

pub fn config_value_for_output(key: &str, value: serde_json::Value) -> serde_json::Value {
    if is_sensitive_key(key) && !value.is_null() {
        serde_json::Value::String("<redacted>".to_string())
    } else {
        value
    }
}

fn is_sensitive_key(key: &str) -> bool {
    key == "remote.token"
}

pub fn set_key(config_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
    ensure_known_key(key)?;
    let mut config = load_file_config(config_path)?;
    apply_set(&mut config, key, value)?;
    save_file_config(config_path, &config)
}

pub fn unset_key(config_path: &Path, key: &str) -> anyhow::Result<()> {
    ensure_known_key(key)?;
    let mut config = load_file_config(config_path)?;
    apply_unset(&mut config, key);
    save_file_config(config_path, &config)
}

fn save_file_config(path: &Path, config: &FileConfig) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }
    let rendered = toml::to_string_pretty(config).with_context(|| "failed to render config")?;
    fs::write(path, rendered)
        .with_context(|| format!("failed to write config file {}", path.display()))
}

fn apply_set(config: &mut FileConfig, key: &str, value: &str) -> anyhow::Result<()> {
    match key {
        "output.default" => {
            parse_output_format(value)?;
            config.output.default = Some(value.to_string());
        }
        "remote.url" => config.remote.url = Some(value.to_string()),
        "remote.token" => config.remote.token = Some(value.to_string()),
        "connection.transport" => config.connection.transport = Some(value.to_string()),
        "connection.usb.device" => config.connection.usb.device = Some(value.to_string()),
        "connection.usb.local_port" => {
            config.connection.usb.local_port = Some(parse_port_config(value)?)
        }
        "connection.usb.device_port" => {
            config.connection.usb.device_port = Some(parse_port_config(value)?)
        }
        "memory.db" => config.memory.db = Some(value.to_string()),
        "artifacts.dir" => config.artifacts.dir = Some(value.to_string()),
        "artifacts.screen_file" => config.artifacts.screen_file = Some(value.to_string()),
        "artifacts.screenshot_file" => config.artifacts.screenshot_file = Some(value.to_string()),
        "artifacts.page_dir" => config.artifacts.page_dir = Some(value.to_string()),
        _ => bail!("unsupported config key '{key}'"),
    }
    Ok(())
}

fn parse_port_config(value: &str) -> anyhow::Result<u16> {
    let port = value
        .parse::<u16>()
        .with_context(|| format!("invalid port value '{value}'"))?;
    if port == 0 {
        bail!("port must be between 1 and 65535");
    }
    Ok(port)
}

fn apply_unset(config: &mut FileConfig, key: &str) {
    match key {
        "output.default" => config.output.default = None,
        "remote.url" => config.remote.url = None,
        "remote.token" => config.remote.token = None,
        "connection.transport" => config.connection.transport = None,
        "connection.usb.device" => config.connection.usb.device = None,
        "connection.usb.local_port" => config.connection.usb.local_port = None,
        "connection.usb.device_port" => config.connection.usb.device_port = None,
        "memory.db" => config.memory.db = None,
        "artifacts.dir" => config.artifacts.dir = None,
        "artifacts.screen_file" => config.artifacts.screen_file = None,
        "artifacts.screenshot_file" => config.artifacts.screenshot_file = None,
        "artifacts.page_dir" => config.artifacts.page_dir = None,
        _ => {}
    }
}

fn ensure_known_key(key: &str) -> anyhow::Result<()> {
    if is_known_key(key) {
        Ok(())
    } else {
        bail!("unsupported config key '{key}'")
    }
}

pub fn known_keys() -> Vec<&'static str> {
    vec![
        "output.default",
        "remote.url",
        "remote.token",
        "connection.transport",
        "connection.usb.device",
        "connection.usb.local_port",
        "connection.usb.device_port",
        "memory.db",
        "artifacts.dir",
        "artifacts.screen_file",
        "artifacts.screenshot_file",
        "artifacts.page_dir",
    ]
}

fn is_known_key(key: &str) -> bool {
    known_keys().contains(&key)
}

pub fn list_entries_map(settings: &ResolvedSettings) -> BTreeMap<String, serde_json::Value> {
    let mut out = BTreeMap::new();
    for entry in list_entries(settings) {
        out.insert(
            entry.key,
            serde_json::json!({
                "value": entry.value,
                "source": entry.source,
            }),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;
    use serde_json::json;

    fn base_settings() -> ResolvedSettings {
        ResolvedSettings {
            config_path: PathBuf::from("/tmp/af-test-config.toml"),
            output: OutputFormat::Text,
            output_source: ConfigSource::Default,
            memory_db: PathBuf::from("af.db"),
            memory_db_source: ConfigSource::Default,
            remote_url: None,
            remote_url_source: None,
            remote_token: None,
            remote_token_source: None,
            connection_transport: None,
            connection_transport_source: None,
            connection_usb_device: None,
            connection_usb_device_source: None,
            connection_usb_local_port: None,
            connection_usb_local_port_source: None,
            connection_usb_device_port: None,
            connection_usb_device_port_source: None,
            artifact_dir: PathBuf::from("/tmp/artifacts"),
            artifact_dir_source: ConfigSource::Default,
            screen_file: None,
            screen_file_source: None,
            screenshot_file: None,
            screenshot_file_source: None,
            page_dir: PathBuf::from("/tmp/artifacts/page"),
            page_dir_source: ConfigSource::Default,
        }
    }

    #[test]
    fn known_optional_config_key_reports_unset() {
        let settings = base_settings();

        let entry = get_entry(&settings, "remote.url").expect("known key");

        assert_eq!(entry.key, "remote.url");
        assert_eq!(entry.source, "unset");
        assert_eq!(entry.value, serde_json::Value::Null);
    }

    #[test]
    fn unknown_config_key_returns_none() {
        let settings = base_settings();

        assert!(get_entry(&settings, "remote.missing").is_none());
    }

    #[test]
    fn config_list_includes_unset_optional_keys() {
        let settings = base_settings();

        let entries = list_entries_map(&settings);

        assert_eq!(
            entries.get("remote.url"),
            Some(&json!({"source": "unset", "value": null}))
        );
        assert_eq!(
            entries.get("artifacts.screen_file"),
            Some(&json!({"source": "unset", "value": null}))
        );
    }

    #[test]
    fn cli_sources_are_preserved_for_root_options() {
        let cli = Cli::parse_from([
            "af",
            "--config",
            "/tmp/af-test-cli-sources.toml",
            "--output",
            "json",
            "--memory-db",
            "/tmp/source-cli.db",
            "config",
            "list",
        ]);
        let settings = resolve_settings(&cli).expect("resolve");
        let entries = list_entries_map(&settings);

        assert_eq!(
            entries.get("output.default"),
            Some(&json!({"source": "cli", "value": "json"}))
        );
        assert_eq!(
            entries.get("memory.db"),
            Some(&json!({"source": "cli", "value": "/tmp/source-cli.db"}))
        );
    }

    #[test]
    fn remote_token_output_is_redacted() {
        let mut settings = base_settings();
        settings.remote_token = Some("secret-token".into());
        settings.remote_token_source = Some(ConfigSource::Env);

        let entry = get_entry(&settings, "remote.token").expect("known key");

        assert_eq!(entry.source, "env");
        assert_eq!(entry.value, json!("<redacted>"));
    }

    #[test]
    fn connection_metadata_keys_are_known() {
        let mut settings = base_settings();
        settings.connection_transport = Some("usb-forward".into());
        settings.connection_transport_source = Some(ConfigSource::File);
        settings.connection_usb_device = Some("RFCX123456".into());
        settings.connection_usb_device_source = Some(ConfigSource::File);
        settings.connection_usb_local_port = Some(18081);
        settings.connection_usb_local_port_source = Some(ConfigSource::File);
        settings.connection_usb_device_port = Some(8081);
        settings.connection_usb_device_port_source = Some(ConfigSource::File);

        let entries = list_entries_map(&settings);

        assert_eq!(
            entries.get("connection.transport"),
            Some(&json!({"source": "file", "value": "usb-forward"}))
        );
        assert_eq!(
            entries.get("connection.usb.device"),
            Some(&json!({"source": "file", "value": "RFCX123456"}))
        );
        assert_eq!(
            entries.get("connection.usb.local_port"),
            Some(&json!({"source": "file", "value": 18081}))
        );
        assert_eq!(
            entries.get("connection.usb.device_port"),
            Some(&json!({"source": "file", "value": 8081}))
        );
    }
}
