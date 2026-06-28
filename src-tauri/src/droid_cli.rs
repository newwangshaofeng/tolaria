use crate::ai_agents::{AiAgentAvailability, AiAgentStreamEvent};
pub use crate::cli_agent_runtime::AgentStreamRequest;
use std::path::Path;

pub fn check_cli() -> AiAgentAvailability {
    crate::droid_discovery::check_cli()
}

pub fn run_agent_stream<F>(request: AgentStreamRequest, emit: F) -> Result<String, String>
where
    F: FnMut(AiAgentStreamEvent),
{
    let binary = crate::droid_discovery::find_binary()?;
    run_agent_stream_with_binary(&binary, request, emit)
}

fn run_agent_stream_with_binary<F>(
    binary: &Path,
    request: AgentStreamRequest,
    emit: F,
) -> Result<String, String>
where
    F: FnMut(AiAgentStreamEvent),
{
    let settings_dir = tempfile::Builder::new()
        .prefix("tolaria-droid-agent-")
        .tempdir()
        .map_err(|error| format!("Failed to create Droid settings directory: {error}"))?;
    let command = crate::droid_config::build_command(binary, &request, settings_dir.path())?;
    crate::cli_agent_runtime::run_ai_agent_json_stream(
        command,
        "droid",
        emit,
        droid_session_id,
        dispatch_droid_event,
        format_droid_error,
    )
}

fn droid_session_id(json: &serde_json::Value) -> Option<&str> {
    json["session_id"].as_str()
}

fn dispatch_droid_event<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    match json["type"].as_str().unwrap_or_default() {
        "session.started" => emit_droid_session_started(json, emit),
        "assistant_text_delta" => emit_droid_text_delta(json, emit),
        "assistant" => emit_droid_assistant(json, emit),
        "thinking_text_delta" => emit_droid_thinking_delta(json, emit),
        "tool_call" => emit_droid_tool_call(json, emit),
        "tool_result" => emit_droid_tool_result(json, emit),
        "error" => emit_droid_error(json, emit),
        "result" => emit_droid_result(json, emit),
        _ => {}
    }
}

fn emit_droid_session_started<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    if let Some(session_id) = json["session_id"].as_str() {
        emit(AiAgentStreamEvent::Init {
            session_id: session_id.to_string(),
        });
    }
}

fn emit_droid_text_delta<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    if let Some(text) = json["text"].as_str().filter(|text| !text.is_empty()) {
        emit(AiAgentStreamEvent::TextDelta {
            text: text.to_string(),
        });
    }
}

fn emit_droid_assistant<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    if let Some(text) = json["text"].as_str().filter(|text| !text.is_empty()) {
        emit(AiAgentStreamEvent::TextDelta {
            text: text.to_string(),
        });
    }
}

fn emit_droid_thinking_delta<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    if let Some(text) = json["text"].as_str().filter(|text| !text.is_empty()) {
        emit(AiAgentStreamEvent::ThinkingDelta {
            text: text.to_string(),
        });
    }
}

fn emit_droid_tool_call<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    let tool_name = json["toolName"]
        .as_str()
        .or_else(|| json["tool_name"].as_str())
        .unwrap_or("Droid tool");
    let tool_id = json["toolUseId"]
        .as_str()
        .or_else(|| json["tool_use_id"].as_str())
        .or_else(|| json["id"].as_str())
        .unwrap_or(tool_name);
    let input = (!json["input"].is_null()).then(|| json["input"].to_string());

    emit(AiAgentStreamEvent::ToolStart {
        tool_name: tool_name.to_string(),
        tool_id: tool_id.to_string(),
        input,
    });
}

fn emit_droid_tool_result<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    let tool_id = json["toolUseId"]
        .as_str()
        .or_else(|| json["tool_use_id"].as_str())
        .or_else(|| json["id"].as_str())
        .unwrap_or("droid-tool");
    let output = json["output"]
        .as_str()
        .or_else(|| json["result"].as_str())
        .or_else(|| json["error"]["message"].as_str())
        .map(str::to_string);

    emit(AiAgentStreamEvent::ToolDone {
        tool_id: tool_id.to_string(),
        output,
    });
}

fn emit_droid_error<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    if let Some(message) = json["message"]
        .as_str()
        .or_else(|| json["error"]["message"].as_str())
    {
        emit(AiAgentStreamEvent::Error {
            message: message.to_string(),
        });
    }
}

fn emit_droid_result<F>(json: &serde_json::Value, emit: &mut F)
where
    F: FnMut(AiAgentStreamEvent),
{
    if json["is_error"].as_bool() == Some(true) {
        if let Some(message) = json["error"]["message"].as_str() {
            emit(AiAgentStreamEvent::Error {
                message: message.to_string(),
            });
        }
        return;
    }

    if let Some(text) = json["result"].as_str().filter(|text| !text.is_empty()) {
        emit(AiAgentStreamEvent::TextDelta {
            text: text.to_string(),
        });
    }
}

fn format_droid_error(stderr_output: String, status: String) -> String {
    let lower = stderr_output.to_ascii_lowercase();
    if is_auth_error(&lower) {
        return "Droid CLI is not authenticated. Set the FACTORY_API_KEY environment variable or run `droid` in your terminal to log in, then retry.".into();
    }

    if stderr_output.trim().is_empty() {
        format!("droid exited with status {status}")
    } else {
        stderr_output.lines().take(3).collect::<Vec<_>>().join("\n")
    }
}

