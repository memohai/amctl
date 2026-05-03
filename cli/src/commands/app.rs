use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::commands::adb::{run_adb, select_device};
use crate::core::error_code::ErrorCode;
use crate::output::{CommandError, CommandResult};

const PACKAGE_NAME: &str = "com.memohai.autofish";
const GITHUB_REPO: &str = "memohai/Autofish";
const DOWNLOAD_BASE: &str = "https://github.com/memohai/Autofish/releases/download";
const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/memohai/Autofish/releases";
const USER_AGENT: &str = concat!("af/", env!("CARGO_PKG_VERSION"));

pub struct InstallOptions<'a> {
    pub device: Option<&'a str>,
    pub version: &'a str,
    pub force: bool,
    pub dry_run: bool,
}

pub struct UninstallOptions<'a> {
    pub device: Option<&'a str>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub version: String,
    pub tag: String,
    pub asset: String,
    pub github_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub version_name: String,
    pub version_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallAction {
    Install,
    Upgrade,
    Downgrade,
    Skip,
}

impl InstallAction {
    fn as_done_str(self) -> &'static str {
        match self {
            InstallAction::Install => "installed",
            InstallAction::Upgrade => "upgraded",
            InstallAction::Downgrade => "downgraded",
            InstallAction::Skip => "skipped",
        }
    }

    fn as_dry_run_str(self) -> &'static str {
        match self {
            InstallAction::Install => "would-install",
            InstallAction::Upgrade => "would-upgrade",
            InstallAction::Downgrade => "would-downgrade",
            InstallAction::Skip => "skipped",
        }
    }
}

pub fn handle_install(options: InstallOptions<'_>) -> CommandResult {
    let client = http_client()?;
    let target = resolve_release(&client, options.version)?;
    let device_serial = select_device(options.device)?;
    let installed_before = query_installed_app(&device_serial)?;
    let action = decide_install_action(
        installed_before
            .as_ref()
            .map(|app| app.version_name.as_str()),
        &target.version,
        options.force,
    )?;

    if options.dry_run {
        return Ok(install_result_json(InstallResultView {
            device_serial: &device_serial,
            target: &target,
            installed_before: installed_before.as_ref(),
            action: action.as_dry_run_str(),
            apk_source: "none",
            apk_path: None,
            installed_after: installed_before.as_ref(),
            dry_run: true,
        }));
    }

    if action == InstallAction::Skip {
        return Ok(install_result_json(InstallResultView {
            device_serial: &device_serial,
            target: &target,
            installed_before: installed_before.as_ref(),
            action: action.as_done_str(),
            apk_source: "cache",
            apk_path: None,
            installed_after: installed_before.as_ref(),
            dry_run: false,
        }));
    }

    let downloaded = resolve_apk(&client, &target)?;
    adb_install(
        &device_serial,
        &downloaded.path,
        action == InstallAction::Downgrade,
    )?;
    let installed_after = query_installed_app(&device_serial)?;
    let after_version = installed_after
        .as_ref()
        .map(|app| app.version_name.as_str())
        .unwrap_or("");
    if after_version != target.version {
        return Err(CommandError::assertion_failed_with_details(
            format!(
                "installed version mismatch: expected {}, got {}",
                target.version,
                if after_version.is_empty() {
                    "<not installed>"
                } else {
                    after_version
                }
            ),
            json!({
                "deviceSerial": device_serial,
                "targetVersion": target.version,
                "installedVersionAfter": after_version,
            }),
        ));
    }

    Ok(install_result_json(InstallResultView {
        device_serial: &device_serial,
        target: &target,
        installed_before: installed_before.as_ref(),
        action: action.as_done_str(),
        apk_source: downloaded.source,
        apk_path: Some(downloaded.path.as_path()),
        installed_after: installed_after.as_ref(),
        dry_run: false,
    }))
}

