use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub feiyue_auth_url: String,
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
        Ok(Self {
            bind_addr,
            database_url,
            feiyue_auth_url,
        })
    }
}
