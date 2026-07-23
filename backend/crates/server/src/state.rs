use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

use crate::{
    auth::FeiyueIdentityClient,
    config::Config,
    github::{GhCliPublisher, GitHubPublishConfig, GitHubPublisher},
    imports::agent::{CodexExecConfig, CodexExecRunner, ImportAgentRunner},
};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub identity: FeiyueIdentityClient,
    pub import_root: Arc<PathBuf>,
    pub project_root: Arc<PathBuf>,
    pub preview_public_base_url: Arc<Option<String>>,
    pub import_max_upload_bytes: u64,
    pub import_max_unpacked_bytes: u64,
    pub ffprobe_bin: Arc<String>,
    pub ffmpeg_bin: Arc<String>,
    pub pdftoppm_bin: Arc<String>,
    pub(crate) import_agent: Arc<dyn ImportAgentRunner>,
    pub(crate) github_publisher: Arc<dyn GitHubPublisher>,
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
        std::fs::create_dir_all(&config.import_root)?;
        std::fs::create_dir_all(&config.project_root)?;
        std::fs::create_dir_all(&config.codex_home)?;
        std::fs::create_dir_all(&config.codex_runtime_root)?;
        std::fs::create_dir_all(&config.github_runtime_root)?;

        let import_agent = CodexExecRunner::new(CodexExecConfig {
            enabled: config.codex_enabled,
            binary: config.codex_bin.clone(),
            codex_home: config.codex_home.clone(),
            runtime_root: config.codex_runtime_root.clone(),
            base_url: config.codex_base_url.clone(),
            model: config.codex_model.clone(),
            api_key_file: config.codex_api_key_file.clone(),
            timeout: std::time::Duration::from_secs(config.codex_timeout_secs.max(30)),
        })?;
        let github_publisher = GhCliPublisher::new(GitHubPublishConfig {
            enabled: config.github_enabled,
            owner: config.github_owner.clone(),
            repo_prefix: config.github_repo_prefix.clone(),
            token_file: config.github_token_file.clone(),
            gh_bin: config.github_cli_bin.clone(),
            git_bin: config.git_bin.clone(),
            runtime_root: config.github_runtime_root.clone(),
            timeout: std::time::Duration::from_secs(config.github_timeout_secs.max(60)),
        })?;

        Ok(Self {
            db,
            identity: FeiyueIdentityClient::new(config.feiyue_auth_url.clone()),
            import_root: Arc::new(config.import_root.clone()),
            project_root: Arc::new(config.project_root.clone()),
            preview_public_base_url: Arc::new(config.preview_public_base_url.clone()),
            import_max_upload_bytes: config.import_max_upload_bytes,
            import_max_unpacked_bytes: config.import_max_unpacked_bytes,
            ffprobe_bin: Arc::new(config.ffprobe_bin.clone()),
            ffmpeg_bin: Arc::new(config.ffmpeg_bin.clone()),
            pdftoppm_bin: Arc::new(config.pdftoppm_bin.clone()),
            import_agent: Arc::new(import_agent),
            github_publisher: Arc::new(github_publisher),
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
        let import_root =
            std::env::temp_dir().join(format!("icthub-import-test-{}", uuid::Uuid::new_v4()));
        let project_root =
            std::env::temp_dir().join(format!("icthub-project-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&import_root)?;
        std::fs::create_dir_all(&project_root)?;
        Ok(Self {
            db,
            identity: FeiyueIdentityClient::new(identity_url.to_owned()),
            import_root: Arc::new(import_root),
            project_root: Arc::new(project_root),
            preview_public_base_url: Arc::new(None),
            import_max_upload_bytes: 16 * 1024 * 1024,
            import_max_unpacked_bytes: 64 * 1024 * 1024,
            ffprobe_bin: Arc::new("ffprobe-not-installed-for-tests".to_owned()),
            ffmpeg_bin: Arc::new("ffmpeg-not-installed-for-tests".to_owned()),
            pdftoppm_bin: Arc::new("pdftoppm-not-installed-for-tests".to_owned()),
            import_agent: Arc::new(CodexExecRunner::disabled()),
            github_publisher: Arc::new(GhCliPublisher::disabled()),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_import_agent(mut self, runner: Arc<dyn ImportAgentRunner>) -> Self {
        self.import_agent = runner;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_github_publisher(mut self, publisher: Arc<dyn GitHubPublisher>) -> Self {
        self.github_publisher = publisher;
        self
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
