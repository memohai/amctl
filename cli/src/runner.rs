use crate::api::request::ApiClient;
use crate::artifact::ArtifactManager;
use crate::builder::ReqClientBuilder;
use crate::cli::{
    ActCommands, AppCommands, Cli, Commands, ConfigCommands, ConnectCommands, MemoryCommands,
    ObserveCommands, OverlayCommands, RecoverCommands, VerifyCommands,
};
use crate::commands::observe::ScreenshotOptions;
use crate::commands::{
    act, app, common::OverlaySetOptions, connect, memory, observe, recover, verify,
};
use crate::config::{
    ConfigSource, ResolvedSettings, config_value_for_output, get_entry, list_entries_map, set_key,
    unset_key,
};
use crate::memory::MemoryStore;
use crate::memory_recording::{
    record_event_and_close, should_record_event, should_update_session_cache, update_session_cache,
};
use crate::output::into_output;
use crossbeam_channel::Receiver;
use reqwest::blocking::Client;
use serde_json::Value;

pub fn run_command(
    client: &Client,
    runtime: &ReqClientBuilder,
    ctrl_c_events: &Receiver<()>,
    cli: &Cli,
    settings: &ResolvedSettings,
    memory_store: Option<&MemoryStore>,
) -> Value {
    let api = ApiClient::new(
        client,
        runtime.base_url.as_str(),
        runtime.token.as_deref(),
        ctrl_c_events,
    );
    let artifacts = ArtifactManager {
        store: memory_store,
        session: &cli.session,
        invocation_id: &runtime.invocation_id,
        artifact_dir: &settings.artifact_dir,
        screen_file: settings.screen_file.as_deref(),
        screenshot_file: settings.screenshot_file.as_deref(),
        page_dir: &settings.page_dir,
    };
    let command = &cli.command;

    match command {
        Commands::Health { .. } => into_output(
            &runtime.invocation_id,
            "health",
            "health",
            observe_health(&api, settings),
        ),
        Commands::Act { command, .. } => match command {
            ActCommands::Tap {
                xy,
                by,
                value,
                exact_match,
            } => into_output(
                &runtime.invocation_id,
                "act",
                "tap",
                act::handle_tap(
                    &api,
                    *xy,
                    by.as_ref().map(|v| v.as_str()),
                    value.as_ref().map(|v| v.as_str()),
                    *exact_match,
                ),
            ),
            ActCommands::Swipe { from, to, duration } => into_output(
                &runtime.invocation_id,
                "act",
                "swipe",
                act::handle_swipe(&api, from[0], from[1], to[0], to[1], *duration),
            ),
            ActCommands::Back => into_output(
                &runtime.invocation_id,
                "act",
                "back",
                act::handle_back(&api),
            ),
            ActCommands::Home => into_output(
                &runtime.invocation_id,
                "act",
                "home",
                act::handle_home(&api),
            ),
            ActCommands::Text { text } => into_output(
                &runtime.invocation_id,
                "act",
                "text",
                act::handle_text(&api, text),
            ),
            ActCommands::Launch { package_name } => into_output(
                &runtime.invocation_id,
                "act",
                "launch",
                act::handle_launch(&api, package_name),
            ),
            ActCommands::Stop { package_name } => into_output(
                &runtime.invocation_id,
                "act",
                "stop",
                act::handle_stop(&api, package_name),
            ),
            ActCommands::Key { key_code } => into_output(
                &runtime.invocation_id,
                "act",
                "key",
                act::handle_key(&api, *key_code),
            ),
        },
        Commands::Observe { command, .. } => match command {
            ObserveCommands::Screen {
                full,
                save_file,
                max_rows,
                fields,
            } => into_output(
                &runtime.invocation_id,
                "observe",
                "screen",
                observe::handle_screen(
                    &api,
                    &artifacts,
                    *full,
                    save_file.as_deref(),
                    *max_rows,
                    fields,
                ),
            ),
            ObserveCommands::Overlay { command } => match command {
                OverlayCommands::Get => into_output(
                    &runtime.invocation_id,
                    "observe",
                    "overlay",
                    observe::handle_overlay_get(&api),
                ),
                OverlayCommands::Set {
                    enable,
                    disable,
                    max_marks,
                    mark_scope,
                    refresh,
                    refresh_interval_ms,
                    offset_x,
                    offset_y,
                } => into_output(
                    &runtime.invocation_id,
                    "observe",
                    "overlay",
                    observe::handle_overlay_set(
                        &api,
                        OverlaySetOptions {
                            enabled: if *enable {
                                true
                            } else if *disable {
                                false
                            } else {
                                unreachable!("clap requires exactly one of --enable or --disable")
                            },
                            max_marks: *max_marks,
                            mark_scope: *mark_scope,
                            refresh: *refresh,
                            refresh_interval_ms: *refresh_interval_ms,
                            offset_x: *offset_x,
                            offset_y: *offset_y,
                        },
                    ),
                ),
            },
            ObserveCommands::Screenshot {
                save_file,
                max_dim,
                quality,
                annotate,
                hide_overlay,
                max_marks,
                mark_scope,
            } => into_output(
                &runtime.invocation_id,
                "observe",
                "screenshot",
                observe::handle_screenshot(
                    &api,
                    &artifacts,
                    ScreenshotOptions {
                        max_dim: *max_dim,
                        quality: *quality,
                        annotate: *annotate,
                        hide_overlay: *hide_overlay,
                        max_marks: *max_marks,
                        mark_scope: *mark_scope,
                        save_file: save_file.as_deref(),
                    },
                ),
            ),
            ObserveCommands::Top => into_output(
                &runtime.invocation_id,
                "observe",
                "top",
                observe::handle_top(&api),
            ),
            ObserveCommands::Refs { max_rows } => into_output(
                &runtime.invocation_id,
                "observe",
                "refs",
                observe::handle_refs(&api, *max_rows),
            ),
            ObserveCommands::Page {
                save_dir,
                fields,
                max_rows,
            } => into_output(
                &runtime.invocation_id,
                "observe",
                "page",
                observe::handle_page(&api, &artifacts, save_dir.as_deref(), fields, *max_rows),
            ),
        },
        Commands::Verify { command, .. } => match command {
            VerifyCommands::TextContains {
                text,
                case_sensitive,
            } => into_output(
                &runtime.invocation_id,
                "verify",
                "text-contains",
                verify::handle_text_contains(&api, text, !*case_sensitive),
            ),
            VerifyCommands::TopActivity { expected, mode } => into_output(
                &runtime.invocation_id,
                "verify",
                "top-activity",
                verify::handle_top_activity(&api, expected, mode),
            ),
            VerifyCommands::NodeExists {
                by,
                value,
                exact_match,
            } => into_output(
                &runtime.invocation_id,
                "verify",
                "node-exists",
                verify::handle_node_exists(&api, by, value, *exact_match),
            ),
        },
        Commands::Memory { .. } => unreachable!("memory commands are handled locally"),
        Commands::App { .. } => unreachable!("app commands are handled locally"),
        Commands::Connect { .. } => unreachable!("connect commands are handled locally"),
        Commands::Recover { command, .. } => match command {
            RecoverCommands::Back { times } => into_output(
                &runtime.invocation_id,
                "recover",
                "back",
                recover::handle_back(&api, *times),
            ),
            RecoverCommands::Home => into_output(
                &runtime.invocation_id,
                "recover",
                "home",
                recover::handle_home(&api),
            ),
            RecoverCommands::Relaunch { package_name } => into_output(
                &runtime.invocation_id,
                "recover",
                "relaunch",
                recover::handle_relaunch(&api, package_name),
            ),
        },
        Commands::Config { .. } => unreachable!("config commands are handled locally"),
    }
}

