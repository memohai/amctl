mod api;
mod builder;
mod cli;
mod commands;
mod core;
mod db;
mod memory;
mod memory_recording;
mod output;
mod runner;

use crate::builder::ReqClientBuilder;
use crate::cli::Cli;
use crate::memory::MemoryStore;
use crate::runner::{persist_memory, run_command, run_memory_command};
use clap::Parser;
use crossbeam_channel::{Receiver, bounded, select};
use std::process;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let ctrl_c_events = ctrl_channel()?;
    let cli = Cli::parse();
    let memory_store = if cli.no_memory {
        None
    } else {
        Some(MemoryStore::new(cli.memory_db.clone())?)
    };
    let started = Instant::now();
    let result = match &cli.command {
        crate::cli::Commands::Health { remote } => {
            let runtime = ReqClientBuilder::new(
                remote.url.trim_end_matches('/').to_string(),
                remote.timeout_ms,
                remote.proxy,
            );
            let client = runtime.build()?;
            let result = run_command(&client, &runtime, &ctrl_c_events, &cli);
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
                remote.url.trim_end_matches('/').to_string(),
                remote.timeout_ms,
                remote.proxy,
            )
            .with_token(Some(remote.token.clone()));
            let client = runtime.build()?;
            let result = run_command(&client, &runtime, &ctrl_c_events, &cli);
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
    };
    println!("{}", serde_json::to_string(&result)?);

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
