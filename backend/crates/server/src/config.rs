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
        let import_max_upload_bytes = env_u64("ICTHUB_IMPORT_MAX_UPLOAD_MB", 256)? * 1024 * 1024;
        let import_max_unpacked_bytes =
            env_u64("ICTHUB_IMPORT_MAX_UNPACKED_MB", 768)? * 1024 * 1024;
        let import_worker_embedded = env_bool("ICTHUB_IMPORT_WORKER_EMBEDDED", true)?;
        let import_worker_poll_ms = env_u64("ICTHUB_IMPORT_WORKER_POLL_MS", 500)?;
        let import_worker_lease_secs = env_u64("ICTHUB_IMPORT_WORKER_LEASE_SECS", 120)?;
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
        })
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
