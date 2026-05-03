use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, ValueEnum};

fn parse_non_negative_f32(v: &str) -> Result<f32, String> {
    let parsed = v
        .parse::<f32>()
        .map_err(|_| format!("invalid float value: {v}"))?;
    if parsed < 0.0 {
        return Err(format!("value must be >= 0, got {parsed}"));
    }
    Ok(parsed)
}

fn parse_non_negative_points2(v: &str) -> Result<[f32; 2], String> {
    let parts = v.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(format!(
            "points must contain exactly 2 comma-separated values: x,y; got '{v}'"
        ));
    }
    let x = parse_non_negative_f32(parts[0])?;
    let y = parse_non_negative_f32(parts[1])?;
    Ok([x, y])
}

fn parse_app_install_version(v: &str) -> Result<String, String> {
    let value = v.trim();
    if matches!(value, "current" | "latest") || is_semver_like(value) {
        return Ok(value.to_string());
    }
    Err(format!(
        "version must be current, latest, or an explicit semver like 0.4.0; got '{v}'"
    ))
}

fn is_semver_like(value: &str) -> bool {
    let core = value.split_once('-').map_or(value, |(core, _)| core);
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn parse_tcp_port(v: &str) -> Result<u16, String> {
    let port = v
        .parse::<u16>()
        .map_err(|_| format!("invalid TCP port: {v}"))?;
    if port == 0 {
        return Err("TCP port must be between 1 and 65535".to_string());
    }
    Ok(port)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProxyMode {
    System,
    Direct,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenFieldArg {
    Id,
    Class,
    Text,
    Desc,
    #[value(name = "resId")]
    ResId,
    Flags,
    Bounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PageSliceArg {
    Screen,
    Refs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MarkScope {
    All,
    Interactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RefreshMode {
    On,
    Off,
}

#[derive(Parser, Debug)]
#[command(name = "af", about = "Deterministic executor for Autofish REST API")]
pub struct Cli {
    #[arg(long, env = "AF_CONFIG", value_hint = clap::ValueHint::FilePath)]
    pub config: Option<PathBuf>,

    #[arg(long, env = "AF_OUTPUT", value_enum)]
    pub output: Option<OutputFormat>,

    #[arg(long = "no-memory", alias = "no-trace", default_value_t = false)]
    pub no_memory: bool,

    #[arg(
        long,
        env = "AF_DB",
        value_hint = clap::ValueHint::FilePath
    )]
    pub memory_db: Option<PathBuf>,

    #[arg(
        long,
        default_value = "default",
        help = "Logical task session used to group memory across multiple CLI calls."
    )]
    pub session: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct HealthRemoteOpts {
    #[arg(long, env = "AF_URL")]
    pub url: Option<String>,

    #[arg(long, default_value_t = 10000)]
    pub timeout_ms: u64,

    #[arg(long, value_enum, default_value_t = ProxyMode::Auto)]
    pub proxy: ProxyMode,
}

#[derive(Args, Debug, Clone)]
pub struct AuthedRemoteOpts {
    #[arg(long, env = "AF_URL")]
    pub url: Option<String>,

    #[arg(long, env = "AF_TOKEN")]
    pub token: Option<String>,

    #[arg(long, default_value_t = 10000)]
    pub timeout_ms: u64,

    #[arg(long, value_enum, default_value_t = ProxyMode::Auto)]
    pub proxy: ProxyMode,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    #[command(name = "health", about = "Check service health")]
    Health {
        #[command(flatten)]
        remote: HealthRemoteOpts,
    },
    #[command(name = "act", about = "Run one device action")]
    Act {
        #[command(flatten)]
        remote: AuthedRemoteOpts,
        #[command(subcommand)]
        command: ActCommands,
    },
    #[command(name = "observe", about = "Observe device state")]
    Observe {
        #[command(flatten)]
        remote: AuthedRemoteOpts,
        #[command(subcommand)]
        command: ObserveCommands,
    },
    #[command(name = "verify", about = "Verify expected state")]
    Verify {
        #[command(flatten)]
        remote: AuthedRemoteOpts,
        #[command(subcommand)]
        command: VerifyCommands,
    },
    #[command(
        name = "memory",
        about = "Query or inspect agent-oriented local memory"
    )]
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },
    #[command(name = "recover", about = "Run simple recovery actions")]
    Recover {
        #[command(flatten)]
        remote: AuthedRemoteOpts,
        #[command(subcommand)]
        command: RecoverCommands,
    },
    #[command(name = "config", about = "Read or update local af config")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    #[command(name = "app", about = "Manage the official Autofish Android app")]
    App {
        #[command(subcommand)]
        command: AppCommands,
    },
    #[command(name = "connect", about = "Configure local connectivity to Autofish")]
    Connect {
        #[command(subcommand)]
        command: ConnectCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum AppCommands {
    #[command(name = "install", about = "Install the official Autofish App with adb")]
    Install {
        #[arg(
            long,
            env = "ANDROID_SERIAL",
            value_name = "ADB_SERIAL",
            help = "ADB device serial."
        )]
        device: Option<String>,
        #[arg(
            long,
            value_name = "VERSION",
            default_value = "current",
            value_parser = parse_app_install_version,
            help = "App version: current | latest | semver."
        )]
        version: String,
        #[arg(
            long,
            default_value_t = false,
            help = "Allow downgrading to the target version."
        )]
        force: bool,
        #[arg(
            long,
            default_value_t = false,
            help = "Show the plan without downloading or installing."
        )]
        dry_run: bool,
    },
    #[command(
        name = "uninstall",
        about = "Uninstall the official Autofish App with adb"
    )]
    Uninstall {
        #[arg(
            long,
            env = "ANDROID_SERIAL",
            value_name = "ADB_SERIAL",
            help = "ADB device serial."
        )]
        device: Option<String>,
        #[arg(
            long,
            default_value_t = false,
            help = "Show the plan without uninstalling."
        )]
        dry_run: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ConnectCommands {
    #[command(name = "usb", about = "Connect to Autofish over USB")]
    Usb {
        #[arg(
            long,
            env = "ANDROID_SERIAL",
            value_name = "ADB_SERIAL",
            help = "ADB device serial."
        )]
        device: Option<String>,
        #[arg(
            long = "local-port",
            value_name = "PORT",
            value_parser = parse_tcp_port,
            help = "Preferred local TCP port for adb forward."
        )]
        local_port: Option<u16>,
        #[arg(
            long,
            default_value_t = false,
            help = "Print the plan without forwarding or writing config."
        )]
        print_only: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ActCommands {
    #[command(
        name = "tap",
        about = "Tap by coordinates (`--xy`) or semantic selector (`--by/--value`)"
    )]
    #[command(group(
        ArgGroup::new("tap_mode")
            .required(true)
            .args(["xy", "by"])
    ))]
    Tap {
        #[arg(
            long,
            value_name = "X,Y",
            help = "Coordinates tuple (>= 0), for example: --xy 540,1200.",
            value_parser = parse_non_negative_points2,
            conflicts_with_all = ["by", "value"]
        )]
        xy: Option<[f32; 2]>,
        #[arg(
            long,
            help = "Semantic selector: text | desc | resid | ref.",
            requires = "value",
            conflicts_with = "xy"
        )]
        by: Option<String>,
        #[arg(
            long,
            help = "Selector value. For --by ref, pass alias like @n1 from `observe refs`.",
            requires = "by",
            conflicts_with = "xy"
        )]
        value: Option<String>,
        #[arg(
            long,
            help = "Use exact match in semantic mode (--by/--value).",
            default_value_t = false
        )]
        exact_match: bool,
    },
    #[command(name = "swipe", about = "Swipe from one coordinate to another")]
    Swipe {
        #[arg(
            long = "from",
            value_name = "X,Y",
            help = "Start coordinate (>= 0), for example: --from 100,1200.",
            value_parser = parse_non_negative_points2
        )]
        from: [f32; 2],
        #[arg(
            long = "to",
            value_name = "X,Y",
            help = "End coordinate (>= 0), for example: --to 900,1200.",
            value_parser = parse_non_negative_points2
        )]
        to: [f32; 2],
        #[arg(
            long,
            help = "Swipe duration in milliseconds (ms).",
            default_value_t = 300
        )]
        duration: i64,
    },
    #[command(name = "back", about = "Press Android Back")]
    Back,
    #[command(name = "home", about = "Press Android Home")]
    Home,
    #[command(name = "text", about = "Input text")]
    Text {
        #[arg(long)]
        text: String,
    },
    #[command(name = "launch", about = "Launch an app by package name")]
    Launch {
        #[arg(long = "package")]
        package_name: String,
    },
    #[command(name = "stop", about = "Stop an app by package name")]
    Stop {
        #[arg(long = "package")]
        package_name: String,
    },
    #[command(name = "key", about = "Press a key by Android key code")]
    Key {
        #[arg(long = "key-code")]
        key_code: i32,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ObserveCommands {
    #[command(name = "screen", about = "Observe current UI tree snapshot")]
    Screen {
        #[arg(
            long,
            help = "Return full snapshot (rows + raw). Cannot be used with --max-rows or --field."
        )]
        full: bool,
        #[arg(
            long = "save-file",
            value_hint = clap::ValueHint::FilePath,
            help = "Save full screen payload as artifact JSON."
        )]
        save_file: Option<PathBuf>,
        #[arg(
            long = "max-rows",
            help = "Maximum returned rows in compact mode. Default is 120.",
            conflicts_with = "full"
        )]
        max_rows: Option<usize>,
        #[arg(
            long = "field",
            value_name = "FIELD",
            value_enum,
            help = "Field to include in compact mode. Repeatable (for example: --field id --field text).",
            conflicts_with = "full"
        )]
        fields: Vec<ScreenFieldArg>,
    },
    #[command(
        name = "overlay",
        about = "Get or set server-side accessibility overlay state"
    )]
    Overlay {
        #[command(subcommand)]
        command: OverlayCommands,
    },
    #[command(name = "screenshot", about = "Capture a compressed screenshot")]
    Screenshot {
        #[arg(
            long = "save-file",
            value_hint = clap::ValueHint::FilePath,
            help = "Save screenshot image to this file instead of using the default artifact path."
        )]
        save_file: Option<PathBuf>,
        #[arg(
            long = "max-dim",
            value_parser = clap::value_parser!(i64).range(1..),
            default_value_t = 700,
            help = "Limit long edge of the image in pixels (>= 1)."
        )]
        max_dim: i64,
        #[arg(
            long,
            value_parser = clap::value_parser!(i64).range(1..=100),
            default_value_t = 80,
            help = "JPEG quality from 1 to 100."
        )]
        quality: i64,
        #[arg(
            long,
            default_value_t = false,
            help = "Include overlay marks in screenshot."
        )]
        annotate: bool,
        #[arg(
            long,
            default_value_t = false,
            requires = "annotate",
            help = "Temporarily hide on-screen overlay while rendering marks."
        )]
        hide_overlay: bool,
        #[arg(
            long = "max-marks",
            requires = "annotate",
            help = "Maximum marks when --annotate is enabled. Default is 120."
        )]
        max_marks: Option<usize>,
        #[arg(
            long = "mark-scope",
            value_enum,
            requires = "annotate",
            help = "Mark scope when --annotate is enabled: all or interactive."
        )]
        mark_scope: Option<MarkScope>,
    },
    #[command(name = "top")]
    Top,
    #[command(name = "refs", about = "Observe clickable refs with aliases")]
    Refs {
        #[arg(long = "max-rows", default_value_t = 120)]
        max_rows: usize,
    },
    #[command(name = "page", about = "Observe top + screen + refs")]
    Page {
        #[arg(
            long = "save-dir",
            value_hint = clap::ValueHint::DirPath,
            help = "Directory for large page artifacts."
        )]
        save_dir: Option<PathBuf>,
        #[arg(
            long = "field",
            value_name = "FIELD",
            value_enum,
            help = "Data slice to include. Repeatable: --field screen --field refs. Default: screen."
        )]
        fields: Vec<PageSliceArg>,
        #[arg(long = "max-rows", default_value_t = 120)]
        max_rows: usize,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum OverlayCommands {
    #[command(name = "get", about = "Get current overlay state")]
    Get,
    #[command(name = "set", about = "Set overlay state and behavior")]
    #[command(group(
        ArgGroup::new("overlay_state")
            .required(true)
            .args(["enable", "disable"])
    ))]
    Set {
        #[arg(long, help = "Enable overlay.")]
        enable: bool,
        #[arg(long, help = "Disable overlay.")]
        disable: bool,
        #[arg(
            long = "max-marks",
            default_value_t = 300,
            help = "Maximum overlay marks."
        )]
        max_marks: usize,
        #[arg(
            long = "mark-scope",
            value_enum,
            default_value_t = MarkScope::All,
            help = "Overlay mark scope: all or interactive."
        )]
        mark_scope: MarkScope,
        #[arg(
            long = "refresh",
            value_enum,
            default_value_t = RefreshMode::On,
            help = "Overlay auto refresh: on or off."
        )]
        refresh: RefreshMode,
        #[arg(
            long = "refresh-interval-ms",
            help = "Refresh interval in milliseconds. Used when --refresh on."
        )]
        refresh_interval_ms: Option<u64>,
        #[arg(long = "offset-x", help = "Overlay horizontal offset in pixels.")]
        offset_x: Option<i32>,
        #[arg(long = "offset-y", help = "Overlay vertical offset in pixels.")]
        offset_y: Option<i32>,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum VerifyCommands {
    #[command(name = "text-contains")]
    TextContains {
        #[arg(long)]
        text: String,
        #[arg(long = "case-sensitive", default_value_t = false)]
        case_sensitive: bool,
    },
    #[command(name = "top-activity")]
    TopActivity {
        #[arg(long)]
        expected: String,
        #[arg(long, default_value = "contains")]
        mode: String,
    },
    #[command(name = "node-exists", about = "Verify a node exists")]
    NodeExists {
        #[arg(long)]
        by: String,
        #[arg(long)]
        value: String,
        #[arg(long, default_value_t = false)]
        exact_match: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum MemoryCommands {
    #[command(name = "save", about = "Save a knowledge note (append-only)")]
    Save {
        #[arg(long, default_value = "")]
        app: String,
        #[arg(long)]
        topic: String,
        #[arg(long)]
        content: String,
    },
    #[command(
        name = "search",
        about = "Search notes by app, topic prefix, or keyword"
    )]
    Search {
        #[arg(long)]
        app: Option<String>,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    #[command(name = "delete", about = "Delete a note by id")]
    Delete {
        #[arg(long)]
        id: i64,
    },
    #[command(name = "log", about = "Query the event log")]
    Log {
        #[arg(long = "for-session")]
        session: Option<String>,
        #[arg(long)]
        app: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    #[command(name = "stats", about = "Show event statistics")]
    Stats {
        #[arg(long = "for-session")]
        session: Option<String>,
    },
    #[command(
        name = "experience",
        about = "Query past transitions and recoveries (three-tier: page → activity → app)"
    )]
    Experience {
        #[arg(long, default_value = "")]
        app: String,
        #[arg(long, default_value = "")]
        activity: String,
        #[arg(long = "page-fp", default_value = "")]
        page_fingerprint: String,
        #[arg(long = "failure-cause")]
        failure_cause: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    #[command(name = "context", about = "Show current session observation cache")]
    Context,
}

#[derive(clap::Subcommand, Debug)]
pub enum ConfigCommands {
    #[command(
        name = "list",
        about = "List effective config values and their sources"
    )]
    List,
    #[command(name = "get", about = "Get one config key")]
    Get {
        #[arg()]
        key: String,
    },
    #[command(name = "set", about = "Write one config key into the config file")]
    Set {
        #[arg()]
        key: String,
        #[arg()]
        value: String,
    },
    #[command(name = "unset", about = "Remove one config key from the config file")]
    Unset {
        #[arg()]
        key: String,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum RecoverCommands {
    #[command(name = "back")]
    Back {
        #[arg(long, default_value_t = 1)]
        times: u32,
    },
    #[command(name = "home")]
    Home,
    #[command(name = "relaunch")]
    Relaunch {
        #[arg(long = "package")]
        package_name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn memory_commands_parse_without_url() {
        let cli = Cli::parse_from(["af", "--session", "demo", "memory", "log", "--limit", "5"]);
        assert!(matches!(cli.command, Commands::Memory { .. }));
    }

    #[test]
    fn app_command_install_parses_as_local_command() {
        let cli = Cli::parse_from([
            "af",
            "app",
            "install",
            "--device",
            "RFCX123456",
            "--dry-run",
        ]);
        match cli.command {
            Commands::App {
                command:
                    AppCommands::Install {
                        device,
                        version,
                        dry_run,
                        ..
                    },
            } => {
                assert_eq!(device.as_deref(), Some("RFCX123456"));
                assert_eq!(version, "current");
                assert!(dry_run);
            }
            _ => panic!("expected app install command"),
        }
    }

    #[test]
    fn app_command_uninstall_parses_as_local_command() {
        let cli = Cli::parse_from([
            "af",
            "app",
            "uninstall",
            "--device",
            "RFCX123456",
            "--dry-run",
        ]);
        match cli.command {
            Commands::App {
                command: AppCommands::Uninstall { device, dry_run },
            } => {
                assert_eq!(device.as_deref(), Some("RFCX123456"));
                assert!(dry_run);
            }
            _ => panic!("expected app uninstall command"),
        }
    }

    #[test]
    fn app_install_accepts_latest_and_semver_versions() {
        for version in ["latest", "0.4.0", "0.4.0-rc.1"] {
            let cli = Cli::parse_from(["af", "app", "install", "--version", version, "--dry-run"]);
            match cli.command {
                Commands::App {
                    command:
                        AppCommands::Install {
                            version: parsed, ..
                        },
                } => assert_eq!(parsed, version),
                _ => panic!("expected app install command"),
            }
        }
    }

    #[test]
    fn app_install_rejects_invalid_version() {
        let err = Cli::try_parse_from(["af", "app", "install", "--version", "banana"])
            .expect_err("invalid version should fail");
        assert!(err.to_string().contains("version must be current"));
    }

    #[test]
    fn connect_usb_parses_as_local_command() {
        let cli = Cli::parse_from([
            "af",
            "connect",
            "usb",
            "--device",
            "RFCX123456",
            "--local-port",
            "18081",
            "--print-only",
        ]);
        match cli.command {
            Commands::Connect {
                command:
                    ConnectCommands::Usb {
                        device,
                        local_port,
                        print_only,
                    },
            } => {
                assert_eq!(device.as_deref(), Some("RFCX123456"));
                assert_eq!(local_port, Some(18081));
                assert!(print_only);
            }
            _ => panic!("expected connect usb command"),
        }
    }

    #[test]
    fn connect_usb_rejects_zero_local_port() {
        let err = Cli::try_parse_from(["af", "connect", "usb", "--local-port", "0"])
            .expect_err("port 0 should fail");
        assert!(err.to_string().contains("TCP port must be between"));
    }

    #[test]
    fn remote_commands_allow_missing_url_for_later_resolution() {
        let cli = Cli::parse_from(["af", "observe", "top"]);
        match cli.command {
            Commands::Observe { remote, .. } => assert!(remote.url.is_none()),
            _ => panic!("expected observe command"),
        }
    }

    #[test]
    fn authed_remote_commands_allow_missing_token_for_later_resolution() {
        let cli = Cli::parse_from([
            "af",
            "verify",
            "--url",
            "http://127.0.0.1:18080",
            "text-contains",
            "--text",
            "x",
        ]);
        match cli.command {
            Commands::Verify { remote, .. } => {
                assert_eq!(remote.url.as_deref(), Some("http://127.0.0.1:18080"));
                assert!(remote.token.is_none());
            }
            _ => panic!("expected verify command"),
        }
    }

    #[test]
    fn health_requires_token() {
        let cli = Cli::parse_from(["af", "health", "--url", "http://127.0.0.1:18080"]);
        assert!(matches!(cli.command, Commands::Health { .. }));
    }

    #[test]
    fn health_rejects_token_flag() {
        let result = Cli::try_parse_from([
            "af",
            "health",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn old_global_url_shape_is_rejected_for_memory() {
        let result =
            Cli::try_parse_from(["af", "--url", "http://127.0.0.1:18080", "memory", "log"]);
        assert!(result.is_err());
    }

    #[test]
    fn remote_url_parses_under_subcommand() {
        let cli = Cli::parse_from([
            "af",
            "observe",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "page",
        ]);
        assert!(matches!(cli.command, Commands::Observe { .. }));
    }

    #[test]
    fn text_contains_supports_case_sensitive_matching() {
        let cli = Cli::parse_from([
            "af",
            "verify",
            "text-contains",
            "--text",
            "Settings",
            "--case-sensitive",
        ]);
        match cli.command {
            Commands::Verify {
                command: VerifyCommands::TextContains { case_sensitive, .. },
                ..
            } => {
                assert!(case_sensitive);
            }
            _ => panic!("expected verify text-contains command"),
        }
    }

    #[test]
    fn text_contains_rejects_legacy_ignore_case_bool() {
        let result = Cli::try_parse_from([
            "af",
            "verify",
            "text-contains",
            "--text",
            "Settings",
            "--ignore-case=false",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn memory_log_accepts_for_session_filter() {
        let cli = Cli::parse_from(["af", "memory", "log", "--for-session", "wf-1"]);
        match cli.command {
            Commands::Memory {
                command: MemoryCommands::Log { session, .. },
            } => assert_eq!(session.as_deref(), Some("wf-1")),
            _ => panic!("expected memory log command"),
        }
    }

    #[test]
    fn memory_log_rejects_session_filter() {
        let result = Cli::try_parse_from(["af", "memory", "log", "--session", "wf-1"]);
        assert!(result.is_err());
    }

    #[test]
    fn memory_stats_accepts_for_session_filter() {
        let cli = Cli::parse_from(["af", "memory", "stats", "--for-session", "wf-1"]);
        match cli.command {
            Commands::Memory {
                command: MemoryCommands::Stats { session },
            } => assert_eq!(session.as_deref(), Some("wf-1")),
            _ => panic!("expected memory stats command"),
        }
    }

    #[test]
    fn memory_stats_rejects_session_filter() {
        let result = Cli::try_parse_from(["af", "memory", "stats", "--session", "wf-1"]);
        assert!(result.is_err());
    }
}