pub fn run_app_command(invocation_id: &str, cli: &Cli) -> Value {
    match &cli.command {
        Commands::App { command } => match command {
            AppCommands::Install {
                device,
                version,
                force,
                dry_run,
            } => into_output(
                invocation_id,
                "app",
                "install",
                app::handle_install(app::InstallOptions {
                    device: device.as_deref(),
                    version,
                    force: *force,
                    dry_run: *dry_run,
                }),
            ),
            AppCommands::Uninstall { device, dry_run } => into_output(
                invocation_id,
                "app",
                "uninstall",
                app::handle_uninstall(app::UninstallOptions {
                    device: device.as_deref(),
                    dry_run: *dry_run,
                }),
            ),
        },
        _ => unreachable!("run_app_command only handles app commands"),
    }
}

pub fn run_connect_command(invocation_id: &str, cli: &Cli, settings: &ResolvedSettings) -> Value {
    match &cli.command {
        Commands::Connect { command } => match command {
            ConnectCommands::Usb {
                device,
                local_port,
                print_only,
            } => into_output(
                invocation_id,
                "connect",
                "usb",
                connect::handle_usb_connect(
                    settings,
                    connect::UsbConnectOptions {
                        device: device.as_deref(),
                        local_port: *local_port,
                        print_only: *print_only,
                    },
                ),
            ),
        },
        _ => unreachable!("run_connect_command only handles connect commands"),
    }
}

