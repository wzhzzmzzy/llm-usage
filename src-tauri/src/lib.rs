use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    runner: RunnerStatus,
    ccusage: CcUsageStatus,
    config_path: String,
}

#[derive(Serialize, Deserialize)]
struct RunnerStatus {
    found: bool,
    path: Option<String>,
    version: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CcUsageStatus {
    available: bool,
    version: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct RefreshResponse {
    status: String,
    snapshot: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    status: String,
    last_refresh: Option<String>,
    last_success: Option<String>,
    last_error: Option<String>,
    cells: serde_json::Value,
    daily: serde_json::Value,
    monthly: serde_json::Value,
    session: serde_json::Value,
    blocks: serde_json::Value,
    timezone: String,
}

fn get_runner() -> String {
    std::env::var("CCUSAGE_RUNNER").unwrap_or_default()
}

fn get_package_spec() -> String {
    std::env::var("CCUSAGE_PACKAGE").unwrap_or_else(|_| "ccusage".to_string())
}

fn run_ccusage(args: &[&str]) -> Result<std::process::Output, String> {
    let runner = get_runner();
    let package = get_package_spec();
    
    let mut full_args: Vec<&str> = args.to_vec();
    full_args.push("--offline");
    
    if runner.is_empty() {
        Command::new(&package)
            .args(&full_args)
            .output()
            .map_err(|e| format!("Failed to run {}: {}", package, e))
    } else {
        let mut cmd_args = vec![package.as_str()];
        cmd_args.extend_from_slice(&full_args);
        Command::new(&runner)
            .args(&cmd_args)
            .output()
            .map_err(|e| format!("Failed to run {} {}: {}", runner, package, e))
    }
}

#[tauri::command]
fn health() -> Result<HealthResponse, String> {
    let runner = get_runner();
    let package = get_package_spec();

    let (runner_found, runner_path, runner_version) = if runner.is_empty() {
        let output = Command::new(&package)
            .arg("--version")
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
                (true, Some(package.clone()), Some(version))
            }
            _ => (false, None, None),
        }
    } else {
        let output = Command::new(&runner)
            .arg("--version")
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
                (true, Some(runner.clone()), Some(version))
            }
            _ => (false, None, None),
        }
    };

    let ccusage_output = run_ccusage(&["--version"]);
    let (ccusage_available, ccusage_version) = match ccusage_output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(version))
        }
        _ => (false, None),
    };

    let status = if runner_found && ccusage_available {
        "healthy"
    } else if ccusage_available {
        "healthy"
    } else {
        "unhealthy"
    };

    let config_path = dirs::config_dir()
        .map(|p| p.join("llm-usage-dashboard").join("config.toml"))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(HealthResponse {
        status: status.to_string(),
        runner: RunnerStatus {
            found: runner_found || ccusage_available,
            path: runner_path,
            version: runner_version,
        },
        ccusage: CcUsageStatus {
            available: ccusage_available,
            version: ccusage_version,
        },
        config_path,
    })
}

#[tauri::command]
fn refresh() -> Result<RefreshResponse, String> {
    let sources = ["all", "claude", "codex", "gemini", "opencode"];
    let reports = ["daily", "monthly", "session", "blocks"];

    let mut daily = serde_json::Map::new();
    let mut monthly = serde_json::Map::new();
    let mut session = serde_json::Map::new();
    let mut blocks = serde_json::Map::new();
    let mut cells = serde_json::Map::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for source in &sources {
        for report in &reports {
            let mut args = vec![];
            if *source != "all" {
                args.push(*source);
            }
            args.push(*report);
            args.push("--json");

            let key = format!("{}_{}", source, report);

            match run_ccusage(&args) {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    match serde_json::from_str::<serde_json::Value>(&stdout) {
                        Ok(value) => {
                            match *report {
                                "daily" => daily.insert(key.clone(), value),
                                "monthly" => monthly.insert(key.clone(), value),
                                "session" => session.insert(key.clone(), value),
                                "blocks" => blocks.insert(key.clone(), value),
                                _ => None,
                            };
                            cells.insert(
                                key,
                                serde_json::json!({
                                    "cell": { "source": source, "report": report },
                                    "status": "success",
                                    "durationMs": 0
                                }),
                            );
                            success_count += 1;
                        }
                        Err(e) => {
                            cells.insert(
                                key,
                                serde_json::json!({
                                    "cell": { "source": source, "report": report },
                                    "status": "error",
                                    "error": format!("Invalid JSON: {}", e),
                                    "durationMs": 0
                                }),
                            );
                            error_count += 1;
                        }
                    }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    cells.insert(
                        key,
                        serde_json::json!({
                            "cell": { "source": source, "report": report },
                            "status": "error",
                            "error": format!("Command failed: {}", stderr),
                            "exitCode": output.status.code(),
                            "durationMs": 0
                        }),
                    );
                    error_count += 1;
                }
                Err(e) => {
                    cells.insert(
                        key,
                        serde_json::json!({
                            "cell": { "source": source, "report": report },
                            "status": "error",
                            "error": e,
                            "durationMs": 0
                        }),
                    );
                    error_count += 1;
                }
            }
        }
    }

    let status = if success_count > 0 && error_count == 0 {
        "success"
    } else if success_count > 0 {
        "partial"
    } else {
        "error"
    };

    let now = chrono::Utc::now().to_rfc3339();

    let snapshot = Snapshot {
        status: status.to_string(),
        last_refresh: Some(now.clone()),
        last_success: if success_count > 0 { Some(now) } else { None },
        last_error: if error_count > 0 { Some(chrono::Utc::now().to_rfc3339()) } else { None },
        cells: serde_json::Value::Object(cells),
        daily: serde_json::Value::Object(daily),
        monthly: serde_json::Value::Object(monthly),
        session: serde_json::Value::Object(session),
        blocks: serde_json::Value::Object(blocks),
        timezone: "UTC".to_string(),
    };

    Ok(RefreshResponse {
        status: status.to_string(),
        snapshot: serde_json::to_value(&snapshot).map_err(|e| e.to_string())?,
    })
}

#[tauri::command]
fn get_snapshot() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "status": "nodata",
        "lastRefresh": null,
        "lastSuccess": null,
        "lastError": null,
        "cells": {},
        "daily": {},
        "monthly": {},
        "session": {},
        "blocks": {},
        "timezone": "UTC"
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![health, refresh, get_snapshot])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
