use std::{
    ffi::OsStr,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, Context};
use async_trait::async_trait;
use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use sqlx::FromRow;
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    auth::AuthContext,
    error::AppError,
    imports::{insert_event, load_job, ImportJobResponse, ImportWorkerOptions},
    state::AppState,
};

const GITHUB_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_SECRET_SCAN_BYTES: u64 = 2 * 1024 * 1024;
const MAX_REPO_NAME_CHARS: usize = 100;

#[derive(Clone, Debug)]
pub(crate) struct GitHubPublishConfig {
    pub enabled: bool,
    pub owner: Option<String>,
    pub repo_prefix: String,
    pub token_file: Option<PathBuf>,
    pub gh_bin: PathBuf,
    pub git_bin: PathBuf,
    pub runtime_root: PathBuf,
    pub timeout: Duration,
}

#[derive(Clone, Debug)]
pub(crate) struct GitHubPublishRequest {
    pub publication_id: String,
    pub job_id: String,
    pub owner: String,
    pub repo_name: String,
    pub source_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct GitHubPublishOutcome {
    pub repo_url: String,
    pub commit_sha: String,
}

#[async_trait]
pub(crate) trait GitHubPublisher: Send + Sync {
    fn enabled(&self) -> bool;
    fn owner(&self) -> Option<&str>;
    fn repo_prefix(&self) -> &str;

    async fn publish(&self, request: GitHubPublishRequest) -> anyhow::Result<GitHubPublishOutcome>;
}

#[derive(Clone, Debug)]
pub(crate) struct GhCliPublisher {
    config: Arc<GitHubPublishConfig>,
}

impl GhCliPublisher {
    pub(crate) fn new(config: GitHubPublishConfig) -> anyhow::Result<Self> {
        if config.enabled {
            let owner = config.owner.as_deref().context("GitHub owner is missing")?;
            if !valid_owner(owner) {
                bail!("GitHub owner contains unsupported characters");
            }
            if config.token_file.is_none() {
                bail!("GitHub token file is missing");
            }
        }
        Ok(Self {
            config: Arc::new(config),
        })
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self {
            config: Arc::new(GitHubPublishConfig {
                enabled: false,
                owner: None,
                repo_prefix: "ict".to_owned(),
                token_file: None,
                gh_bin: PathBuf::from("gh"),
                git_bin: PathBuf::from("git"),
                runtime_root: PathBuf::from("data/github-runs"),
                timeout: Duration::from_secs(900),
            }),
        }
    }

    async fn token(&self) -> anyhow::Result<String> {
        let path = self
            .config
            .token_file
            .as_deref()
            .context("GitHub token file is not configured")?;
        validate_token_file_permissions(path)?;
        let token = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("cannot read GitHub token file {}", path.display()))?;
        let token = token.trim().to_owned();
        if token.is_empty() || token.contains(char::is_whitespace) {
            bail!("GitHub token file must contain one non-empty token");
        }
        Ok(token)
    }

    async fn run_command(
        &self,
        program: &Path,
        args: &[&OsStr],
        cwd: &Path,
        token: Option<&str>,
        label: &str,
    ) -> anyhow::Result<std::process::Output> {
        let output = self
            .command_output(program, args, cwd, token, label)
            .await?;
        if !output.status.success() {
            let detail = command_error_detail(&output.stderr, token);
            bail!("{label} failed with status {}: {detail}", output.status);
        }
        Ok(output)
    }

