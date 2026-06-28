use crate::ai_agents::AiAgentAvailability;
use std::path::{Path, PathBuf};

pub(crate) fn check_cli() -> AiAgentAvailability {
    crate::cli_agent_runtime::check_cli_availability(find_binary)
}

pub(crate) fn find_binary() -> Result<PathBuf, String> {
    crate::cli_agent_runtime::find_cli_binary(
        "droid",
        droid_binary_candidates(),
        "Droid CLI",
        "https://docs.factory.ai",
    )
}

fn droid_binary_candidates() -> Vec<PathBuf> {
    dirs::home_dir()
        .map(|home| droid_binary_candidates_for_home(&home))
        .unwrap_or_default()
}

fn droid_binary_candidates_for_home(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join("bin/droid"),
        home.join("bin/droid.exe"),
        home.join(".local/bin/droid"),
        home.join(".local/bin/droid.exe"),
        home.join(".factory/bin/droid"),
        home.join(".factory/bin/droid.exe"),
        home.join(".local/share/mise/shims/droid"),
        home.join(".local/share/mise/shims/droid.exe"),
        home.join(".asdf/shims/droid"),
        home.join(".asdf/shims/droid.exe"),
        home.join(".npm-global/bin/droid"),
        home.join(".npm-global/bin/droid.cmd"),
        home.join(".npm-global/bin/droid.exe"),
        home.join(".npm/bin/droid"),
        home.join(".npm/bin/droid.cmd"),
        home.join(".npm/bin/droid.exe"),
        home.join(".bun/bin/droid"),
        home.join(".bun/bin/droid.exe"),
        home.join(".linuxbrew/bin/droid"),
        home.join("AppData/Roaming/npm/droid.cmd"),
        home.join("AppData/Roaming/npm/droid.exe"),
        home.join("AppData/Local/pnpm/droid.cmd"),
        home.join("AppData/Local/pnpm/droid.exe"),
        home.join("scoop/shims/droid.exe"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin/droid"),
        PathBuf::from("/usr/local/bin/droid"),
        PathBuf::from("/opt/homebrew/bin/droid"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_candidates_include_supported_installs() {
        let home = PathBuf::from("/Users/alex");
        let candidates = droid_binary_candidates_for_home(&home);
        let expected = [
            home.join(".local/bin/droid"),
            home.join(".factory/bin/droid"),
            home.join(".local/share/mise/shims/droid"),
            home.join(".asdf/shims/droid"),
            home.join(".npm-global/bin/droid"),
            home.join(".bun/bin/droid"),
            PathBuf::from("/opt/homebrew/bin/droid"),
        ];

        for candidate in expected {
            assert!(
                candidates.contains(&candidate),
                "missing {}",
                candidate.display()
            );
        }
    }

    #[test]
    fn binary_candidates_include_windows_native_installs() {
        let home = PathBuf::from("C:/Users/alex");
        let candidates = droid_binary_candidates_for_home(&home);
        let expected = [
            home.join("bin/droid.exe"),
            home.join("AppData/Roaming/npm/droid.cmd"),
            home.join("AppData/Local/pnpm/droid.cmd"),
            home.join("scoop/shims/droid.exe"),
        ];

        for candidate in expected {
            assert!(
                candidates.contains(&candidate),
                "missing {}",
                candidate.display()
            );
        }
    }
}
