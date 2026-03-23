use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProxyMode {
    System,
    Direct,
    Auto,
}

#[derive(Parser, Debug)]
#[command(name = "amc", about = "Deterministic executor for amctl REST API")]
pub struct Cli {
    #[arg(long)]
    pub url: String,

    #[arg(long)]
    pub token: Option<String>,

    #[arg(long, default_value_t = 10000)]
    pub timeout_ms: u64,

    #[arg(long, value_enum, default_value_t = ProxyMode::Auto)]
    pub proxy: ProxyMode,

    #[arg(long = "no-trace", default_value_t = false)]
    pub no_trace: bool,

    #[arg(long, default_value = "amc.db", value_hint = clap::ValueHint::FilePath)]
    pub trace_db: PathBuf,

    #[arg(long, default_value = "default")]
    pub session: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    #[command(name = "health")]
    Health,
    #[command(name = "act")]
    Act {
        #[command(subcommand)]
        command: ActCommands,
    },
    #[command(name = "observe")]
    Observe {
        #[command(subcommand)]
        command: ObserveCommands,
    },
    #[command(name = "verify")]
    Verify {
        #[command(subcommand)]
        command: VerifyCommands,
    },
    #[command(name = "recover")]
    Recover {
        #[command(subcommand)]
        command: RecoverCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ActCommands {
    #[command(name = "tap")]
    Tap {
        #[arg(long)]
        x: f32,
        #[arg(long)]
        y: f32,
    },
    #[command(name = "swipe")]
    Swipe {
        #[arg(long)]
        x1: f32,
        #[arg(long)]
        y1: f32,
        #[arg(long)]
        x2: f32,
        #[arg(long)]
        y2: f32,
        #[arg(long, default_value_t = 300)]
        duration: i64,
    },
    #[command(name = "back")]
    Back,
    #[command(name = "home")]
    Home,
    #[command(name = "text")]
    Text {
        #[arg(long)]
        text: String,
    },
    #[command(name = "launch")]
    Launch {
        #[arg(long = "package")]
        package_name: String,
    },
    #[command(name = "stop")]
    Stop {
        #[arg(long = "package")]
        package_name: String,
    },
    #[command(name = "key")]
    Key {
        #[arg(long = "key-code")]
        key_code: i32,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ObserveCommands {
    #[command(name = "screen")]
    Screen {
        #[arg(long, default_value_t = false)]
        full: bool,
        #[arg(long = "max-rows", default_value_t = 120)]
        max_rows: usize,
        #[arg(long, default_value = "id,class,text,desc,resId,flags")]
        fields: String,
    },
    #[command(name = "screenshot")]
    Screenshot {
        #[arg(long = "max-dim", default_value_t = 700)]
        max_dim: i64,
        #[arg(long, default_value_t = 80)]
        quality: i64,
    },
    #[command(name = "top")]
    Top,
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
    #[command(name = "node-exists")]
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
