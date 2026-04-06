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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProxyMode {
    System,
    Direct,
    Auto,
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
    #[arg(long = "no-memory", alias = "no-trace", default_value_t = false)]
    pub no_memory: bool,

    #[arg(
        long,
        env = "AF_DB",
        default_value = "af.db",
        value_hint = clap::ValueHint::FilePath
    )]
    pub memory_db: PathBuf,

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
    pub url: String,

    #[arg(long, default_value_t = 10000)]
    pub timeout_ms: u64,

    #[arg(long, value_enum, default_value_t = ProxyMode::Auto)]
    pub proxy: ProxyMode,
}

#[derive(Args, Debug, Clone)]
pub struct AuthedRemoteOpts {
    #[arg(long, env = "AF_URL")]
    pub url: String,

    #[arg(long, env = "AF_TOKEN")]
    pub token: String,

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
    #[command(
        name = "back",
        about = "Press Android Back",
        long_about = "Press Android Back once."
    )]
    Back,
    #[command(
        name = "home",
        about = "Press Android Home",
        long_about = "Press Android Home once."
    )]
    Home,
    #[command(
        name = "text",
        about = "Input text",
        long_about = "Input text to the current focused field."
    )]
    Text {
        #[arg(long)]
        text: String,
    },
    #[command(
        name = "launch",
        about = "Launch an app by package name",
        long_about = "Launch an app by package name."
    )]
    Launch {
        #[arg(long = "package")]
        package_name: String,
    },
    #[command(
        name = "stop",
        about = "Stop an app by package name",
        long_about = "Stop (force-stop) an app by package name."
    )]
    Stop {
        #[arg(long = "package")]
        package_name: String,
    },
    #[command(
        name = "key",
        about = "Press a key by Android key code",
        long_about = "Press a key by Android key code."
    )]
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
    #[command(
        name = "refs",
        about = "Observe server-generated clickable refs with aliases",
        long_about = "Observe server-generated clickable refs with aliases (`@n1`, `@n2`, ...).\nUse returned ref alias as `act tap --by ref --value <ref>`."
    )]
    Refs {
        #[arg(long = "max-rows", default_value_t = 120)]
        max_rows: usize,
    },
    #[command(
        name = "page",
        about = "Atomic page observation: top + screen + refs in one call",
        long_about = "Observe page state atomically. Always returns base metadata; topActivity may be null when a stable value cannot be determined.\nUse --field to select additional data slices (default: screen). All data comes from the same point-in-time capture."
    )]
    Page {
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
        #[arg(long, default_value_t = true)]
        ignore_case: bool,
    },
    #[command(name = "top-activity")]
    TopActivity {
        #[arg(long)]
        expected: String,
        #[arg(long, default_value = "contains")]
        mode: String,
    },
    #[command(
        name = "node-exists",
        about = "Verify a node exists by text/content_desc/resource_id/class_name",
        long_about = "Verify a node exists by text/content_desc/resource_id/class_name.\n\nAliases: `desc` -> `content_desc`, `class` -> `class_name`.\nIf the screen is WebView-heavy (`hasWebView=true` or `nodeReliability=low`), prefer `verify text-contains` first."
    )]
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
        #[arg(long)]
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
        #[arg(long)]
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
    fn remote_commands_require_url() {
        let result = Cli::try_parse_from(["af", "observe", "top"]);
        assert!(result.is_err());
    }

    #[test]
    fn authed_remote_commands_require_token() {
        let result = Cli::try_parse_from([
            "af",
            "verify",
            "--url",
            "http://127.0.0.1:18080",
            "text-contains",
            "--text",
            "x",
        ]);
        assert!(result.is_err());
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
}