pub fn run_memory_command(
    invocation_id: &str,
    cli: &Cli,
    memory_store: Option<&MemoryStore>,
) -> Value {
    match &cli.command {
        Commands::Memory { command } => match command {
            MemoryCommands::Save {
                app,
                topic,
                content,
            } => into_output(
                invocation_id,
                "memory",
                "save",
                memory::handle_save(memory_store, &cli.session, app, topic, content),
            ),
            MemoryCommands::Search {
                app,
                topic,
                query,
                limit,
            } => into_output(
                invocation_id,
                "memory",
                "search",
                memory::handle_search(
                    memory_store,
                    app.as_deref(),
                    topic.as_deref(),
                    query.as_deref(),
                    *limit,
                ),
            ),
            MemoryCommands::Delete { id } => into_output(
                invocation_id,
                "memory",
                "delete",
                memory::handle_delete(memory_store, *id),
            ),
            MemoryCommands::Log {
                session,
                app,
                status,
                limit,
            } => into_output(
                invocation_id,
                "memory",
                "log",
                memory::handle_log(
                    memory_store,
                    session.as_deref(),
                    app.as_deref(),
                    status.as_deref(),
                    *limit,
                ),
            ),
            MemoryCommands::Stats { session } => into_output(
                invocation_id,
                "memory",
                "stats",
                memory::handle_stats(memory_store, session.as_deref()),
            ),
            MemoryCommands::Experience {
                app,
                activity,
                page_fingerprint,
                failure_cause,
                limit,
            } => into_output(
                invocation_id,
                "memory",
                "experience",
                memory::handle_experience(
                    memory_store,
                    app,
                    activity,
                    page_fingerprint,
                    failure_cause.as_deref(),
                    *limit,
                ),
            ),
            MemoryCommands::Context => into_output(
                invocation_id,
                "memory",
                "context",
                memory::handle_context(memory_store, &cli.session),
            ),
        },
        _ => unreachable!("run_memory_command only handles memory commands"),
    }
}

pub fn run_config_command(invocation_id: &str, cli: &Cli, settings: &ResolvedSettings) -> Value {
    match &cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::List => into_output(
                invocation_id,
                "config",
                "list",
                Ok(serde_json::to_value(list_entries_map(settings))
                    .unwrap_or_else(|_| serde_json::json!({}))),
            ),
            ConfigCommands::Get { key } => into_output(
                invocation_id,
                "config",
                "get",
                match get_entry(settings, key) {
                    Some(entry) => Ok(serde_json::json!({
                        "key": entry.key,
                        "value": entry.value,
                        "source": entry.source,
                    })),
                    None => Err(crate::output::CommandError::invalid_params(format!(
                        "unknown config key: {key}"
                    ))),
                },
            ),
            ConfigCommands::Set { key, value } => into_output(
                invocation_id,
                "config",
                "set",
                set_key(&settings.config_path, key, value)
                    .map(|_| {
                        serde_json::json!({
                            "key": key,
                            "value": config_value_for_output(
                                key,
                                serde_json::Value::String(value.clone())
                            ),
                            "configPath": settings.config_path.display().to_string(),
                        })
                    })
                    .map_err(|e| crate::output::CommandError::internal(e.to_string())),
            ),
            ConfigCommands::Unset { key } => into_output(
                invocation_id,
                "config",
                "unset",
                unset_key(&settings.config_path, key)
                    .map(|_| {
                        serde_json::json!({
                            "key": key,
                            "configPath": settings.config_path.display().to_string(),
                        })
                    })
                    .map_err(|e| crate::output::CommandError::internal(e.to_string())),
            ),
        },
        _ => unreachable!("run_config_command only handles config commands"),
    }
}

pub fn persist_memory(
    memory_store: &Option<MemoryStore>,
    cli: &Cli,
    invocation_id: &str,
    result: &Value,
    duration_ms: u128,
) {
    let Some(store) = memory_store else {
        return;
    };

    if should_update_session_cache(&cli.command) {
        update_session_cache(store, &cli.session, &cli.command, result);
    }

    if should_record_event(&cli.command) {
        record_event_and_close(store, cli, invocation_id, result, duration_ms);
    }
}

fn observe_health(
    api: &ApiClient<'_>,
    settings: &ResolvedSettings,
) -> crate::output::CommandResult {
    let health = api.health().map_err(crate::output::CommandError::from)?;
    Ok(serde_json::json!({
        "health": health.payload,
        "connection": connection_metadata(settings)
    }))
}

