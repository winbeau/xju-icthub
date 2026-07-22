use std::path::Path;

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

use crate::{auth::FeiyueIdentityClient, config::Config};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub identity: FeiyueIdentityClient,
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        if let Some(path) = sqlite_file_path(&config.database_url) {
            if let Some(parent) = Path::new(&path).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let db = SqlitePoolOptions::new()
            .max_connections(8)
            .connect(&config.database_url)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON").execute(&db).await?;
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&db)
            .await?;
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&db)
            .await?;
        sqlx::migrate!("../../migrations").run(&db).await?;

        Ok(Self {
            db,
            identity: FeiyueIdentityClient::new(config.feiyue_auth_url.clone()),
        })
    }

    #[cfg(test)]
    pub async fn for_test() -> anyhow::Result<Self> {
        Self::for_test_with_identity_url("http://127.0.0.1:9").await
    }

    #[cfg(test)]
    pub async fn for_test_with_identity_url(identity_url: &str) -> anyhow::Result<Self> {
        let db = SqlitePoolOptions::new().connect("sqlite::memory:").await?;
        sqlx::query("PRAGMA foreign_keys = ON").execute(&db).await?;
        sqlx::migrate!("../../migrations").run(&db).await?;
        Ok(Self {
            db,
            identity: FeiyueIdentityClient::new(identity_url.to_owned()),
        })
    }
}

fn sqlite_file_path(url: &str) -> Option<String> {
    let path = url.strip_prefix("sqlite://")?.split('?').next()?.to_owned();
    if path == ":memory:" {
        None
    } else {
        Some(path)
    }
}