    async fn run_authenticated_git(
        &self,
        args: &[&OsStr],
        cwd: &Path,
        token: &str,
        label: &str,
    ) -> anyhow::Result<std::process::Output> {
        let mut command = Command::new(&self.config.git_bin);
        command
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("GH_TOKEN", token)
            .env("GITHUB_TOKEN", token)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "credential.https://github.com.helper")
            .env("GIT_CONFIG_VALUE_0", "!gh auth git-credential");
        let output = tokio::time::timeout(self.config.timeout, command.output())
            .await
            .with_context(|| format!("{label} timed out"))?
            .with_context(|| format!("failed to start {label}"))?;
        if !output.status.success() {
            let detail = command_error_detail(&output.stderr, Some(token));
            bail!("{label} failed with status {}: {detail}", output.status);
        }
        Ok(output)
    }

    async fn push_repository(
        &self,
        full_name: &str,
        staging_dir: &Path,
        token: &str,
    ) -> anyhow::Result<()> {
        let remote_url = format!("https://github.com/{full_name}.git");
        self.run_command(
            &self.config.git_bin,
            &[
                OsStr::new("remote"),
                OsStr::new("add"),
                OsStr::new("origin"),
                OsStr::new(&remote_url),
            ],
            staging_dir,
            None,
            "git remote add",
        )
        .await?;
        self.run_authenticated_git(
            &[
                OsStr::new("push"),
                OsStr::new("--set-upstream"),
                OsStr::new("origin"),
                OsStr::new("main"),
            ],
            staging_dir,
            token,
            "git push",
        )
        .await?;
        Ok(())
    }

    async fn command_output(
        &self,
        program: &Path,
        args: &[&OsStr],
        cwd: &Path,
        token: Option<&str>,
        label: &str,
    ) -> anyhow::Result<std::process::Output> {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(token) = token {
            command.env("GH_TOKEN", token).env("GITHUB_TOKEN", token);
        }
        let output = tokio::time::timeout(self.config.timeout, command.output())
            .await
            .with_context(|| format!("{label} timed out"))?
            .with_context(|| format!("failed to start {label}"))?;
        Ok(output)
    }
}

#[async_trait]
impl GitHubPublisher for GhCliPublisher {
    fn enabled(&self) -> bool {
        self.config.enabled
    }

    fn owner(&self) -> Option<&str> {
        self.config.owner.as_deref()
    }

    fn repo_prefix(&self) -> &str {
        &self.config.repo_prefix
    }

    async fn publish(&self, request: GitHubPublishRequest) -> anyhow::Result<GitHubPublishOutcome> {
        if !self.enabled() {
            bail!("GitHub publisher is disabled");
        }
        let token = self.token().await?;
        let staging_dir = self.config.runtime_root.join(&request.publication_id);
        let source_dir = request.source_dir.clone();
        let staging_for_copy = staging_dir.clone();
        tokio::task::spawn_blocking(move || prepare_staging(&source_dir, &staging_for_copy))
            .await
            .context("GitHub staging task failed")??;

        let result = async {
            self.run_command(
                &self.config.git_bin,
                &[OsStr::new("init"), OsStr::new("-b"), OsStr::new("main")],
                &staging_dir,
                None,
                "git init",
            )
            .await?;
            self.run_command(
                &self.config.git_bin,
                &[
                    OsStr::new("config"),
                    OsStr::new("user.name"),
                    OsStr::new("ICTHub Importer"),
                ],
                &staging_dir,
                None,
                "git config user.name",
            )
            .await?;
            self.run_command(
                &self.config.git_bin,
                &[
                    OsStr::new("config"),
                    OsStr::new("user.email"),
                    OsStr::new("icthub@icthub.top"),
                ],
                &staging_dir,
                None,
                "git config user.email",
            )
            .await?;
            self.run_command(
                &self.config.git_bin,
                &[OsStr::new("add"), OsStr::new("--all")],
                &staging_dir,
                None,
                "git add",
            )
            .await?;
            self.run_command(
                &self.config.git_bin,
                &[
                    OsStr::new("commit"),
                    OsStr::new("-m"),
                    OsStr::new("Import project sources"),
                ],
                &staging_dir,
                None,
                "git commit",
            )
            .await?;
            let sha_output = self
                .run_command(
                    &self.config.git_bin,
                    &[OsStr::new("rev-parse"), OsStr::new("HEAD")],
                    &staging_dir,
                    None,
                    "git rev-parse",
                )
                .await?;
            let commit_sha = String::from_utf8(sha_output.stdout)
                .context("git returned a non-UTF-8 commit id")?
                .trim()
                .to_owned();
            let full_name = format!("{}/{}", request.owner, request.repo_name);
            let description = format!("ICTHub import job {}", request.job_id);
            let existing = self
                .command_output(
                    &self.config.gh_bin,
                    &[
                        OsStr::new("repo"),
                        OsStr::new("view"),
                        OsStr::new(&full_name),
                        OsStr::new("--json"),
                        OsStr::new("description,isPrivate,url,defaultBranchRef"),
                    ],
                    &staging_dir,
                    Some(&token),
                    "gh repo view",
                )
                .await?;
            if existing.status.success() {
                let metadata: serde_json::Value = serde_json::from_slice(&existing.stdout)
                    .context("gh repo view returned invalid JSON")?;
                let same_job = metadata["description"].as_str() == Some(description.as_str());
                let is_private = metadata["isPrivate"].as_bool() == Some(true);
                let repo_url = metadata["url"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("https://github.com/{full_name}"));
                if same_job && is_private {
                    if repository_is_empty(&metadata) {
                        self.push_repository(&full_name, &staging_dir, &token)
                            .await?;
                        return Ok(GitHubPublishOutcome {
                            repo_url,
                            commit_sha,
                        });
                    }
                    let commits_endpoint = format!("repos/{full_name}/commits/HEAD");
                    let remote_head = self
                        .run_command(
                            &self.config.gh_bin,
                            &[
                                OsStr::new("api"),
                                OsStr::new(&commits_endpoint),
                                OsStr::new("--jq"),
                                OsStr::new(".sha"),
                            ],
                            &staging_dir,
                            Some(&token),
                            "gh api repository head",
                        )
                        .await?;
                    let remote_sha = String::from_utf8(remote_head.stdout)
                        .context("gh returned a non-UTF-8 commit id")?
                        .trim()
                        .to_owned();
                    return Ok(GitHubPublishOutcome {
                        repo_url,
                        commit_sha: remote_sha,
                    });
                }
                bail!("GitHub repository name already exists and belongs to another source");
            }
            self.run_command(
                &self.config.gh_bin,
                &[
                    OsStr::new("repo"),
                    OsStr::new("create"),
                    OsStr::new(&full_name),
                    OsStr::new("--private"),
                    OsStr::new("--description"),
                    OsStr::new(&description),
                ],
                &staging_dir,
                Some(&token),
                "gh repo create",
            )
            .await?;
            self.push_repository(&full_name, &staging_dir, &token)
                .await?;
            Ok(GitHubPublishOutcome {
                repo_url: format!("https://github.com/{full_name}"),
                commit_sha,
            })
        }
        .await;

        let cleanup_dir = staging_dir.clone();
        let cleanup = tokio::task::spawn_blocking(move || {
            if cleanup_dir.exists() {
                fs::remove_dir_all(cleanup_dir)?;
            }
            Ok::<_, std::io::Error>(())
        })
        .await;
        match cleanup {
            Err(error) => tracing::warn!(error = %error, "GitHub staging cleanup task failed"),
            Ok(Err(error)) => {
                tracing::warn!(error = %error, "GitHub staging directory cleanup failed")
            }
            Ok(Ok(())) => {}
        }
        result
    }
}