pub fn handle_uninstall(options: UninstallOptions<'_>) -> CommandResult {
    let device_serial = select_device(options.device)?;
    let installed_before = query_installed_app(&device_serial)?;
    let action = if installed_before.is_some() {
        if options.dry_run {
            "would-uninstall"
        } else {
            adb_uninstall(&device_serial)?;
            "uninstalled"
        }
    } else {
        "skipped"
    };
    let installed_after = if options.dry_run {
        installed_before.clone()
    } else if installed_before.is_some() {
        query_installed_app(&device_serial)?
    } else {
        None
    };
    if !options.dry_run && installed_after.is_some() {
        return Err(CommandError::assertion_failed_with_details(
            "uninstall reported success, but Autofish App is still installed",
            json!({
                "deviceSerial": device_serial,
                "packageName": PACKAGE_NAME,
                "installedVersionAfter": installed_after.as_ref().map(|app| app.version_name.as_str()),
            }),
        ));
    }

    Ok(json!({
        "deviceSerial": device_serial,
        "packageName": PACKAGE_NAME,
        "installedBefore": installed_before.is_some(),
        "installedVersionBefore": installed_before.as_ref().map(|app| app.version_name.as_str()),
        "installedVersionCodeBefore": installed_before.as_ref().and_then(|app| app.version_code.as_deref()),
        "action": action,
        "installedAfter": installed_after.is_some(),
        "installedVersionAfter": installed_after.as_ref().map(|app| app.version_name.as_str()),
        "dryRun": options.dry_run,
    }))
}

fn http_client() -> Result<Client, CommandError> {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| CommandError::internal(format!("failed to build HTTP client: {e}")))
}

struct InstallResultView<'a> {
    device_serial: &'a str,
    target: &'a ReleaseAsset,
    installed_before: Option<&'a InstalledApp>,
    action: &'a str,
    apk_source: &'a str,
    apk_path: Option<&'a Path>,
    installed_after: Option<&'a InstalledApp>,
    dry_run: bool,
}

fn install_result_json(view: InstallResultView<'_>) -> Value {
    json!({
        "deviceSerial": view.device_serial,
        "targetVersion": view.target.version,
        "targetTag": view.target.tag,
        "targetAsset": view.target.asset,
        "installedBefore": view.installed_before.is_some(),
        "installedVersionBefore": view.installed_before.map(|app| app.version_name.as_str()),
        "installedVersionCodeBefore": view.installed_before.and_then(|app| app.version_code.as_deref()),
        "action": view.action,
        "apkSource": view.apk_source,
        "apkPath": view.apk_path.map(|path| path.display().to_string()),
        "installedVersionAfter": view.installed_after.map(|app| app.version_name.as_str()),
        "installedVersionCodeAfter": view.installed_after.and_then(|app| app.version_code.as_deref()),
        "dryRun": view.dry_run,
    })
}

fn query_installed_app(serial: &str) -> Result<Option<InstalledApp>, CommandError> {
    let output = run_adb(
        ["-s", serial, "shell", "dumpsys", "package", PACKAGE_NAME],
        "adb shell dumpsys package failed",
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_dumpsys_package(&stdout))
}

fn adb_install(serial: &str, apk_path: &Path, allow_downgrade: bool) -> Result<(), CommandError> {
    let mut args = vec![
        "-s".to_string(),
        serial.to_string(),
        "install".to_string(),
        "-r".to_string(),
    ];
    if allow_downgrade {
        args.push("-d".to_string());
    }
    args.push(apk_path.display().to_string());
    run_adb(args, "adb install failed")?;
    Ok(())
}

fn adb_uninstall(serial: &str) -> Result<(), CommandError> {
    run_adb(
        ["-s", serial, "uninstall", PACKAGE_NAME],
        "adb uninstall failed",
    )?;
    Ok(())
}

pub fn parse_dumpsys_package(output: &str) -> Option<InstalledApp> {
    if output.contains("Unable to find package")
        || output.contains("not found")
        || !output.contains(PACKAGE_NAME)
    {
        return None;
    }
    let version_name = output
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("versionName="))
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    let version_code = output.lines().map(str::trim).find_map(|line| {
        line.strip_prefix("versionCode=").map(|value| {
            value
                .split_whitespace()
                .next()
                .unwrap_or(value)
                .trim()
                .to_string()
        })
    });
    Some(InstalledApp {
        version_name,
        version_code,
    })
}

