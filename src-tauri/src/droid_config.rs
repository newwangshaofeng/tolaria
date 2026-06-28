use crate::ai_agents::AiAgentPermissionMode;
use crate::cli_agent_runtime::AgentStreamRequest;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub(crate) fn build_command(
    binary: &Path,
    request: &AgentStreamRequest,
    settings_dir: &Path,
) -> Result<std::process::Command, String> {
    write_mcp_config(settings_dir, &request.vault_path, &request.vault_paths)?;
    let target = crate::cli_agent_runtime::command_target_avoiding_windows_cmd_shim(binary)?;
    let mut command = crate::hidden_command(&target.program);
    crate::cli_agent_runtime::configure_agent_command_environment(&mut command, binary);
    if let Some(first_arg) = target.first_arg {
        command.arg(first_arg);
    }
    command
        .args(build_args(request.permission_mode))
        .arg("--cwd")
        .arg(&request.vault_path)
        .arg(build_prompt(request))
        .env("NO_COLOR", "1")
        .current_dir(&request.vault_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(command)
}

fn build_args(permission_mode: AiAgentPermissionMode) -> Vec<String> {
    vec![
        "exec".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--auto".into(),
        autonomy_level(permission_mode).into(),
    ]
}

fn autonomy_level(permission_mode: AiAgentPermissionMode) -> &'static str {
    match permission_mode {
        AiAgentPermissionMode::Safe => "low",
        AiAgentPermissionMode::PowerUser => "medium",
    }
}

fn build_prompt(request: &AgentStreamRequest) -> String {
    crate::cli_agent_runtime::build_prompt(&request.message, request.system_prompt.as_deref())
}

fn write_mcp_config(
    settings_dir: &Path,
    vault_path: &str,
    vault_paths: &[String],
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(settings_dir)
        .map_err(|error| format!("Failed to create Droid settings directory: {error}"))?;
    let config_path = settings_dir.join("mcp.json");
    let config = build_mcp_config(vault_path, vault_paths)?;
    std::fs::write(&config_path, config)
        .map_err(|error| format!("Failed to write Droid mcp.json: {error}"))?;
    Ok(config_path)
}

fn build_mcp_config(vault_path: &str, vault_paths: &[String]) -> Result<String, String> {
    let mcp_server_path = crate::cli_agent_runtime::mcp_server_path_string()?;
    let vault_paths_json = crate::cli_agent_runtime::active_vault_paths_json(vault_path, vault_paths);
    let config = serde_json::json!({
        "mcpServers": {
            "tolaria": {
                "command": "node",
                "args": [mcp_server_path],
                "env": {
                    "VAULT_PATH": vault_path,
                    "VAULT_PATHS": vault_paths_json,
                    "WS_UI_PORT": "9711"
                },
                "disabled": false
            }
        }
    });

    serde_json::to_string(&config)
        .map_err(|error| format!("Failed to serialize Droid mcp.json: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn request() -> AgentStreamRequest {
        AgentStreamRequest {
            message: "Rename the note".into(),
            system_prompt: None,
            vault_path: "/tmp/vault".into(),
            vault_paths: Vec::new(),
            permission_mode: AiAgentPermissionMode::Safe,
        }
    }

    #[test]
    fn command_uses_headless_stream_json_mode() {
        let settings_dir = tempfile::tempdir().unwrap();
        let command =
            build_command(&PathBuf::from("droid"), &request(), settings_dir.path()).unwrap();
        let actual_args: Vec<&OsStr> = command.get_args().collect();

        assert_eq!(command.get_program(), OsStr::new("droid"));
        assert_eq!(actual_args[0], OsStr::new("exec"));
        assert_eq!(actual_args[1], OsStr::new("--output-format"));
        assert_eq!(actual_args[2], OsStr::new("stream-json"));
        assert_eq!(actual_args[3], OsStr::new("--auto"));
        assert_eq!(actual_args[4], OsStr::new("low"));
        assert!(actual_args.contains(&OsStr::new("--cwd")));
        assert_eq!(actual_args.last(), Some(&OsStr::new("Rename the note")));
        assert_eq!(command.get_current_dir(), Some(Path::new("/tmp/vault")));
    }

    #[test]
    fn power_user_maps_to_medium_autonomy() {
        let settings_dir = tempfile::tempdir().unwrap();
        let command = build_command(
            &PathBuf::from("droid"),
            &AgentStreamRequest {
                permission_mode: AiAgentPermissionMode::PowerUser,
                ..request()
            },
            settings_dir.path(),
        )
        .unwrap();
        let actual_args: Vec<&OsStr> = command.get_args().collect();

        assert_eq!(actual_args[4], OsStr::new("medium"));
    }

    #[test]
    fn mcp_config_includes_tolaria_server() {
        let config = build_mcp_config("/tmp/vault", &[]).unwrap();
        let json: serde_json::Value = serde_json::from_str(&config).unwrap();

        assert_eq!(json["mcpServers"]["tolaria"]["command"], "node");
        assert_eq!(
            json["mcpServers"]["tolaria"]["env"]["VAULT_PATH"],
            "/tmp/vault"
        );
        assert_eq!(json["mcpServers"]["tolaria"]["env"]["WS_UI_PORT"], "9711");
        assert_eq!(json["mcpServers"]["tolaria"]["disabled"], false);
        assert!(json["mcpServers"]["tolaria"]["args"][0]
            .as_str()
            .unwrap()
            .ends_with("index.js"));
    }

    #[test]
    fn write_mcp_config_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_mcp_config(dir.path(), "/tmp/vault", &[]).unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with("mcp.json"));
    }

    #[test]
    fn prompt_keeps_system_prompt_first() {
        let prompt = build_prompt(&AgentStreamRequest {
            system_prompt: Some("Be concise".into()),
            ..request()
        });

        assert!(prompt.starts_with("System instructions:\nBe concise"));
        assert!(prompt.contains("User request:\nRename the note"));
    }
}