#[derive(Clone, Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHubPublicationView {
    id: String,
    owner: String,
    repo_number: i64,
    repo_name: String,
    repo_url: Option<String>,
    source_ref: String,
    status: String,
    error_message: Option<String>,
    commit_sha: Option<String>,
    attempt_count: i64,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(Debug, FromRow)]
struct ClaimedPublication {
    id: String,
    job_id: String,
    owner: String,
    repo_name: String,
    source_ref: String,
}

pub(crate) async fn queue_publication(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    AxumPath(id): AxumPath<String>,
) -> Result<(StatusCode, Json<ImportJobResponse>), AppError> {
    if !identity.can_access_icthub() {
        return Err(AppError::Forbidden);
    }
    let (status, created_by_sid, result_json, agent_result_json) =
        sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(
            "SELECT status, created_by_sid, result_json, agent_result_json
             FROM import_jobs WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    if created_by_sid != identity.sid && !identity.is_superadmin() {
        return Err(AppError::Forbidden);
    }
    if status != "completed" {
        return Err(AppError::Conflict(
            "请等待 Codex 整理完成后再创建源码仓库".to_owned(),
        ));
    }
    if !state.github_publisher.enabled() {
        return Err(AppError::Conflict("GitHub 私有仓库发布尚未配置".to_owned()));
    }
    let owner = state
        .github_publisher
        .owner()
        .ok_or_else(|| AppError::Conflict("GitHub 组织尚未配置".to_owned()))?
        .to_owned();
    let source_ref = source_ref_from_results(agent_result_json.as_deref(), result_json.as_deref())
        .ok_or_else(|| AppError::Conflict("Codex 尚未识别出可发布的源码目录".to_owned()))?;
    let project_slug = result_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .and_then(|value| value["projectDraft"]["slug"].as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("import-{}", &id[..8.min(id.len())]));

    let mut tx = state.db.begin().await?;
    if let Some((publication_id, publication_status)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id, status FROM github_publications WHERE job_id = ?",
    )
    .bind(&id)
    .fetch_optional(&mut *tx)
    .await?
    {
        if publication_status == "failed" {
            sqlx::query(
                "UPDATE github_publications SET status = 'queued', error_message = NULL,
                    worker_id = NULL, lease_expires_at = NULL, completed_at = NULL,
                    updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(publication_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        return Ok((StatusCode::ACCEPTED, Json(load_job(&state, &id).await?)));
    }
    let repo_number = sqlx::query_scalar::<_, i64>(
        "INSERT INTO github_repo_sequences (owner, next_number) VALUES (?, 2)
         ON CONFLICT(owner) DO UPDATE SET next_number = next_number + 1,
            updated_at = CURRENT_TIMESTAMP
         RETURNING next_number - 1",
    )
    .bind(&owner)
    .fetch_one(&mut *tx)
    .await?;
    let repo_name = numbered_repo_name(&state, repo_number, &project_slug);
    sqlx::query(
        "INSERT INTO github_publications (
            id, job_id, requested_by_sid, owner, repo_number, repo_name, source_ref, status
         ) VALUES (?, ?, ?, ?, ?, ?, ?, 'queued')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&id)
    .bind(&identity.sid)
    .bind(&owner)
    .bind(repo_number)
    .bind(&repo_name)
    .bind(&source_ref)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    insert_event(
        &state,
        &id,
        "github_publish_queued",
        "completed",
        "等待确认",
        100,
        Some(&format!("私有源码仓库 {owner}/{repo_name} 已排队")),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(load_job(&state, &id).await?)))
}

pub(crate) async fn load_publication(
    state: &AppState,
    job_id: &str,
) -> Result<Option<GitHubPublicationView>, AppError> {
    Ok(sqlx::query_as::<_, GitHubPublicationView>(
        "SELECT id, owner, repo_number, repo_name, repo_url, source_ref, status,
                error_message, commit_sha, attempt_count, created_at, updated_at,
                started_at, completed_at
         FROM github_publications WHERE job_id = ?",
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?)
}

pub(crate) async fn process_one_queued_publication(
    state: &AppState,
    options: &ImportWorkerOptions,
) -> anyhow::Result<bool> {
    if !state.github_publisher.enabled() {
        return Ok(false);
    }
    let lease_modifier = format!("+{} seconds", options.lease_duration.as_secs());
    let claimed = sqlx::query_as::<_, ClaimedPublication>(
        "UPDATE github_publications SET status = 'running', worker_id = ?,
            lease_expires_at = datetime('now', ?), attempt_count = attempt_count + 1,
            error_message = NULL, started_at = COALESCE(started_at, CURRENT_TIMESTAMP),
            updated_at = CURRENT_TIMESTAMP
         WHERE id = (
            SELECT id FROM github_publications
             WHERE status = 'queued'
                OR (status = 'running' AND lease_expires_at IS NOT NULL
                    AND lease_expires_at <= CURRENT_TIMESTAMP)
             ORDER BY created_at ASC LIMIT 1
         )
         RETURNING id, job_id, owner, repo_name, source_ref",
    )
    .bind(&options.worker_id)
    .bind(lease_modifier)
    .fetch_optional(&state.db)
    .await?;
    let Some(claimed) = claimed else {
        return Ok(false);
    };
    insert_event(
        state,
        &claimed.job_id,
        "github_publish_started",
        "completed",
        "等待确认",
        100,
        Some(&format!(
            "正在创建私有源码仓库 {}/{}",
            claimed.owner, claimed.repo_name
        )),
    )
    .await?;

    let heartbeat_state = state.clone();
    let heartbeat_id = claimed.id.clone();
    let heartbeat_worker_id = options.worker_id.clone();
    let heartbeat_lease = options.lease_duration;
    let heartbeat_interval = Duration::from_secs((heartbeat_lease.as_secs() / 3).max(10));
    let heartbeat = tokio::spawn(async move {
        loop {
            tokio::time::sleep(heartbeat_interval).await;
            let lease_modifier = format!("+{} seconds", heartbeat_lease.as_secs());
            let updated = sqlx::query(
                "UPDATE github_publications SET lease_expires_at = datetime('now', ?),
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ? AND worker_id = ? AND status = 'running'",
            )
            .bind(lease_modifier)
            .bind(&heartbeat_id)
            .bind(&heartbeat_worker_id)
            .execute(&heartbeat_state.db)
            .await;
            match updated {
                Ok(result) if result.rows_affected() == 1 => {}
                _ => break,
            }
        }
    });

    let source_dir = resolve_source_root(&state.import_root, &claimed.job_id, &claimed.source_ref);
    let result = match source_dir {
        Ok(source_dir) => {
            state
                .github_publisher
                .publish(GitHubPublishRequest {
                    publication_id: claimed.id.clone(),
                    job_id: claimed.job_id.clone(),
                    owner: claimed.owner.clone(),
                    repo_name: claimed.repo_name.clone(),
                    source_dir,
                })
                .await
        }
        Err(error) => Err(error),
    };
    heartbeat.abort();

    match result {
        Ok(outcome) => {
            let updated = sqlx::query(
                "UPDATE github_publications SET status = 'completed', repo_url = ?,
                    commit_sha = ?, error_message = NULL, worker_id = NULL,
                    lease_expires_at = NULL, completed_at = CURRENT_TIMESTAMP,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ? AND worker_id = ? AND status = 'running'",
            )
            .bind(&outcome.repo_url)
            .bind(&outcome.commit_sha)
            .bind(&claimed.id)
            .bind(&options.worker_id)
            .execute(&state.db)
            .await?;
            if updated.rows_affected() != 1 {
                bail!("GitHub publication lease was lost before completion");
            }
            insert_event(
                state,
                &claimed.job_id,
                "github_publish_completed",
                "completed",
                "等待确认",
                100,
                Some(&format!("私有源码仓库已创建：{}", outcome.repo_url)),
            )
            .await?;
        }
        Err(error) => {
            tracing::error!(
                publication_id = %claimed.id,
                job_id = %claimed.job_id,
                error = %error,
                "GitHub publication failed"
            );
            let message = user_facing_publish_error(&error);
            sqlx::query(
                "UPDATE github_publications SET status = 'failed', error_message = ?,
                    worker_id = NULL, lease_expires_at = NULL, completed_at = CURRENT_TIMESTAMP,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ? AND worker_id = ?",
            )
            .bind(&message)
            .bind(&claimed.id)
            .bind(&options.worker_id)
            .execute(&state.db)
            .await?;
            insert_event(
                state,
                &claimed.job_id,
                "github_publish_failed",
                "completed",
                "等待确认",
                100,
                Some(&message),
            )
            .await?;
        }
    }
    Ok(true)
}

fn source_ref_from_results(agent_result: Option<&str>, result: Option<&str>) -> Option<String> {
    let from_value = |value: &serde_json::Value, pointer: &str| {
        value
            .pointer(pointer)
            .and_then(serde_json::Value::as_array)
            .and_then(|resources| resources.first())
            .and_then(|resource| resource["sourceRef"].as_str())
            .map(str::to_owned)
    };
    agent_result
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
        .and_then(|value| from_value(&value, "/resources/sourceCode"))
        .or_else(|| {
            result
                .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                .and_then(|value| from_value(&value, "/normalizedResources/sourceCode"))
        })
}

fn numbered_repo_name(state: &AppState, number: i64, project_slug: &str) -> String {
    let prefix = state.github_publisher.repo_prefix();
    let base = format!("{prefix}-{number:04}-{project_slug}");
    base.chars().take(MAX_REPO_NAME_CHARS).collect()
}

fn repository_is_empty(metadata: &serde_json::Value) -> bool {
    metadata
        .pointer("/defaultBranchRef/name")
        .and_then(serde_json::Value::as_str)
        .is_none_or(str::is_empty)
}

fn command_error_detail(stderr: &[u8], token: Option<&str>) -> String {
    let mut detail = String::from_utf8_lossy(stderr).trim().to_owned();
    if let Some(token) = token.filter(|token| !token.is_empty()) {
        detail = detail.replace(token, "[redacted]");
    }
    if detail.is_empty() {
        return "no stderr output".to_owned();
    }
    detail.chars().take(2_000).collect()
}

fn resolve_source_root(
    import_root: &Path,
    job_id: &str,
    source_ref: &str,
) -> anyhow::Result<PathBuf> {
    if source_ref.starts_with("http://") || source_ref.starts_with("https://") {
        bail!("remote GitHub sources are not implemented in this milestone");
    }
    let relative = PathBuf::from(source_ref.replace('\\', "/"));
    if !safe_relative_path(&relative) {
        bail!("source reference contains an unsafe path");
    }
    let extracted = import_root.join(job_id).join("extracted");
    let mut candidate = extracted.join(&relative);
    if is_archive(&candidate) {
        let stem = candidate
            .file_stem()
            .and_then(OsStr::to_str)
            .context("source archive has no valid name")?;
        candidate.set_file_name(format!("{stem}.__contents"));
    } else if candidate.is_file() {
        candidate = candidate
            .parent()
            .context("source file has no parent directory")?
            .to_owned();
    }
    let extracted = extracted
        .canonicalize()
        .context("normalized extraction directory is missing")?;
    let mut candidate = candidate
        .canonicalize()
        .context("Codex source directory is missing")?;
    if !candidate.starts_with(&extracted) || !candidate.is_dir() {
        bail!("source directory is outside the normalized import tree");
    }
    let entries = fs::read_dir(&candidate)?.collect::<Result<Vec<_>, _>>()?;
    let visible = entries
        .into_iter()
        .filter(|entry| entry.file_name() != OsStr::new("__MACOSX"))
        .collect::<Vec<_>>();
    if visible.len() == 1 && visible[0].file_type()?.is_dir() {
        candidate = visible[0].path().canonicalize()?;
    }
    Ok(candidate)
}

fn prepare_staging(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination).context("cannot reset GitHub staging directory")?;
    }
    fs::create_dir_all(destination).context("cannot create GitHub staging directory")?;
    let mut copied_files = 0_u64;
    copy_safe_tree(source, source, destination, &mut copied_files)?;
    if copied_files == 0 {
        bail!("source directory contains no publishable files");
    }
    Ok(())
}

fn copy_safe_tree(
    root: &Path,
    current: &Path,
    destination: &Path,
    copied_files: &mut u64,
) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(current)?;
    if metadata.file_type().is_symlink() {
        bail!("source tree contains a symbolic link");
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let name = entry.file_name();
        let source_path = entry.path();
        let relative = source_path.strip_prefix(root)?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            bail!(
                "source tree contains a symbolic link at {}",
                relative.display()
            );
        }
        if file_type.is_dir() {
            if ignored_directory(&name) {
                continue;
            }
            let next_destination = destination.join(relative);
            fs::create_dir_all(&next_destination)?;
            copy_safe_tree(root, &source_path, destination, copied_files)?;
            continue;
        }
        if !file_type.is_file() {
            bail!(
                "source tree contains an unsupported file at {}",
                relative.display()
            );
        }
        let metadata = entry.metadata()?;
        if metadata.len() > GITHUB_MAX_FILE_BYTES {
            bail!(
                "source file is larger than GitHub's 100 MiB limit: {}",
                relative.display()
            );
        }
        scan_for_secrets(&source_path, relative, metadata.len())?;
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source_path, target)?;
        *copied_files += 1;
    }
    Ok(())
}

