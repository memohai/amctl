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

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    Cli,
    Env,
    File,
    Default,
}

impl ConfigSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ConfigSource::Cli => "cli",
            ConfigSource::Env => "env",
            ConfigSource::File => "file",
            ConfigSource::Default => "default",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedSettings {
    pub config_path: PathBuf,
    pub file_config: FileConfig,
    pub output: OutputFormat,
    pub memory_db: PathBuf,
    pub remote_url: Option<String>,
    pub remote_token: Option<String>,
    pub artifact_dir: PathBuf,
    pub screen_file: Option<PathBuf>,
    pub screenshot_file: Option<PathBuf>,
    pub page_dir: PathBuf,
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

    let output = resolve_output(cli, &file_config)?;
    let memory_db = resolve_path(
        cli.memory_db.as_ref().cloned(),
        env::var_os(ENV_AF_DB).map(PathBuf::from),
        file_config.memory.db.as_ref().map(PathBuf::from),
        PathBuf::from("af.db"),
    );
    let artifact_dir = resolve_path(
        None,
        env::var_os(ENV_AF_ARTIFACT_DIR).map(PathBuf::from),
        file_config.artifacts.dir.as_ref().map(PathBuf::from),
        default_artifact_dir(&config_path),
    );
    let screen_file = resolve_optional_path(
        env::var_os(ENV_AF_SCREEN_FILE).map(PathBuf::from),
        file_config
            .artifacts
            .screen_file
            .as_ref()
            .map(PathBuf::from),
    );
    let screenshot_file = resolve_optional_path(
        env::var_os(ENV_AF_SCREENSHOT_FILE).map(PathBuf::from),
        file_config
            .artifacts
            .screenshot_file
            .as_ref()
            .map(PathBuf::from),
    );
    let page_dir = resolve_path(
        None,
        env::var_os(ENV_AF_PAGE_DIR).map(PathBuf::from),
        file_config.artifacts.page_dir.as_ref().map(PathBuf::from),
        artifact_dir.join("page"),
    );

    let (remote_url, remote_token) = match &cli.command {
        Commands::Health { remote } => (
            resolve_optional_string(
                remote.url.clone(),
                env::var(ENV_AF_URL).ok(),
                file_config.remote.url.clone(),
            ),
            None,
        ),
        Commands::Act { remote, .. }
        | Commands::Observe { remote, .. }
        | Commands::Verify { remote, .. }
        | Commands::Recover { remote, .. } => (
            resolve_optional_string(
                remote.url.clone(),
                env::var(ENV_AF_URL).ok(),
                file_config.remote.url.clone(),
            ),
            resolve_optional_string(
                remote.token.clone(),
                env::var(ENV_AF_TOKEN).ok(),
                file_config.remote.token.clone(),
            ),
        ),
        _ => (None, None),
    };

    Ok(ResolvedSettings {
        config_path,
        file_config,
        output,
        memory_db,
        remote_url,
        remote_token,
        artifact_dir,
        screen_file,
        screenshot_file,
        page_dir,
    })
}

fn resolve_output(cli: &Cli, file_config: &FileConfig) -> anyhow::Result<OutputFormat> {
    if let Some(value) = cli.output {
        return Ok(value);
    }
    if let Ok(value) = env::var(ENV_AF_OUTPUT) {
        return parse_output_format(&value);
    }
    if let Some(value) = &file_config.output.default {
        return parse_output_format(value);
    }
    Ok(OutputFormat::Text)
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
) -> PathBuf {
    cli_value
        .or(env_value)
        .or(file_value)
        .unwrap_or(default_value)
}

fn resolve_optional_path(
    env_value: Option<PathBuf>,
    file_value: Option<PathBuf>,
) -> Option<PathBuf> {
    env_value.or(file_value)
}

fn resolve_optional_string(
    cli_value: Option<String>,
    env_value: Option<String>,
    file_value: Option<String>,
) -> Option<String> {
    cli_value.or(env_value).or(file_value)
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
    let value = effective_value(settings, key)?;
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
    match key {
        "output.default" => effective_output(settings),
        "remote.url" => effective_string(
            settings,
            key,
            None,
            Some(ENV_AF_URL),
            settings.file_config.remote.url.as_deref(),
            settings.remote_url.as_deref(),
        ),
        "remote.token" => effective_string(
            settings,
            key,
            None,
            Some(ENV_AF_TOKEN),
            settings.file_config.remote.token.as_deref(),
            settings.remote_token.as_deref(),
        ),
        "memory.db" => effective_path(
            settings,
            key,
            settings.memory_db.as_path(),
            None,
            Some(ENV_AF_DB),
            settings.file_config.memory.db.as_deref(),
            Some(PathBuf::from("af.db")),
        ),
        "artifacts.dir" => effective_path(
            settings,
            key,
            settings.artifact_dir.as_path(),
            None,
            Some(ENV_AF_ARTIFACT_DIR),
            settings.file_config.artifacts.dir.as_deref(),
            Some(default_artifact_dir(&settings.config_path)),
        ),
        "artifacts.screen_file" => effective_optional_path(
            key,
            Some(ENV_AF_SCREEN_FILE),
            settings.file_config.artifacts.screen_file.as_deref(),
            settings.screen_file.as_deref(),
        ),
        "artifacts.screenshot_file" => effective_optional_path(
            key,
            Some(ENV_AF_SCREENSHOT_FILE),
            settings.file_config.artifacts.screenshot_file.as_deref(),
            settings.screenshot_file.as_deref(),
        ),
        "artifacts.page_dir" => effective_path(
            settings,
            key,
            settings.page_dir.as_path(),
            None,
            Some(ENV_AF_PAGE_DIR),
            settings.file_config.artifacts.page_dir.as_deref(),
            Some(settings.artifact_dir.join("page")),
        ),
        _ => None,
    }
}