fn is_auth_error(lower: &str) -> bool {
    [
        "auth",
        "login",
        "sign in",
        "api key",
        "factory_api_key",
        "unauthorized",
        "401",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_agents::AiAgentPermissionMode;

    #[cfg(unix)]
    fn executable_script(dir: &Path, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script = dir.join("droid");
        std::fs::write(&script, format!("#!/bin/sh\n{body}")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn request(vault_path: String) -> AgentStreamRequest {
        AgentStreamRequest {
            message: "Summarize".into(),
            system_prompt: None,
            vault_path,
            vault_paths: Vec::new(),
            permission_mode: AiAgentPermissionMode::Safe,
        }
    }

    #[cfg(unix)]
    #[test]
    fn run_agent_stream_maps_droid_stream_json_response() {
        let dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let binary = executable_script(
            dir.path(),
            r#"printf '%s\n' '{"type":"session.started","session_id":"droid_1","model":"claude-sonnet-4-5-20250929"}'
printf '%s\n' '{"type":"assistant_text_delta","text":"Done"}'
printf '%s\n' '{"type":"result","is_error":false,"result":"","session_id":"droid_1"}'
"#,
        );

        let mut events = Vec::new();
        let session_id = run_agent_stream_with_binary(
            &binary,
            request(vault.path().to_string_lossy().into_owned()),
            |event| events.push(event),
        )
        .unwrap();

        assert_eq!(session_id, "droid_1");
        assert!(matches!(
            &events[0],
            AiAgentStreamEvent::Init { session_id } if session_id == "droid_1"
        ));
        assert!(matches!(
            &events[1],
            AiAgentStreamEvent::TextDelta { text } if text == "Done"
        ));
        assert!(matches!(events.last(), Some(AiAgentStreamEvent::Done)));
    }

    #[cfg(unix)]
    #[test]
    fn run_agent_stream_maps_droid_tool_events() {
        let dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let binary = executable_script(
            dir.path(),
            r#"printf '%s\n' '{"type":"session.started","session_id":"droid_2"}'
printf '%s\n' '{"type":"tool_call","toolName":"tolaria__search_notes","toolUseId":"tool_1","input":{"query":"meeting"}}'
printf '%s\n' '{"type":"tool_result","toolUseId":"tool_1","output":"2 notes"}'
printf '%s\n' '{"type":"assistant_text_delta","text":"I found 2 notes."}'
printf '%s\n' '{"type":"result","is_error":false,"result":"","session_id":"droid_2"}'
"#,
        );

        let mut events = Vec::new();
        let session_id = run_agent_stream_with_binary(
            &binary,
            request(vault.path().to_string_lossy().into_owned()),
            |event| events.push(event),
        )
        .unwrap();

        assert_eq!(session_id, "droid_2");
        assert!(events.iter().any(|event| matches!(
            event,
            AiAgentStreamEvent::ToolStart { tool_name, tool_id, input }
                if tool_name == "tolaria__search_notes"
                    && tool_id == "tool_1"
                    && input.is_some()
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AiAgentStreamEvent::ToolDone { tool_id, output }
                if tool_id == "tool_1" && output.as_deref() == Some("2 notes")
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AiAgentStreamEvent::TextDelta { text } if text == "I found 2 notes."
        )));
        assert!(matches!(events.last(), Some(AiAgentStreamEvent::Done)));
    }

    #[cfg(unix)]
    #[test]
    fn run_agent_stream_reports_droid_auth_errors() {
        let dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let binary = executable_script(
            dir.path(),
            r#"printf '%s\n' 'FACTORY_API_KEY not set' >&2
exit 3
"#,
        );

        let mut events = Vec::new();
        let session_id = run_agent_stream_with_binary(
            &binary,
            request(vault.path().to_string_lossy().into_owned()),
            |event| events.push(event),
        )
        .unwrap();

        assert!(session_id.is_empty());
        assert!(events.iter().any(|event| matches!(
            event,
            AiAgentStreamEvent::Error { message } if message.contains("not authenticated")
        )));
        assert!(matches!(events.last(), Some(AiAgentStreamEvent::Done)));
    }

    #[cfg(unix)]
    #[test]
    fn run_agent_stream_emits_result_text_as_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let binary = executable_script(
            dir.path(),
            r#"printf '%s\n' '{"type":"session.started","session_id":"droid_3"}'
printf '%s\n' '{"type":"result","is_error":false,"result":"Final answer","session_id":"droid_3"}'
"#,
        );

        let mut events = Vec::new();
        run_agent_stream_with_binary(
            &binary,
            request(vault.path().to_string_lossy().into_owned()),
            |event| events.push(event),
        )
        .unwrap();

        assert!(events.iter().any(|event| matches!(
            event,
            AiAgentStreamEvent::TextDelta { text } if text == "Final answer"
        )));
    }

    #[test]
    fn format_droid_error_returns_status_for_empty_stderr() {
        let result = format_droid_error(String::new(), "1".into());
        assert!(result.contains("status 1"));
    }
}