fn decide_install_action(
    installed_version: Option<&str>,
    target_version: &str,
    force: bool,
) -> Result<InstallAction, CommandError> {
    let Some(installed_version) = installed_version else {
        return Ok(InstallAction::Install);
    };
    match compare_versions(installed_version, target_version) {
        Ordering::Equal => Ok(InstallAction::Skip),
        Ordering::Less => Ok(InstallAction::Upgrade),
        Ordering::Greater if force => Ok(InstallAction::Downgrade),
        Ordering::Greater => Err(CommandError::invalid_params(format!(
            "installed Autofish App version {installed_version} is newer than target {target_version}; pass --force to downgrade"
        ))),
    }
}

#[cfg(test)]
fn plan_install_action(
    installed_version: Option<&str>,
    target_version: &str,
    force: bool,
) -> Result<&'static str, String> {
    decide_install_action(installed_version, target_version, force)
        .map(|action| action.as_done_str())
        .map_err(|error| error.message)
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left_parts = parse_version_parts(left);
    let right_parts = parse_version_parts(right);
    left_parts.cmp(&right_parts)
}

fn parse_version_parts(version: &str) -> (Vec<u64>, bool, String) {
    let (core, suffix) = version
        .split_once('-')
        .map_or((version, ""), |(core, suffix)| (core, suffix));
    let nums = core
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect::<Vec<_>>();
    (nums, suffix.is_empty(), suffix.to_string())
}

fn resolve_release(client: &Client, requested: &str) -> Result<ReleaseAsset, CommandError> {
    match requested {
        "current" => Ok(release_for_version(env!("CARGO_PKG_VERSION"))),
        "latest" => latest_app_release(client),
        explicit => {
            validate_version(explicit)?;
            Ok(release_for_version(explicit))
        }
    }
}

pub fn release_for_version(version: &str) -> ReleaseAsset {
    let tag = format!("app-v{version}");
    let asset = format!("auto-fish-{version}-release.apk");
    ReleaseAsset {
        version: version.to_string(),
        github_url: github_download_url(&tag, &asset),
        tag,
        asset,
    }
}

fn validate_version(version: &str) -> Result<(), CommandError> {
    let core = version.split_once('-').map_or(version, |(core, _)| core);
    let parts = core.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || parts.iter().any(|part| part.parse::<u64>().is_err()) {
        return Err(CommandError::invalid_params(format!(
            "invalid app version `{version}`; expected current, latest, or semver like 0.4.0"
        )));
    }
    Ok(())
}

fn github_download_url(tag: &str, asset: &str) -> String {
    format!("{DOWNLOAD_BASE}/{tag}/{asset}")
}

fn latest_app_release(client: &Client) -> Result<ReleaseAsset, CommandError> {
    let response = client
        .get(GITHUB_RELEASES_API)
        .query(&[("per_page", "30")])
        .send()
        .map_err(|e| network_error(format!("failed to query GitHub releases: {e}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(network_error(format!(
            "failed to query GitHub releases: HTTP {status}"
        )));
    }
    let releases = response
        .json::<Vec<Value>>()
        .map_err(|e| network_error(format!("failed to parse GitHub releases response: {e}")))?;
    for release in releases {
        let tag = release
            .get("tag_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !tag.starts_with("app-v") {
            continue;
        }
        let prerelease = release
            .get("prerelease")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if prerelease || tag.contains("-rc") {
            continue;
        }
        let assets = release
            .get("assets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let asset = assets.iter().find_map(|asset| {
            let name = asset.get("name")?.as_str()?;
            if name.ends_with(".apk") && name.contains("release") {
                let url = asset.get("browser_download_url")?.as_str()?;
                Some((name.to_string(), url.to_string()))
            } else {
                None
            }
        });
        let version = tag.trim_start_matches("app-v").to_string();
        let (asset, github_url) = asset.unwrap_or_else(|| {
            let asset = format!("auto-fish-{version}-release.apk");
            let url = github_download_url(tag, &asset);
            (asset, url)
        });
        return Ok(ReleaseAsset {
            version,
            tag: tag.to_string(),
            asset,
            github_url,
        });
    }
    Err(network_error(format!(
        "no stable app release found in {GITHUB_REPO}"
    )))
}