fn connection_metadata(settings: &ResolvedSettings) -> Value {
    if settings.remote_url_source != Some(ConfigSource::File)
        || !is_configured_usb_forward_url(settings)
    {
        return serde_json::json!({
            "url": settings.remote_url.as_deref(),
            "transport": "unknown",
            "device": null,
            "localPort": null,
            "devicePort": null,
        });
    }

    serde_json::json!({
        "url": settings.remote_url.as_deref(),
        "transport": settings.connection_transport.as_deref().unwrap_or("unknown"),
        "device": settings.connection_usb_device.as_deref(),
        "localPort": settings.connection_usb_local_port,
        "devicePort": settings.connection_usb_device_port,
    })
}

fn is_configured_usb_forward_url(settings: &ResolvedSettings) -> bool {
    let Some("usb-forward") = settings.connection_transport.as_deref() else {
        return false;
    };
    let (Some(remote_url), Some(local_port)) = (
        settings.remote_url.as_deref(),
        settings.connection_usb_local_port,
    ) else {
        return false;
    };
    let remote_url = remote_url.trim_end_matches('/');
    remote_url == format!("http://127.0.0.1:{local_port}")
        || remote_url == format!("http://localhost:{local_port}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::memory::PageContext;
    use clap::Parser;
    use serde_json::json;
    use std::path::PathBuf;

    fn base_settings() -> ResolvedSettings {
        ResolvedSettings {
            config_path: PathBuf::from("/tmp/af-runner-test.toml"),
            output: crate::cli::OutputFormat::Text,
            output_source: ConfigSource::Default,
            memory_db: PathBuf::from("af.db"),
            memory_db_source: ConfigSource::Default,
            remote_url: Some("http://127.0.0.1:18081".into()),
            remote_url_source: Some(ConfigSource::File),
            remote_token: None,
            remote_token_source: None,
            connection_transport: Some("usb-forward".into()),
            connection_transport_source: Some(ConfigSource::File),
            connection_usb_device: Some("RFCX123456".into()),
            connection_usb_device_source: Some(ConfigSource::File),
            connection_usb_local_port: Some(18081),
            connection_usb_local_port_source: Some(ConfigSource::File),
            connection_usb_device_port: Some(8081),
            connection_usb_device_port_source: Some(ConfigSource::File),
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
    fn health_connection_metadata_includes_configured_transport_for_config_url() {
        let settings = base_settings();

        assert_eq!(
            connection_metadata(&settings),
            json!({
                "url": "http://127.0.0.1:18081",
                "transport": "usb-forward",
                "device": "RFCX123456",
                "localPort": 18081,
                "devicePort": 8081,
            })
        );
    }

    #[test]
    fn health_connection_metadata_ignores_stale_transport_for_cli_url() {
        let mut settings = base_settings();
        settings.remote_url = Some("http://192.0.2.10:8081".into());
        settings.remote_url_source = Some(ConfigSource::Cli);

        assert_eq!(
            connection_metadata(&settings),
            json!({
                "url": "http://192.0.2.10:8081",
                "transport": "unknown",
                "device": null,
                "localPort": null,
                "devicePort": null,
            })
        );
    }

    #[test]
    fn health_connection_metadata_ignores_stale_usb_transport_for_file_lan_url() {
        let mut settings = base_settings();
        settings.remote_url = Some("http://192.0.2.10:8081".into());

        assert_eq!(
            connection_metadata(&settings),
            json!({
                "url": "http://192.0.2.10:8081",
                "transport": "unknown",
                "device": null,
                "localPort": null,
                "devicePort": null,
            })
        );
    }

    #[test]
    fn persist_memory_records_event_for_act() {
        let cli = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "act",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "back",
        ]);
        let store = Some(MemoryStore::new_in_memory().expect("init"));

        persist_memory(
            &store,
            &cli,
            "invoke-1",
            &json!({"status": "ok", "data": {}, "category": "act", "op": "back"}),
            12,
        );

        let events = store
            .as_ref()
            .unwrap()
            .query_events(Some("wf-test"), None, None, 10)
            .expect("query");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, "act");
        assert_eq!(events[0].op, "back");
    }

    #[test]
    fn persist_memory_updates_session_state_for_observe_top() {
        let cli = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "observe",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "top",
        ]);
        let store = Some(MemoryStore::new_in_memory().expect("init"));

        persist_memory(
            &store,
            &cli,
            "invoke-1",
            &json!({"status": "ok", "data": {"topActivity": "com.android.settings/.Settings"}}),
            5,
        );

        let ctx = store
            .as_ref()
            .unwrap()
            .get_session_state("wf-test")
            .expect("get")
            .expect("should exist");
        assert_eq!(ctx.app, "com.android.settings");
        assert_eq!(ctx.activity, "com.android.settings/.Settings");
    }

    #[test]
    fn persist_memory_skips_observe_events() {
        let cli = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "observe",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "top",
        ]);
        let store = Some(MemoryStore::new_in_memory().expect("init"));

        persist_memory(
            &store,
            &cli,
            "invoke-1",
            &json!({"status": "ok", "data": {"topActivity": "com.a/.M"}}),
            5,
        );

        let events = store
            .as_ref()
            .unwrap()
            .query_events(Some("wf-test"), None, None, 10)
            .expect("query");
        assert_eq!(events.len(), 0, "observe should not create events");
    }

    #[test]
    fn event_picks_up_session_context() {
        let store = Some(MemoryStore::new_in_memory().expect("init"));
        let s = store.as_ref().unwrap();
        s.update_session_activity("wf-test", "com.a", "com.a/.Main", "2026-01-01T00:00:00Z")
            .expect("update");

        let cli = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "act",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "back",
        ]);
        persist_memory(
            &store,
            &cli,
            "invoke-1",
            &json!({"status": "ok", "data": {}}),
            10,
        );

        let events = s
            .query_events(Some("wf-test"), None, None, 10)
            .expect("query");
        assert_eq!(events[0].app, "com.a");
        assert_eq!(events[0].activity, "com.a/.Main");
    }

    #[test]
    fn verify_closes_transition_when_observation_is_fresh() {
        let store = Some(MemoryStore::new_in_memory().expect("init"));
        let s = store.as_ref().unwrap();

        let pre_ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0|rid=list".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "2026-01-01T00:00:00Z".into(),
        };
        s.update_session_state("wf-test", &pre_ctx).expect("pre");

        let cli_act = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "act",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "tap",
            "--by",
            "text",
            "--value",
            "Wi-Fi",
        ]);
        persist_memory(
            &store,
            &cli_act,
            "inv-1",
            &json!({"status": "ok", "data": {}}),
            10,
        );

        // Simulate observe screen/top AFTER the act (fresh observation)
        let future_ts = "2099-12-31T23:59:59Z";
        let post_ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.WiFi".into(),
            page_fingerprint: "act=com.a/.WiFi|wv=0|rid=wifi_list".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: future_ts.into(),
        };
        s.update_session_state("wf-test", &post_ctx).expect("post");

        let cli_verify = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "verify",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "text-contains",
            "--text",
            "Wi-Fi",
        ]);
        persist_memory(
            &store,
            &cli_verify,
            "inv-2",
            &json!({"status": "ok", "data": {"matched": true, "text": "Wi-Fi"}}),
            5,
        );

        let transitions = s
            .query_transitions("com.a", "com.a/.Main", &pre_ctx.page_fingerprint, 10)
            .expect("query");
        assert!(!transitions.is_empty(), "transition should be closed");
        assert_eq!(transitions[0].1.verified_count, 1);
    }

    #[test]
    fn verify_skips_transition_when_observation_is_stale() {
        let store = Some(MemoryStore::new_in_memory().expect("init"));
        let s = store.as_ref().unwrap();

        let pre_ctx = PageContext {
            app: "com.a".into(),
            activity: "com.a/.Main".into(),
            page_fingerprint: "act=com.a/.Main|wv=0|rid=list".into(),
            fingerprint_source: "screen".into(),
            mode: "SYSTEM_API".into(),
            has_webview: false,
            node_reliability: "high".into(),
            ref_version: None,
            observed_at: "2020-01-01T00:00:00Z".into(),
        };
        s.update_session_state("wf-test", &pre_ctx).expect("pre");

        let cli_act = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "act",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "tap",
            "--by",
            "text",
            "--value",
            "Wi-Fi",
        ]);
        persist_memory(
            &store,
            &cli_act,
            "inv-1",
            &json!({"status": "ok", "data": {}}),
            10,
        );

        // NO observe between act and verify — stale observation

        let cli_verify = Cli::parse_from([
            "af",
            "--session",
            "wf-test",
            "verify",
            "--url",
            "http://127.0.0.1:18080",
            "--token",
            "demo-token",
            "text-contains",
            "--text",
            "Wi-Fi",
        ]);
        persist_memory(
            &store,
            &cli_verify,
            "inv-2",
            &json!({"status": "ok", "data": {"matched": true, "text": "Wi-Fi"}}),
            5,
        );

        let transitions = s
            .query_transitions("com.a", "com.a/.Main", &pre_ctx.page_fingerprint, 10)
            .expect("query");
        assert!(
            transitions.is_empty(),
            "transition should NOT be closed without fresh observation"
        );
    }
}