fn effective_output(settings: &ResolvedSettings) -> Option<(serde_json::Value, ConfigSource)> {
    if settings.config_path == PathBuf::new() {
        return None;
    }
    if let Ok(value) = env::var(ENV_AF_OUTPUT) {
        return Some((serde_json::Value::String(value), ConfigSource::Env));
    }
    if let Some(value) = &settings.file_config.output.default {
        return Some((serde_json::Value::String(value.clone()), ConfigSource::File));
    }
    Some((
        serde_json::Value::String(settings.output.as_str().to_string()),
        ConfigSource::Default,
    ))
}

fn effective_string(
    _settings: &ResolvedSettings,
    _key: &str,
    cli_value: Option<&str>,
    env_key: Option<&str>,
    file_value: Option<&str>,
    effective: Option<&str>,
) -> Option<(serde_json::Value, ConfigSource)> {
    if let Some(value) = cli_value {
        return Some((
            serde_json::Value::String(value.to_string()),
            ConfigSource::Cli,
        ));
    }
    if let Some(env_key) = env_key {
        if let Ok(value) = env::var(env_key) {
            return Some((serde_json::Value::String(value), ConfigSource::Env));
        }
    }
    if let Some(value) = file_value {
        return Some((
            serde_json::Value::String(value.to_string()),
            ConfigSource::File,
        ));
    }
    effective.map(|v| {
        (
            serde_json::Value::String(v.to_string()),
            ConfigSource::Default,
        )
    })
}

fn effective_path(
    _settings: &ResolvedSettings,
    _key: &str,
    effective: &Path,
    cli_value: Option<&Path>,
    env_key: Option<&str>,
    file_value: Option<&str>,
    default_value: Option<PathBuf>,
) -> Option<(serde_json::Value, ConfigSource)> {
    if let Some(value) = cli_value {
        return Some((
            serde_json::Value::String(value.display().to_string()),
            ConfigSource::Cli,
        ));
    }
    if let Some(env_key) = env_key {
        if let Ok(value) = env::var(env_key) {
            return Some((serde_json::Value::String(value), ConfigSource::Env));
        }
    }
    if let Some(value) = file_value {
        return Some((
            serde_json::Value::String(value.to_string()),
            ConfigSource::File,
        ));
    }
    if default_value.is_some() {
        return Some((
            serde_json::Value::String(effective.display().to_string()),
            ConfigSource::Default,
        ));
    }
    None
}

fn effective_optional_path(
    _key: &str,
    env_key: Option<&str>,
    file_value: Option<&str>,
    effective: Option<&Path>,
) -> Option<(serde_json::Value, ConfigSource)> {
    if let Some(env_key) = env_key {
        if let Ok(value) = env::var(env_key) {
            return Some((serde_json::Value::String(value), ConfigSource::Env));
        }
    }
    if let Some(value) = file_value {
        return Some((
            serde_json::Value::String(value.to_string()),
            ConfigSource::File,
        ));
    }
    effective.map(|value| {
        (
            serde_json::Value::String(value.display().to_string()),
            ConfigSource::Default,
        )
    })
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
        "memory.db" => config.memory.db = Some(value.to_string()),
        "artifacts.dir" => config.artifacts.dir = Some(value.to_string()),
        "artifacts.screen_file" => config.artifacts.screen_file = Some(value.to_string()),
        "artifacts.screenshot_file" => config.artifacts.screenshot_file = Some(value.to_string()),
        "artifacts.page_dir" => config.artifacts.page_dir = Some(value.to_string()),
        _ => bail!("unsupported config key '{key}'"),
    }
    Ok(())
}

fn apply_unset(config: &mut FileConfig, key: &str) {
    match key {
        "output.default" => config.output.default = None,
        "remote.url" => config.remote.url = None,
        "remote.token" => config.remote.token = None,
        "memory.db" => config.memory.db = None,
        "artifacts.dir" => config.artifacts.dir = None,
        "artifacts.screen_file" => config.artifacts.screen_file = None,
        "artifacts.screenshot_file" => config.artifacts.screenshot_file = None,
        "artifacts.page_dir" => config.artifacts.page_dir = None,
        _ => {}
    }
}

fn ensure_known_key(key: &str) -> anyhow::Result<()> {
    if known_keys().contains(&key) {
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
        "memory.db",
        "artifacts.dir",
        "artifacts.screen_file",
        "artifacts.screenshot_file",
        "artifacts.page_dir",
    ]
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
