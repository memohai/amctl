mod api;
mod artifact;
mod builder;
mod cli;
mod commands;
mod config;
mod core;
mod db;
mod memory;
mod memory_recording;
mod output;
mod runner;

use crate::builder::ReqClientBuilder;
use crate::cli::Cli;
use crate::config::{require_token, require_url, resolve_settings};
use crate::memory::MemoryStore;
use crate::output::render_output;
use crate::runner::{
    persist_memory, run_app_command, run_command, run_config_command, run_connect_command,
    run_memory_command,
};
use clap::Parser;
use crossbeam_channel::{Receiver, bounded, select};
use std::process;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let ctrl_c_events = ctrl_channel()?;
    let cli = Cli::parse();
    let settings = resolve_settings(&cli)?;
    let memory_store = if cli.no_memory
        || matches!(
            cli.command,
            crate::cli::Commands::Config { .. }
                | crate::cli::Commands::App { .. }
                | crate::cli::Commands::Connect { .. }
        ) {
        None
    } else {
        Some(MemoryStore::new(settings.memory_db.clone())?)
    };
    let started = Instant::now();
    let result = match &cli.command {
        crate::cli::Commands::Health { remote } => {
            let runtime = ReqClientBuilder::new(
                require_url(&settings)?.trim_end_matches('/').to_string(),
                remote.timeout_ms,
                remote.proxy,
            );
            let client = runtime.build()?;
            let result = run_command(
                &client,
                &runtime,
                &ctrl_c_events,
                &cli,
                &settings,
                memory_store.as_ref(),
            );
            persist_memory(
                &memory_store,
                &cli,
                &runtime.invocation_id,
                &result,
                started.elapsed().as_millis(),
            );
            result
        }
        crate::cli::Commands::Act { remote, .. }
        | crate::cli::Commands::Observe { remote, .. }
        | crate::cli::Commands::Verify { remote, .. }
        | crate::cli::Commands::Recover { remote, .. } => {
            let runtime = ReqClientBuilder::new(
                require_url(&settings)?.trim_end_matches('/').to_string(),
                remote.timeout_ms,
                remote.proxy,
            )
            .with_token(Some(require_token(&settings)?.to_string()));
            let client = runtime.build()?;
            let result = run_command(
                &client,
                &runtime,
                &ctrl_c_events,
                &cli,
                &settings,
                memory_store.as_ref(),
            );
            persist_memory(
                &memory_store,
                &cli,
                &runtime.invocation_id,
                &result,
                started.elapsed().as_millis(),
            );
            result
        }
        crate::cli::Commands::Memory { .. } => {
            let invocation_id = crate::builder::new_invocation_id();
            run_memory_command(&invocation_id, &cli, memory_store.as_ref())
        }
        crate::cli::Commands::Config { .. } => {
            let invocation_id = crate::builder::new_invocation_id();
            run_config_command(&invocation_id, &cli, &settings)
        }
        crate::cli::Commands::App { .. } => {
            let invocation_id = crate::builder::new_invocation_id();
            run_app_command(&invocation_id, &cli)
        }
        crate::cli::Commands::Connect { .. } => {
            let invocation_id = crate::builder::new_invocation_id();
            run_connect_command(&invocation_id, &cli, &settings)
        }
    };
    println!("{}", render_output(&result, settings.output)?);

    let exit_code = match result.get("status").and_then(|value| value.as_str()) {
        Some("ok") => 0,
        Some("interrupted") => 130,
        _ => 1,
    };
    if exit_code != 0 {
        process::exit(exit_code);
    }
    Ok(())
}

pub(crate) fn run_with_interrupt<T, F>(ctrl_c_events: &Receiver<()>, work: F) -> anyhow::Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    let (done_tx, done_rx) = bounded::<anyhow::Result<T>>(1);
    std::thread::spawn(move || {
        let _ = done_tx.send(work());
    });

    select! {
        recv(ctrl_c_events) -> _ => Err(anyhow::anyhow!("Interrupted by SIGINT (Ctrl+C)")),
        recv(done_rx) -> msg => {
            match msg {
                Ok(res) => res,
                Err(_) => Err(anyhow::anyhow!("worker channel closed unexpectedly")),
            }
        }
    }
}

fn ctrl_channel() -> Result<Receiver<()>, ctrlc::Error> {
    let (sender, receiver) = bounded(100);
    ctrlc::set_handler(move || {
        let _ = sender.send(());
    })?;
    Ok(receiver)
}