struct DownloadedApk {
    path: PathBuf,
    source: &'static str,
}

fn resolve_apk(client: &Client, target: &ReleaseAsset) -> Result<DownloadedApk, CommandError> {
    let cache_path = cache_path_for(&target.version, &target.asset)?;
    if cache_path.exists() {
        return Ok(DownloadedApk {
            path: cache_path,
            source: "cache",
        });
    }
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| CommandError::internal(format!("failed to create APK cache: {e}")))?;
    }

    download_to_cache(client, &target.github_url, &cache_path).map_err(|github_error| {
        CommandError {
            code: ErrorCode::NetworkError,
            message: "failed to download Autofish APK from GitHub".to_string(),
            retryable: true,
            status: None,
            raw: None,
            details: Some(json!({
                "githubUrl": target.github_url,
                "githubError": github_error,
            })),
        }
    })?;
    Ok(DownloadedApk {
        path: cache_path,
        source: "github",
    })
}

fn cache_path_for(version: &str, asset: &str) -> Result<PathBuf, CommandError> {
    let base = env::var_os("AF_CACHE_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache").join("af")))
        .unwrap_or_else(|| env::temp_dir().join("af-cache"));
    Ok(base.join("app").join(version).join(asset))
}

fn download_to_cache(client: &Client, url: &str, cache_path: &Path) -> Result<(), String> {
    let response = client.get(url).send().map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let bytes = response.bytes().map_err(|e| e.to_string())?;
    let tmp_path = cache_path.with_extension("apk.tmp");
    fs::write(&tmp_path, bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, cache_path).map_err(|e| e.to_string())?;
    Ok(())
}

fn network_error(message: String) -> CommandError {
    CommandError {
        code: ErrorCode::NetworkError,
        message,
        retryable: true,
        status: None,
        raw: None,
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_release_from_version() {
        let release = release_for_version("0.4.0");
        assert_eq!(release.tag, "app-v0.4.0");
        assert_eq!(release.asset, "auto-fish-0.4.0-release.apk");
        assert_eq!(
            release.github_url,
            "https://github.com/memohai/Autofish/releases/download/app-v0.4.0/auto-fish-0.4.0-release.apk"
        );
    }

    #[test]
    fn parses_dumpsys_package_versions() {
        let app = parse_dumpsys_package(
            "Package [com.memohai.autofish] (abc):\n  versionCode=10 minSdk=26 targetSdk=35\n  versionName=0.4.0\n",
        )
        .expect("expected package");
        assert_eq!(app.version_name, "0.4.0");
        assert_eq!(app.version_code.as_deref(), Some("10"));
    }

    #[test]
    fn treats_missing_package_as_not_installed() {
        assert!(parse_dumpsys_package("Unable to find package: com.memohai.autofish").is_none());
    }

    #[test]
    fn plans_install_when_missing() {
        assert_eq!(
            plan_install_action(None, "0.4.0", false).unwrap(),
            "installed"
        );
    }

    #[test]
    fn plans_skip_same_version_by_default() {
        assert_eq!(
            plan_install_action(Some("0.4.0"), "0.4.0", false).unwrap(),
            "skipped"
        );
    }

    #[test]
    fn plans_upgrade_from_old_version() {
        assert_eq!(
            plan_install_action(Some("0.3.0"), "0.4.0", false).unwrap(),
            "upgraded"
        );
    }

    #[test]
    fn rejects_downgrade_without_force() {
        let err = plan_install_action(Some("0.5.0"), "0.4.0", false).unwrap_err();
        assert!(err.contains("pass --force"));
    }

    #[test]
    fn plans_downgrade_with_force() {
        assert_eq!(
            plan_install_action(Some("0.5.0"), "0.4.0", true).unwrap(),
            "downgraded"
        );
    }
}