fn scan_for_secrets(path: &Path, relative: &Path, size: u64) -> anyhow::Result<()> {
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let sensitive_name = (name == ".env" || name.starts_with(".env."))
        && !name.ends_with(".example")
        || matches!(
            name.as_str(),
            "id_rsa"
                | "id_ed25519"
                | "auth.json"
                | "credentials"
                | "credentials.json"
                | "service-account.json"
        )
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".p12")
        || name.ends_with(".pfx");
    if sensitive_name {
        bail!(
            "source tree contains a secret-like file: {}",
            relative.display()
        );
    }
    if size > MAX_SECRET_SCAN_BYTES {
        return Ok(());
    }
    let mut input = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(size as usize);
    input.read_to_end(&mut bytes)?;
    if bytes.contains(&0) {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
    let secret_markers = [
        "-----begin private key-----",
        "-----begin rsa private key-----",
        "github_pat_",
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "ghr_",
        "openai_api_key=",
        "github_token=",
        "gh_token=",
        "sk-proj-",
    ];
    if secret_markers.iter().any(|marker| text.contains(marker)) {
        bail!(
            "source tree contains secret-like content: {}",
            relative.display()
        );
    }
    Ok(())
}

fn ignored_directory(name: &OsStr) -> bool {
    matches!(
        name.to_string_lossy().to_ascii_lowercase().as_str(),
        ".git"
            | "node_modules"
            | "target"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".next"
            | "dist"
            | "build"
    )
}

