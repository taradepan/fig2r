use crate::error::Fig2rError;
use std::fs;
use std::io;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".fig2r")
}

fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn save_token(token: &str) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    // Use toml serialization to handle escaping properly
    let escaped = token.replace('\\', "\\\\").replace('"', "\\\"");
    let content = format!("[auth]\ntoken = \"{escaped}\"\n");
    let path = config_path();
    fs::write(&path, content)?;
    // Restrict file permissions to owner-only
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn load_token() -> Option<String> {
    // Priority: FIGMA_TOKEN env var > config file
    if let Ok(token) = std::env::var("FIGMA_TOKEN")
        && !token.is_empty()
    {
        return Some(token);
    }

    let content = fs::read_to_string(config_path()).ok()?;
    let table: toml::Table = content.parse().ok()?;
    table.get("auth")?.get("token")?.as_str().map(String::from)
}

pub fn resolve_token(cli_token: Option<&str>) -> Result<String, Fig2rError> {
    // Priority: --token flag > env var > config file
    if let Some(t) = cli_token
        && !t.is_empty()
    {
        return Ok(t.to_string());
    }
    load_token().ok_or_else(|| {
        Fig2rError::Message(
            "No Figma token found. Run `fig2r auth` or set FIGMA_TOKEN env var.\n\
             Generate a token at: https://www.figma.com/settings → Security → Personal access tokens\n\
             Required scope: file_content:read"
                .to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_exists() {
        let dir = config_dir();
        assert!(dir.to_str().unwrap().contains(".fig2r"));
    }

    #[test]
    fn test_resolve_token_cli_flag_wins() {
        let result = resolve_token(Some("cli-token"));
        assert_eq!(result.unwrap(), "cli-token");
    }
}
