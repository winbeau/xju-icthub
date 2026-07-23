use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub feiyue_auth_url: String,
    pub import_root: PathBuf,
    pub import_max_upload_bytes: u64,
    pub import_max_unpacked_bytes: u64,
    pub import_worker_embedded: bool,
    pub import_worker_poll_ms: u64,
    pub import_worker_lease_secs: u64,
    pub ffprobe_bin: String,
    pub ffmpeg_bin: String,
    pub pdftoppm_bin: String,
    pub codex_enabled: bool,
    pub codex_bin: PathBuf,
    pub codex_home: PathBuf,
    pub codex_runtime_root: PathBuf,
    pub codex_base_url: Option<String>,
    pub codex_model: Option<String>,
    pub codex_api_key_file: Option<PathBuf>,
    pub codex_timeout_secs: u64,
    pub github_enabled: bool,
    pub github_owner: Option<String>,
    pub github_repo_prefix: String,
    pub github_token_file: Option<PathBuf>,
    pub github_cli_bin: PathBuf,
    pub git_bin: PathBuf,
    pub github_runtime_root: PathBuf,
    pub github_timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let bind_addr = env::var("ICTHUB_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8003".to_owned())
            .parse()?;
        let database_url = env::var("ICTHUB_DATABASE_URL").unwrap_or_else(|_| {
            let path = PathBuf::from("data").join("icthub.db");
            format!(
                "sqlite://{}?mode=rwc",
                path.to_string_lossy().replace('\\', "/")
            )
        });
        let feiyue_auth_url =
            env::var("FEIYUE_AUTH_URL").unwrap_or_else(|_| "http://127.0.0.1:8001".to_owned());
        let import_root = env::var_os("ICTHUB_IMPORT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("uploads").join("imports"));
        let import_max_upload_bytes = env_u64("ICTHUB_IMPORT_MAX_UPLOAD_MB", 500)? * 1024 * 1024;
        let import_max_unpacked_bytes =
            env_u64("ICTHUB_IMPORT_MAX_UNPACKED_MB", 768)? * 1024 * 1024;
        let import_worker_embedded = env_bool("ICTHUB_IMPORT_WORKER_EMBEDDED", true)?;
        let import_worker_poll_ms = env_u64("ICTHUB_IMPORT_WORKER_POLL_MS", 500)?;
        let import_worker_lease_secs = env_u64("ICTHUB_IMPORT_WORKER_LEASE_SECS", 120)?;
        let ffprobe_bin = env::var("ICTHUB_FFPROBE_BIN").unwrap_or_else(|_| "ffprobe".to_owned());
        let ffmpeg_bin = env::var("ICTHUB_FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".to_owned());
        let pdftoppm_bin =
            env::var("ICTHUB_PDFTOPPM_BIN").unwrap_or_else(|_| "pdftoppm".to_owned());
        let codex_enabled = env_bool("ICTHUB_CODEX_ENABLED", false)?;
        let codex_bin = env::var_os("ICTHUB_CODEX_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("codex"));
        let codex_home = env::var_os("ICTHUB_CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data").join("codex-home"));
        let codex_runtime_root = env::var_os("ICTHUB_CODEX_RUNTIME_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data").join("codex-runs"));
        let codex_base_url = non_empty_env("ICTHUB_CODEX_BASE_URL")?;
        let codex_model = non_empty_env("ICTHUB_CODEX_MODEL")?;
        let codex_api_key_file = env::var_os("ICTHUB_CODEX_API_KEY_FILE")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let codex_timeout_secs = env_u64("ICTHUB_CODEX_TIMEOUT_SECS", 600)?;
        let github_enabled = env_bool("ICTHUB_GITHUB_ENABLED", false)?;
        let github_owner = non_empty_env("ICTHUB_GITHUB_OWNER")?;
        let github_repo_prefix = env::var("ICTHUB_GITHUB_REPO_PREFIX")
            .unwrap_or_else(|_| "ict".to_owned())
            .trim()
            .to_ascii_lowercase();
        let github_token_file = env::var_os("ICTHUB_GITHUB_TOKEN_FILE")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let github_cli_bin = env::var_os("ICTHUB_GH_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("gh"));
        let git_bin = env::var_os("ICTHUB_GIT_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("git"));
        let github_runtime_root = env::var_os("ICTHUB_GITHUB_RUNTIME_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data").join("github-runs"));
        let github_timeout_secs = env_u64("ICTHUB_GITHUB_TIMEOUT_SECS", 900)?;
        if codex_enabled && (codex_base_url.is_none() || codex_model.is_none()) {
            return Err(
                "ICTHUB_CODEX_ENABLED=true requires ICTHUB_CODEX_BASE_URL and ICTHUB_CODEX_MODEL"
                    .into(),
            );
        }
        if codex_enabled && codex_api_key_file.is_none() && !codex_home.join("auth.json").is_file()
        {
            return Err("ICTHUB_CODEX_ENABLED=true requires either ICTHUB_CODEX_API_KEY_FILE or ICTHUB_CODEX_HOME/auth.json".into());
        }
        if github_enabled && (github_owner.is_none() || github_token_file.is_none()) {
            return Err(
                "ICTHUB_GITHUB_ENABLED=true requires ICTHUB_GITHUB_OWNER and ICTHUB_GITHUB_TOKEN_FILE"
                    .into(),
            );
        }
        if github_repo_prefix.is_empty()
            || !github_repo_prefix
                .chars()
                .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == '-')
        {
            return Err(
                "ICTHUB_GITHUB_REPO_PREFIX must use lowercase letters, digits, or hyphens".into(),
            );
        }
        Ok(Self {
            bind_addr,
            database_url,
            feiyue_auth_url,
            import_root,
            import_max_upload_bytes,
            import_max_unpacked_bytes,
            import_worker_embedded,
            import_worker_poll_ms,
            import_worker_lease_secs,
            ffprobe_bin,
            ffmpeg_bin,
            pdftoppm_bin,
            codex_enabled,
            codex_bin,
            codex_home,
            codex_runtime_root,
            codex_base_url,
            codex_model,
            codex_api_key_file,
            codex_timeout_secs,
            github_enabled,
            github_owner,
            github_repo_prefix,
            github_token_file,
            github_cli_bin,
            git_bin,
            github_runtime_root,
            github_timeout_secs,
        })
    }
}

fn non_empty_env(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Err(format!("{name} must not be empty").into()),
        Ok(value) => Ok(Some(value.trim().to_owned())),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(Box::new(error)),
    }
}

fn env_bool(name: &str, default: bool) -> Result<bool, Box<dyn std::error::Error>> {
    match env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(format!("{name} must be a boolean").into()),
        },
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(Box::new(error)),
    }
}

fn env_u64(name: &str, default: u64) -> Result<u64, Box<dyn std::error::Error>> {
    match env::var(name) {
        Ok(value) => Ok(value.parse()?),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(Box::new(error)),
    }
}