fn is_archive(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "zip" | "7z" | "rar"
            )
        })
}

fn safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_owner(owner: &str) -> bool {
    !owner.is_empty()
        && owner.len() <= 100
        && owner
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '-')
}

fn validate_token_file_permissions(path: &Path) -> anyhow::Result<()> {
    if !path.is_file() {
        bail!("GitHub token file does not exist");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)?.permissions().mode();
        if mode & 0o077 != 0 {
            bail!("GitHub token file permissions must be 0600 or stricter");
        }
    }
    Ok(())
}

fn user_facing_publish_error(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("secret-like") {
        format!("发布已停止：{message}")
    } else if message.contains("symbolic link") {
        "发布已停止：源码中包含符号链接，请人工确认后重试。".to_owned()
    } else if message.contains("100 MiB") {
        format!("发布已停止：{message}")
    } else if message.contains("token file") {
        "GitHub 凭据文件不可用，请管理员检查配置。".to_owned()
    } else if message.contains("failed to start gh repo create") {
        "服务器尚未安装或无法启动 GitHub CLI。".to_owned()
    } else if message.contains("gh repo create failed") {
        "GitHub 未能创建或推送私有仓库，请检查账号权限、组织授权和仓库名。".to_owned()
    } else if message.contains("belongs to another source") {
        "自动编号对应的仓库名已被其他项目占用，请管理员核对组织仓库后重试。".to_owned()
    } else if message.contains("source directory") || message.contains("source reference") {
        "未找到安全、完整的源码目录，请在材料中补充源码包后重试。".to_owned()
    } else {
        "私有源码仓库发布失败，请管理员查看 Worker 日志后重试。".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn staging_removes_git_and_generated_directories() {
        let root = tempdir().expect("root");
        let source = root.path().join("source");
        let destination = root.path().join("staging");
        fs::create_dir_all(source.join(".git")).expect("git directory");
        fs::create_dir_all(source.join("node_modules/pkg")).expect("dependencies");
        fs::create_dir_all(source.join("src")).expect("source directory");
        fs::write(source.join(".git/config"), "secret remote").expect("git config");
        fs::write(source.join("node_modules/pkg/index.js"), "generated").expect("dependency");
        fs::write(source.join("src/main.rs"), "fn main() {}").expect("source");

        prepare_staging(&source, &destination).expect("prepare staging");

        assert!(destination.join("src/main.rs").is_file());
        assert!(!destination.join(".git").exists());
        assert!(!destination.join("node_modules").exists());
    }

    #[test]
    fn staging_rejects_secret_like_files() {
        let root = tempdir().expect("root");
        let source = root.path().join("source");
        let destination = root.path().join("staging");
        fs::create_dir_all(&source).expect("source directory");
        fs::write(source.join(".env"), "GITHUB_TOKEN=do-not-publish").expect("secret");

        let error = prepare_staging(&source, &destination).expect_err("secret rejection");
        assert!(error.to_string().contains("secret-like"));
    }

    #[test]
    fn source_ref_prefers_codex_source_code() {
        let agent = serde_json::json!({
            "resources": {"sourceCode": [{"sourceRef": "project/source.zip"}]}
        });
        assert_eq!(
            source_ref_from_results(Some(&agent.to_string()), None).as_deref(),
            Some("project/source.zip")
        );
    }

    #[test]
    fn empty_github_repository_is_detected_for_recovery_push() {
        let metadata = serde_json::json!({
            "description": "ICTHub import job example",
            "isPrivate": true,
            "defaultBranchRef": {"name": ""}
        });

        assert!(repository_is_empty(&metadata));
        assert!(!repository_is_empty(&serde_json::json!({
            "defaultBranchRef": {"name": "main"}
        })));
    }

    #[test]
    fn command_errors_redact_the_github_token() {
        let detail = command_error_detail(
            b"authentication failed for github_pat_secret_value",
            Some("github_pat_secret_value"),
        );

        assert_eq!(detail, "authentication failed for [redacted]");
    }

    #[test]
    fn enabled_publisher_requires_a_token_file() {
        let config = GitHubPublishConfig {
            enabled: true,
            owner: Some("xjuIcthub".to_owned()),
            repo_prefix: "ict".to_owned(),
            token_file: None,
            gh_bin: PathBuf::from("gh"),
            git_bin: PathBuf::from("git"),
            runtime_root: PathBuf::from("data/github-runs"),
            timeout: Duration::from_secs(60),
        };

        assert!(GhCliPublisher::new(config).is_err());
    }
}
