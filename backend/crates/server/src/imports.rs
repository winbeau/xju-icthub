use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, Context};
use axum::{
    extract::{Multipart, Path as AxumPath, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Sqlite, Transaction};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{auth::AuthContext, error::AppError, state::AppState};

mod extractors;

use extractors::extract_artifact;

const MAX_ARCHIVE_ENTRIES: usize = 5_000;
const MAX_SINGLE_FILE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TEXT_CORPUS_BYTES: usize = 512 * 1024;
const MAX_VISIBLE_ARTIFACTS: i64 = 500;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkInput {
    url: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefinementInput {
    prompt: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportJobResponse {
    id: String,
    status: String,
    stage: String,
    progress: i64,
    source_kind: String,
    source_name: String,
    analysis_engine: String,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
    attempt_count: i64,
    started_at: Option<String>,
    completed_at: Option<String>,
    inputs: Vec<ImportInputView>,
    artifacts: Vec<ImportArtifactView>,
    events: Vec<ImportJobEventView>,
    result: Option<ImportAnalysis>,
}

#[derive(Clone, Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct ImportJobEventView {
    id: i64,
    event_type: String,
    status: String,
    stage: String,
    progress: i64,
    message: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct ImportInputView {
    id: String,
    input_kind: String,
    provider: String,
    display_name: String,
    source_ref: Option<String>,
    mime_type: Option<String>,
    size_bytes: Option<i64>,
    status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportArtifactView {
    id: String,
    relative_path: String,
    artifact_kind: String,
    mime_type: Option<String>,
    size_bytes: i64,
    extractor: String,
    metadata: serde_json::Value,
    is_cover_candidate: bool,
}

#[derive(Clone, Debug, FromRow)]
struct ImportArtifactRow {
    id: String,
    relative_path: String,
    artifact_kind: String,
    mime_type: Option<String>,
    size_bytes: i64,
    extractor: String,
    metadata_json: String,
    is_cover_candidate: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportAnalysis {
    project_draft: ImportProjectDraft,
    artifact_summary: Vec<ArtifactSummary>,
    warnings: Vec<String>,
    agent: AgentState,
    capabilities: ImportCapabilities,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportProjectDraft {
    name: String,
    slug: String,
    summary: String,
    primary_category: String,
    suggested_tags: Vec<String>,
    owner_name: Option<String>,
    source_name: Option<String>,
    highest_award: Option<String>,
    status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactSummary {
    kind: String,
    count: usize,
    total_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentState {
    status: String,
    mode: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportCapabilities {
    zip_upload: String,
    github_link: String,
    mixed_files: String,
    codex_agent: String,
    github_publish: String,
}

#[derive(Debug, FromRow)]
struct ImportJobRow {
    id: String,
    status: String,
    stage: String,
    progress: i64,
    source_kind: String,
    source_name: String,
    analysis_engine: String,
    result_json: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
    attempt_count: i64,
    started_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ImportWorkerOptions {
    pub worker_id: String,
    pub poll_interval: Duration,
    pub lease_duration: Duration,
}

impl ImportWorkerOptions {
    pub fn new(poll_ms: u64, lease_secs: u64) -> Self {
        Self {
            worker_id: format!("worker-{}", Uuid::new_v4()),
            poll_interval: Duration::from_millis(poll_ms.max(50)),
            lease_duration: Duration::from_secs(lease_secs.max(30)),
        }
    }
}

struct AnalysisBuild {
    artifacts: Vec<ArtifactRecord>,
    analysis: ImportAnalysis,
}

struct ArtifactRecord {
    relative_path: String,
    artifact_kind: String,
    mime_type: Option<String>,
    size_bytes: u64,
    extractor: String,
    metadata_json: String,
    is_cover_candidate: bool,
}

#[derive(Clone, Debug)]
struct UploadedInput {
    id: String,
    display_name: String,
    local_path: PathBuf,
    mime_type: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, FromRow)]
struct StoredFileInput {
    id: String,
    display_name: String,
    local_path: String,
    mime_type: String,
    size_bytes: i64,
    sha256: String,
}

struct JobDirectoryGuard {
    path: PathBuf,
    keep: bool,
}

impl JobDirectoryGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for JobDirectoryGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImportJobResponse>), AppError> {
    if !identity.can_access_icthub() {
        return Err(AppError::Forbidden);
    }

    let job_id = Uuid::new_v4().to_string();
    let job_dir = state.import_root.join(&job_id);
    let mut job_dir_guard = JobDirectoryGuard::new(job_dir.clone());
    let source_dir = job_dir.join("source");
    tokio::fs::create_dir_all(&source_dir).await?;
    let mut uploads = Vec::<UploadedInput>::new();
    let mut total_upload_size = 0_u64;
    let mut links = Vec::<LinkInput>::new();
    let mut prompt = String::new();

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::BadRequest(format!("无法读取上传内容：{error}")))?
    {
        match field.name() {
            Some("file") => {
                if uploads.len() >= 50 {
                    return Err(AppError::BadRequest(
                        "单个任务最多上传 50 个附件".to_owned(),
                    ));
                }
                let display_name = safe_upload_name(field.file_name().unwrap_or("attachment"));
                let mime_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_owned();
                let input_id = Uuid::new_v4().to_string();
                let input_dir = source_dir.join(&input_id);
                tokio::fs::create_dir_all(&input_dir).await?;
                let local_path = input_dir.join(&display_name);
                let mut output = tokio::fs::File::create(&local_path).await?;
                let mut hasher = Sha256::new();
                let mut upload_size = 0_u64;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|error| AppError::BadRequest(format!("上传中断：{error}")))?
                {
                    upload_size += chunk.len() as u64;
                    total_upload_size += chunk.len() as u64;
                    if total_upload_size > state.import_max_upload_bytes {
                        return Err(AppError::BadRequest(format!(
                            "附件总大小超过上传限制（{} MB）",
                            state.import_max_upload_bytes / 1024 / 1024
                        )));
                    }
                    hasher.update(&chunk);
                    output.write_all(&chunk).await?;
                }
                output.flush().await?;
                uploads.push(UploadedInput {
                    id: input_id,
                    display_name,
                    local_path,
                    mime_type,
                    size_bytes: upload_size,
                    sha256: format!("{:x}", hasher.finalize()),
                });
            }
            Some("links") => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| AppError::BadRequest(format!("无法读取链接：{error}")))?;
                links = serde_json::from_str(&value)
                    .map_err(|_| AppError::BadRequest("链接列表格式不正确".to_owned()))?;
                if links.len() > 20 {
                    return Err(AppError::BadRequest(
                        "单个任务最多附加 20 个链接".to_owned(),
                    ));
                }
            }
            Some("prompt") => {
                prompt = field
                    .text()
                    .await
                    .map_err(|error| AppError::BadRequest(format!("无法读取项目简介：{error}")))?;
                if prompt.chars().count() > 4_000 {
                    return Err(AppError::BadRequest(
                        "项目简介不能超过 4000 个字符".to_owned(),
                    ));
                }
            }
            _ => {}
        }
    }

    validate_links(&links)?;
    if uploads.is_empty() && links.is_empty() && prompt.trim().is_empty() {
        return Err(AppError::BadRequest(
            "请至少填写项目简介、项目链接或上传一个附件".to_owned(),
        ));
    }
    let source_name = uploads
        .first()
        .map(|upload| {
            if uploads.len() == 1 {
                upload.display_name.clone()
            } else {
                format!("{} 等 {} 个附件", upload.display_name, uploads.len())
            }
        })
        .or_else(|| links.first().map(|link| link.url.clone()))
        .unwrap_or_else(|| "项目简介".to_owned());
    let source_kind = if uploads.len() == 1 && is_zip_path(&uploads[0].local_path) {
        "zip"
    } else if uploads.is_empty() && !links.is_empty() {
        "link"
    } else if uploads.is_empty() {
        "prompt"
    } else {
        "mixed"
    };

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO import_jobs (
            id, status, stage, progress, source_kind, source_name, created_by_sid
         ) VALUES (?, 'queued', '等待解析', 5, ?, ?, ?)",
    )
    .bind(&job_id)
    .bind(source_kind)
    .bind(&source_name)
    .bind(&identity.sid)
    .execute(&mut *tx)
    .await?;
    insert_event_tx(
        &mut tx,
        &job_id,
        "queued",
        "queued",
        "等待解析",
        5,
        Some("材料已保存，等待后台整理"),
    )
    .await?;

    for (index, upload) in uploads.iter().enumerate() {
        let local_path = upload
            .local_path
            .strip_prefix(&job_dir)
            .map(path_for_json)
            .unwrap_or_else(|_| path_for_json(&upload.local_path));
        sqlx::query(
            "INSERT INTO import_inputs (
                id, job_id, input_kind, provider, display_name, local_path, mime_type,
                size_bytes, sha256, sort_order, status
             ) VALUES (?, ?, 'file', 'upload', ?, ?, ?, ?, ?, ?, 'queued')",
        )
        .bind(&upload.id)
        .bind(&job_id)
        .bind(&upload.display_name)
        .bind(local_path)
        .bind(&upload.mime_type)
        .bind(upload.size_bytes as i64)
        .bind(&upload.sha256)
        .bind(index as i64)
        .execute(&mut *tx)
        .await?;
    }

    if !prompt.trim().is_empty() {
        sqlx::query(
            "INSERT INTO import_inputs (
                id, job_id, input_kind, provider, display_name, source_ref, sort_order, status
             ) VALUES (?, ?, 'prompt', 'user', '项目简介', ?, ?, 'parsed')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&job_id)
        .bind(prompt.trim())
        .bind(uploads.len() as i64)
        .execute(&mut *tx)
        .await?;
    }

    for (index, link) in links.iter().enumerate() {
        sqlx::query(
            "INSERT INTO import_inputs (
                id, job_id, input_kind, provider, display_name, source_ref, sort_order, status
             ) VALUES (?, ?, 'link', ?, ?, ?, ?, 'pending_parser')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&job_id)
        .bind(link_provider(&link.url))
        .bind(
            link.title
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(&link.url),
        )
        .bind(&link.url)
        .bind((uploads.len() + usize::from(!prompt.trim().is_empty()) + index) as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    job_dir_guard.keep();

    Ok((StatusCode::ACCEPTED, Json(load_job(&state, &job_id).await?)))
}

pub async fn detail(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ImportJobResponse>, AppError> {
    if !identity.can_access_icthub() {
        return Err(AppError::Forbidden);
    }
    Ok(Json(load_job(&state, &id).await?))
}

pub async fn cancel(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ImportJobResponse>, AppError> {
    ensure_job_owner(&state, &id, &identity.sid, identity.is_superadmin()).await?;
    let status = sqlx::query_scalar::<_, String>("SELECT status FROM import_jobs WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    if matches!(status.as_str(), "completed" | "failed" | "cancelled") {
        return Err(AppError::Conflict("该整理任务已经结束".to_owned()));
    }
    sqlx::query(
        "UPDATE import_jobs SET status = 'cancelled', stage = '已取消',
            error_message = NULL, cancel_requested_at = CURRENT_TIMESTAMP,
            lease_expires_at = NULL, updated_at = CURRENT_TIMESTAMP,
            completed_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&id)
    .execute(&state.db)
    .await?;
    insert_event(
        &state,
        &id,
        "cancelled",
        "cancelled",
        "已取消",
        100,
        Some("成员取消了本次整理任务"),
    )
    .await?;
    Ok(Json(load_job(&state, &id).await?))
}

pub async fn refine(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<RefinementInput>,
) -> Result<Json<ImportJobResponse>, AppError> {
    ensure_job_owner(&state, &id, &identity.sid, identity.is_superadmin()).await?;
    let prompt = input.prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > 4_000 {
        return Err(AppError::BadRequest(
            "补充提示词不能为空且不能超过 4000 个字符".to_owned(),
        ));
    }
    let status = sqlx::query_scalar::<_, String>("SELECT status FROM import_jobs WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    if status != "completed" {
        return Err(AppError::Conflict(
            "请等待整理流程完成后再提交补充提示".to_owned(),
        ));
    }
    sqlx::query(
        "DELETE FROM import_inputs WHERE job_id = ? AND input_kind = 'prompt'
            AND display_name = '整理补充提示'",
    )
    .bind(&id)
    .execute(&state.db)
    .await?;
    let sort_order = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM import_inputs WHERE job_id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;
    sqlx::query(
        "INSERT INTO import_inputs (
            id, job_id, input_kind, provider, display_name, source_ref, sort_order, status
         ) VALUES (?, ?, 'prompt', 'user', '整理补充提示', ?, ?, 'queued_codex')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&id)
    .bind(prompt)
    .bind(sort_order)
    .execute(&state.db)
    .await?;
    sqlx::query(
        "UPDATE import_jobs SET stage = '补充提示已保存，等待 Codex', updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'completed'",
    )
    .bind(&id)
    .execute(&state.db)
    .await?;
    insert_event(
        &state,
        &id,
        "refinement_saved",
        "completed",
        "补充提示已保存，等待 Codex",
        100,
        Some("补充提示已加入任务上下文"),
    )
    .await?;
    Ok(Json(load_job(&state, &id).await?))
}

async fn ensure_job_owner(
    state: &AppState,
    id: &str,
    sid: &str,
    is_superadmin: bool,
) -> Result<(), AppError> {
    let owner =
        sqlx::query_scalar::<_, String>("SELECT created_by_sid FROM import_jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    if owner != sid && !is_superadmin {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn is_cancelled(state: &AppState, job_id: &str) -> anyhow::Result<bool> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT status FROM import_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&state.db)
            .await?
            .is_some_and(|status| status == "cancelled"),
    )
}

pub async fn run_import_worker(
    state: AppState,
    options: ImportWorkerOptions,
) -> anyhow::Result<()> {
    tracing::info!(worker_id = %options.worker_id, "import worker started");
    loop {
        match process_one_queued_job(&state, &options).await {
            Ok(true) => {}
            Ok(false) => tokio::time::sleep(options.poll_interval).await,
            Err(error) => {
                tracing::error!(
                    worker_id = %options.worker_id,
                    error = %error,
                    "import worker polling failed"
                );
                tokio::time::sleep(options.poll_interval.max(Duration::from_secs(1))).await;
            }
        }
    }
}

pub async fn process_one_queued_job(
    state: &AppState,
    options: &ImportWorkerOptions,
) -> anyhow::Result<bool> {
    let Some(job_id) = claim_next_job(state, options).await? else {
        return Ok(false);
    };

    insert_event(
        state,
        &job_id,
        "claimed",
        "normalizing",
        "准备材料",
        10,
        Some(&format!("任务已由 {} 领取", options.worker_id)),
    )
    .await?;

    let heartbeat_state = state.clone();
    let heartbeat_job_id = job_id.clone();
    let heartbeat_worker_id = options.worker_id.clone();
    let heartbeat_lease = options.lease_duration;
    let heartbeat_interval = Duration::from_secs((heartbeat_lease.as_secs() / 3).max(10));
    let heartbeat = tokio::spawn(async move {
        loop {
            tokio::time::sleep(heartbeat_interval).await;
            let lease_modifier = format!("+{} seconds", heartbeat_lease.as_secs());
            let updated = sqlx::query(
                "UPDATE import_jobs SET last_heartbeat_at = CURRENT_TIMESTAMP,
                    lease_expires_at = datetime('now', ?), updated_at = CURRENT_TIMESTAMP
                 WHERE id = ? AND worker_id = ?
                    AND status NOT IN ('completed', 'failed', 'cancelled')",
            )
            .bind(lease_modifier)
            .bind(&heartbeat_job_id)
            .bind(&heartbeat_worker_id)
            .execute(&heartbeat_state.db)
            .await;
            match updated {
                Ok(result) if result.rows_affected() == 1 => {}
                _ => break,
            }
        }
    });

    let result =
        process_claimed_job(state, &job_id, &options.worker_id, options.lease_duration).await;
    heartbeat.abort();

    if let Err(error) = result {
        tracing::error!(job_id = %job_id, error = %error, "import job failed");
        if !is_cancelled(state, &job_id).await? {
            let message = user_facing_worker_error(&error);
            sqlx::query(
                "UPDATE import_jobs SET status = 'failed', stage = '解析失败', progress = 100,
                    error_message = ?, worker_id = NULL, lease_expires_at = NULL,
                    updated_at = CURRENT_TIMESTAMP, completed_at = CURRENT_TIMESTAMP
                 WHERE id = ? AND worker_id = ? AND status != 'cancelled'",
            )
            .bind(&message)
            .bind(&job_id)
            .bind(&options.worker_id)
            .execute(&state.db)
            .await?;
            insert_event(
                state,
                &job_id,
                "failed",
                "failed",
                "解析失败",
                100,
                Some(&message),
            )
            .await?;
        }
    }

    sqlx::query(
        "UPDATE import_jobs SET worker_id = NULL, lease_expires_at = NULL
         WHERE id = ? AND worker_id = ? AND status = 'cancelled'",
    )
    .bind(&job_id)
    .bind(&options.worker_id)
    .execute(&state.db)
    .await?;
    Ok(true)
}

async fn claim_next_job(
    state: &AppState,
    options: &ImportWorkerOptions,
) -> anyhow::Result<Option<String>> {
    let lease_modifier = format!("+{} seconds", options.lease_duration.as_secs());
    let job_id = sqlx::query_scalar::<_, String>(
        "UPDATE import_jobs SET status = 'normalizing', stage = '准备材料', progress = 10,
            worker_id = ?, lease_expires_at = datetime('now', ?),
            last_heartbeat_at = CURRENT_TIMESTAMP, started_at = COALESCE(started_at, CURRENT_TIMESTAMP),
            attempt_count = attempt_count + 1, error_message = NULL,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = (
            SELECT id FROM import_jobs
             WHERE status = 'queued'
                OR (
                    status IN ('normalizing', 'extracting', 'indexing', 'analyzing')
                    AND lease_expires_at IS NOT NULL
                    AND lease_expires_at <= CURRENT_TIMESTAMP
                )
             ORDER BY created_at ASC
             LIMIT 1
         ) AND status != 'cancelled'
         RETURNING id",
    )
    .bind(&options.worker_id)
    .bind(lease_modifier)
    .fetch_optional(&state.db)
    .await?;
    Ok(job_id)
}

async fn process_claimed_job(
    state: &AppState,
    job_id: &str,
    worker_id: &str,
    lease_duration: Duration,
) -> anyhow::Result<()> {
    let source_name = sqlx::query_scalar::<_, String>(
        "SELECT source_name FROM import_jobs WHERE id = ? AND worker_id = ?",
    )
    .bind(job_id)
    .bind(worker_id)
    .fetch_one(&state.db)
    .await?;
    let stored_files = sqlx::query_as::<_, StoredFileInput>(
        "SELECT id, display_name, local_path, mime_type, size_bytes, sha256
         FROM import_inputs WHERE job_id = ? AND input_kind = 'file'
         ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(job_id)
    .fetch_all(&state.db)
    .await?;
    let job_dir = state.import_root.join(job_id);
    let uploads = stored_files
        .into_iter()
        .map(|input| UploadedInput {
            id: input.id,
            display_name: input.display_name,
            local_path: job_dir.join(input.local_path),
            mime_type: input.mime_type,
            size_bytes: input.size_bytes.max(0) as u64,
            sha256: input.sha256,
        })
        .collect::<Vec<_>>();
    let prompt_parts = sqlx::query_scalar::<_, String>(
        "SELECT source_ref FROM import_inputs
         WHERE job_id = ? AND input_kind = 'prompt' AND source_ref IS NOT NULL
         ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(job_id)
    .fetch_all(&state.db)
    .await?;
    let prompt = prompt_parts.join("\n\n");
    let links = sqlx::query_as::<_, (String, String)>(
        "SELECT source_ref, display_name FROM import_inputs
         WHERE job_id = ? AND input_kind = 'link' AND source_ref IS NOT NULL
         ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(job_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(url, title)| LinkInput {
        url,
        title: Some(title),
    })
    .collect::<Vec<_>>();
    let analysis_name = uploads
        .first()
        .map(|upload| upload.display_name.clone())
        .unwrap_or_else(|| {
            let hint = project_name_hint(&prompt);
            if hint == "待识别项目" {
                source_name
            } else {
                hint
            }
        });

    process_job(
        state,
        job_id,
        worker_id,
        lease_duration,
        &uploads,
        &analysis_name,
        &prompt,
        &links,
    )
    .await
}

async fn process_job(
    state: &AppState,
    job_id: &str,
    worker_id: &str,
    lease_duration: Duration,
    uploads: &[UploadedInput],
    source_name: &str,
    prompt: &str,
    links: &[LinkInput],
) -> anyhow::Result<()> {
    if is_cancelled(state, job_id).await? {
        return Ok(());
    }
    update_progress(
        state,
        job_id,
        worker_id,
        lease_duration,
        "extracting",
        "正在安全整理附件",
        18,
    )
    .await?;
    let job_dir = state.import_root.join(job_id);
    let max_unpacked = state.import_max_unpacked_bytes;
    let source_name = source_name.to_owned();
    let job_id_owned = job_id.to_owned();
    let uploads = uploads.to_vec();
    let primary_input_id = uploads.first().map(|upload| upload.id.clone());
    let context = import_context(prompt, links);
    let prompt = prompt.to_owned();
    let ffprobe_bin = state.ffprobe_bin.as_ref().clone();
    let build = tokio::task::spawn_blocking(move || {
        if uploads.is_empty() {
            Ok(analyze_context_only(
                &source_name,
                &job_id_owned,
                &context,
                &prompt,
            ))
        } else {
            prepare_normalized_archive(&job_dir, &uploads, max_unpacked)?;
            safe_extract_and_analyze(
                &job_dir,
                &source_name,
                &job_id_owned,
                max_unpacked,
                &context,
                &prompt,
                &ffprobe_bin,
            )
        }
    })
    .await
    .context("import worker stopped unexpectedly")??;

    if is_cancelled(state, job_id).await? {
        return Ok(());
    }

    update_progress(
        state,
        job_id,
        worker_id,
        lease_duration,
        "analyzing",
        "正在生成导入预览",
        76,
    )
    .await?;
    persist_analysis(state, job_id, primary_input_id.as_deref(), &build).await?;
    let result_json = serde_json::to_string(&build.analysis)?;
    let completed = sqlx::query(
        "UPDATE import_jobs SET status = 'completed', stage = '等待确认', progress = 100,
            result_json = ?, error_message = NULL, updated_at = CURRENT_TIMESTAMP,
            worker_id = NULL, lease_expires_at = NULL, completed_at = CURRENT_TIMESTAMP
         WHERE id = ? AND worker_id = ? AND status != 'cancelled'",
    )
    .bind(result_json)
    .bind(job_id)
    .bind(worker_id)
    .execute(&state.db)
    .await?;
    if completed.rows_affected() != 1 {
        bail!("import job lease was lost before completion");
    }
    if is_cancelled(state, job_id).await? {
        return Ok(());
    }
    sqlx::query(
        "UPDATE import_inputs SET status = 'parsed' WHERE job_id = ? AND input_kind = 'file'",
    )
    .bind(job_id)
    .execute(&state.db)
    .await?;
    insert_event(
        state,
        job_id,
        "completed",
        "completed",
        "等待确认",
        100,
        Some("材料整理完成，项目草稿已生成"),
    )
    .await?;
    Ok(())
}

async fn update_progress(
    state: &AppState,
    job_id: &str,
    worker_id: &str,
    lease_duration: Duration,
    status: &str,
    stage: &str,
    progress: i64,
) -> anyhow::Result<()> {
    let lease_modifier = format!("+{} seconds", lease_duration.as_secs());
    let updated = sqlx::query(
        "UPDATE import_jobs SET status = ?, stage = ?, progress = ?, updated_at = CURRENT_TIMESTAMP
            , last_heartbeat_at = CURRENT_TIMESTAMP, lease_expires_at = datetime('now', ?)
         WHERE id = ? AND worker_id = ? AND status != 'cancelled'",
    )
    .bind(status)
    .bind(stage)
    .bind(progress)
    .bind(lease_modifier)
    .bind(job_id)
    .bind(worker_id)
    .execute(&state.db)
    .await?;
    if updated.rows_affected() != 1 {
        bail!("import job lease was lost or the job was cancelled");
    }
    insert_event(state, job_id, "progress", status, stage, progress, None).await?;
    Ok(())
}

async fn insert_event(
    state: &AppState,
    job_id: &str,
    event_type: &str,
    status: &str,
    stage: &str,
    progress: i64,
    message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO import_job_events (
            job_id, event_type, status, stage, progress, message
         ) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(job_id)
    .bind(event_type)
    .bind(status)
    .bind(stage)
    .bind(progress)
    .bind(message)
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn insert_event_tx(
    tx: &mut Transaction<'_, Sqlite>,
    job_id: &str,
    event_type: &str,
    status: &str,
    stage: &str,
    progress: i64,
    message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO import_job_events (
            job_id, event_type, status, stage, progress, message
         ) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(job_id)
    .bind(event_type)
    .bind(status)
    .bind(stage)
    .bind(progress)
    .bind(message)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn persist_analysis(
    state: &AppState,
    job_id: &str,
    input_id: Option<&str>,
    build: &AnalysisBuild,
) -> anyhow::Result<()> {
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM import_artifacts WHERE job_id = ?")
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
    for artifact in &build.artifacts {
        insert_artifact(&mut tx, job_id, input_id, artifact).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn insert_artifact(
    tx: &mut Transaction<'_, Sqlite>,
    job_id: &str,
    input_id: Option<&str>,
    artifact: &ArtifactRecord,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO import_artifacts (
            id, job_id, input_id, relative_path, artifact_kind, mime_type, size_bytes,
            extractor, metadata_json, is_cover_candidate
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(job_id)
    .bind(input_id)
    .bind(&artifact.relative_path)
    .bind(&artifact.artifact_kind)
    .bind(&artifact.mime_type)
    .bind(artifact.size_bytes as i64)
    .bind(&artifact.extractor)
    .bind(&artifact.metadata_json)
    .bind(artifact.is_cover_candidate)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn safe_upload_name(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let basename = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment");
    let cleaned = basename
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect::<String>();
    if cleaned.trim_matches(['.', ' ']).is_empty() {
        "attachment".to_owned()
    } else {
        cleaned
    }
}

fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("zip"))
}

fn import_context(prompt: &str, links: &[LinkInput]) -> String {
    let mut context = String::new();
    if !prompt.trim().is_empty() {
        context.push_str("用户提供的项目简介：\n");
        context.push_str(prompt.trim());
    }
    if !links.is_empty() {
        if !context.is_empty() {
            context.push_str("\n\n");
        }
        context.push_str("用户提供的项目链接：\n");
        for link in links {
            context.push_str("- ");
            context.push_str(&link.url);
            context.push('\n');
        }
    }
    context
}

fn project_name_hint(prompt: &str) -> String {
    for line in prompt
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some((label, value)) = line.split_once(['：', ':']) {
            if matches!(label.trim(), "项目名" | "项目名称" | "名称") && !value.trim().is_empty()
            {
                return value
                    .trim()
                    .split(['。', '；', ';'])
                    .next()
                    .unwrap_or(value.trim())
                    .trim()
                    .chars()
                    .take(80)
                    .collect();
            }
        }
    }
    "待识别项目".to_owned()
}

fn analyze_context_only(
    source_name: &str,
    job_id: &str,
    context: &str,
    prompt: &str,
) -> AnalysisBuild {
    let artifacts = Vec::new();
    let analysis = build_fallback_analysis(
        source_name,
        job_id,
        &artifacts,
        context,
        &BTreeSet::new(),
        prompt,
    );
    AnalysisBuild {
        artifacts,
        analysis,
    }
}

fn prepare_normalized_archive(
    job_dir: &Path,
    uploads: &[UploadedInput],
    max_unpacked_bytes: u64,
) -> anyhow::Result<()> {
    let archive_path = job_dir.join("source").join("input.zip");
    if uploads.len() == 1 && is_zip_path(&uploads[0].local_path) {
        std::fs::copy(&uploads[0].local_path, &archive_path)?;
        return Ok(());
    }

    let destination = File::create(&archive_path)?;
    let mut writer = ZipWriter::new(destination);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let mut total_written = 0_u64;
    let mut entry_count = 0_usize;

    for (index, upload) in uploads.iter().enumerate() {
        if is_zip_path(&upload.local_path) {
            let source = File::open(&upload.local_path)?;
            let mut archive =
                ZipArchive::new(source).context("uploaded file is not a valid ZIP")?;
            if entry_count.saturating_add(archive.len()) > MAX_ARCHIVE_ENTRIES {
                bail!("archive contains too many entries");
            }
            let stem = Path::new(&upload.display_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .map(safe_upload_name)
                .unwrap_or_else(|| format!("archive-{}", index + 1));
            let prefix = format!("{:02}-{}", index + 1, stem);
            for entry_index in 0..archive.len() {
                let mut entry = archive.by_index(entry_index)?;
                let enclosed = entry
                    .enclosed_name()
                    .context("archive contains an unsafe path")?
                    .to_owned();
                if !is_safe_relative_path(&enclosed) {
                    bail!("archive contains an unsafe path");
                }
                if entry
                    .unix_mode()
                    .is_some_and(|mode| mode & 0o170000 == 0o120000)
                {
                    bail!("archive contains a symbolic link");
                }
                if entry.is_dir() {
                    continue;
                }
                entry_count += 1;
                let normalized_name = format!("{prefix}/{}", path_for_json(&enclosed));
                writer.start_file(normalized_name, options)?;
                copy_limited(
                    &mut entry,
                    &mut writer,
                    &mut total_written,
                    max_unpacked_bytes,
                )?;
            }
        } else {
            entry_count += 1;
            if entry_count > MAX_ARCHIVE_ENTRIES {
                bail!("archive contains too many entries");
            }
            let normalized_name = format!("{:02}-{}", index + 1, upload.display_name);
            writer.start_file(normalized_name, options)?;
            let mut source = File::open(&upload.local_path)?;
            copy_limited(
                &mut source,
                &mut writer,
                &mut total_written,
                max_unpacked_bytes,
            )?;
        }
    }
    writer.finish()?;
    Ok(())
}

fn copy_limited(
    source: &mut impl Read,
    destination: &mut impl Write,
    total_written: &mut u64,
    max_unpacked_bytes: u64,
) -> anyhow::Result<()> {
    let allowed = MAX_SINGLE_FILE_BYTES.min(max_unpacked_bytes.saturating_sub(*total_written));
    if allowed == 0 {
        bail!("archive exceeds the unpacked size limit");
    }
    let copied = io::copy(&mut source.take(allowed + 1), destination)?;
    if copied > allowed || copied > MAX_SINGLE_FILE_BYTES {
        bail!("archive contains an oversized file");
    }
    *total_written += copied;
    if *total_written > max_unpacked_bytes {
        bail!("archive exceeds the unpacked size limit");
    }
    Ok(())
}

fn safe_extract_and_analyze(
    job_dir: &Path,
    source_name: &str,
    job_id: &str,
    max_unpacked_bytes: u64,
    context: &str,
    prompt: &str,
    ffprobe_bin: &str,
) -> anyhow::Result<AnalysisBuild> {
    let archive_path = job_dir.join("source").join("input.zip");
    let extracted_root = job_dir.join("extracted");
    std::fs::create_dir_all(&extracted_root)?;
    let source = File::open(&archive_path).context("cannot open uploaded ZIP")?;
    let mut archive = ZipArchive::new(source).context("uploaded file is not a valid ZIP")?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        bail!("archive contains too many entries");
    }

    let mut artifacts = Vec::new();
    let mut total_written = 0_u64;
    let mut skipped_directories = BTreeSet::new();
    let mut text_corpus = context
        .chars()
        .take(MAX_TEXT_CORPUS_BYTES)
        .collect::<String>();
    let mut top_levels = BTreeSet::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let enclosed = entry
            .enclosed_name()
            .context("archive contains an unsafe path")?
            .to_owned();
        if !is_safe_relative_path(&enclosed) {
            bail!("archive contains an unsafe path");
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("archive contains a symbolic link");
        }
        if entry.is_dir() {
            continue;
        }
        if let Some(directory) = ignored_generated_directory(&enclosed) {
            skipped_directories.insert(directory.to_owned());
            continue;
        }
        if let Some(Component::Normal(component)) = enclosed.components().next() {
            top_levels.insert(component.to_string_lossy().to_string());
        }

        let output_path = extracted_root.join(&enclosed);
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if output_path.exists() {
            bail!("archive contains duplicate file paths");
        }
        let mut output = File::create(&output_path)?;
        let allowed = MAX_SINGLE_FILE_BYTES.min(max_unpacked_bytes.saturating_sub(total_written));
        if allowed == 0 {
            bail!("archive exceeds the unpacked size limit");
        }
        let copied = io::copy(&mut (&mut entry).take(allowed + 1), &mut output)?;
        if copied > allowed || copied > MAX_SINGLE_FILE_BYTES {
            bail!("archive contains an oversized file");
        }
        total_written += copied;
        if total_written > max_unpacked_bytes {
            bail!("archive exceeds the unpacked size limit");
        }

        let relative_path = path_for_json(&enclosed);
        let kind = artifact_kind(&enclosed).to_owned();
        let mime_type = mime_guess::from_path(&enclosed)
            .first_raw()
            .map(str::to_owned);
        let is_cover_candidate = kind == "image" && cover_candidate_rank(&enclosed) > 0;
        let extraction = extract_artifact(&output_path, &enclosed, &kind, copied, ffprobe_bin);
        if let Some(text) = extraction.text.as_deref() {
            append_extracted_preview(&relative_path, text, &mut text_corpus);
        }
        artifacts.push(ArtifactRecord {
            relative_path,
            artifact_kind: kind,
            mime_type,
            size_bytes: copied,
            extractor: extraction.extractor,
            metadata_json: serde_json::to_string(&extraction.metadata)?,
            is_cover_candidate,
        });
    }

    if artifacts.is_empty() {
        bail!("archive does not contain files that can be indexed");
    }
    artifacts.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let project_name = infer_project_name(source_name, &top_levels);
    let analysis = build_fallback_analysis(
        &project_name,
        job_id,
        &artifacts,
        &text_corpus,
        &skipped_directories,
        prompt,
    );
    Ok(AnalysisBuild {
        artifacts,
        analysis,
    })
}

fn append_extracted_preview(label: &str, text: &str, corpus: &mut String) {
    if corpus.len() >= MAX_TEXT_CORPUS_BYTES {
        return;
    }
    let remaining = (MAX_TEXT_CORPUS_BYTES - corpus.len()).min(32 * 1024);
    corpus.push_str("\n--- ");
    corpus.push_str(label);
    corpus.push_str(" ---\n");
    corpus.extend(text.chars().take(remaining));
}

fn build_fallback_analysis(
    project_name: &str,
    job_id: &str,
    artifacts: &[ArtifactRecord],
    text_corpus: &str,
    skipped_directories: &BTreeSet<String>,
    prompt: &str,
) -> ImportAnalysis {
    let mut totals = BTreeMap::<String, (usize, u64)>::new();
    for artifact in artifacts {
        let entry = totals
            .entry(artifact.artifact_kind.clone())
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += artifact.size_bytes;
    }
    let artifact_summary = totals
        .iter()
        .map(|(kind, (count, total_bytes))| ArtifactSummary {
            kind: kind.clone(),
            count: *count,
            total_bytes: *total_bytes,
        })
        .collect::<Vec<_>>();
    let searchable = format!(
        "{}\n{}",
        artifacts
            .iter()
            .map(|artifact| artifact.relative_path.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        text_corpus
    )
    .to_lowercase();
    let primary_category = infer_category(&searchable, &totals).to_owned();
    let suggested_tags = explicit_tags(prompt);
    let owner_name = explicit_field(prompt, &["当前负责", "负责人", "维护者"]);
    let source_name = explicit_field(prompt, &["来源者", "来源方", "来源"]);
    let highest_award = explicit_field(prompt, &["最高奖项", "获奖", "奖项"]);
    let summary = summary_sentence(&artifact_summary);
    let mut warnings = vec![
        "当前使用确定性回退生成草稿；配置 Codex 后将补充语义摘要、奖项识别和更准确的分类。"
            .to_owned(),
        "PPT、文档和视频已完成文件级归类，内容抽取器将在 Agent 链路配置阶段接入。".to_owned(),
    ];
    if !skipped_directories.is_empty() {
        warnings.push(format!(
            "为控制体积与风险，已跳过生成目录：{}。",
            skipped_directories
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("、")
        ));
    }

    ImportAnalysis {
        project_draft: ImportProjectDraft {
            name: project_name.to_owned(),
            slug: slugify(project_name, job_id),
            summary,
            primary_category,
            suggested_tags,
            owner_name,
            source_name,
            highest_award,
            status: "待确认".to_owned(),
        },
        artifact_summary,
        warnings,
        agent: AgentState {
            status: "awaiting_configuration".to_owned(),
            mode: "deterministic_fallback".to_owned(),
            message: "材料收集与安全归类链路已打通，等待配置 Codex Base URL 与 API Token。"
                .to_owned(),
        },
        capabilities: ImportCapabilities {
            zip_upload: "prototype_ready".to_owned(),
            github_link: "input_reserved".to_owned(),
            mixed_files: "prototype_ready".to_owned(),
            codex_agent: "awaiting_configuration".to_owned(),
            github_publish: "awaiting_credentials".to_owned(),
        },
    }
}

fn infer_category(searchable: &str, totals: &BTreeMap<String, (usize, u64)>) -> &'static str {
    if contains_any(
        searchable,
        &[
            "arduino",
            "esp32",
            "stm32",
            "kicad",
            "物联网",
            "嵌入式",
            "机器人",
        ],
    ) {
        "智能硬件"
    } else if contains_any(
        searchable,
        &[
            "unity",
            "unreal",
            "blender",
            ".blend",
            "vr",
            "动画",
            "数字展陈",
        ],
    ) {
        "数字媒体"
    } else if contains_any(
        searchable,
        &[
            "pytorch",
            "tensorflow",
            "transformers",
            "opencv",
            "yolo",
            "langchain",
            "machine learning",
            "人工智能",
            "大数据",
            "模型训练",
        ],
    ) {
        "AI 软件"
    } else if contains_any(
        searchable,
        &["thesis", "paper", "论文", "实验研究", "latex"],
    ) && totals.get("code").map_or(0, |value| value.0) < 5
    {
        "研究成果"
    } else {
        "传统软件"
    }
}

fn explicit_tags(prompt: &str) -> Vec<String> {
    const FORMAL_TAGS: &[&str] = &[
        "国创赛（互联网+）",
        "计算机设计大赛",
        "智能应用技术大赛",
        "大数据",
        "人工智能应用",
        "LLM/Agent",
        "计算机视觉",
        "NLP",
        "物联网",
        "嵌入式",
        "机器人",
        "Web",
        "移动端",
        "3D/VR",
        "软硬结合",
        "AI核心",
        "AI增强",
        "非AI",
        "开源项目",
        "校园服务",
        "教育",
        "农业",
        "医疗",
        "文旅",
        "工业",
        "科研辅助",
        "比赛项目",
        "实验室建设",
        "课程项目",
        "日常工具",
        "个人探索",
        "对外服务",
    ];
    let Some(value) = explicit_field(prompt, &["标签", "项目标签"]) else {
        return Vec::new();
    };
    value
        .split([',', '，', '、', '/', '|'])
        .map(str::trim)
        .filter(|tag| {
            FORMAL_TAGS
                .iter()
                .any(|formal| formal.eq_ignore_ascii_case(tag))
        })
        .map(ToOwned::to_owned)
        .take(3)
        .collect()
}

fn explicit_field(prompt: &str, labels: &[&str]) -> Option<String> {
    for line in prompt
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some((label, value)) = line.split_once(['：', ':']) else {
            continue;
        };
        if labels.iter().any(|candidate| label.trim() == *candidate) {
            let value = value
                .trim()
                .split(['。', '；', ';'])
                .next()
                .unwrap_or(value.trim())
                .trim();
            if !value.is_empty() {
                return Some(value.chars().take(120).collect());
            }
        }
    }
    None
}

fn summary_sentence(summary: &[ArtifactSummary]) -> String {
    if summary.is_empty() {
        return "已收集项目简介与链接，等待进一步理解与确认。".to_owned();
    }
    let parts = summary
        .iter()
        .filter(|item| item.count > 0)
        .map(|item| format!("{} 个{}", item.count, kind_label(&item.kind)))
        .collect::<Vec<_>>();
    format!(
        "项目材料中识别到{}，等待进一步理解与确认。",
        parts.join("、")
    )
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "code" => "源码文件",
        "document" => "文档",
        "presentation" => "演示文件",
        "video" => "视频",
        "image" => "图片",
        "archive" => "压缩文件",
        "data" => "数据文件",
        _ => "其他文件",
    }
}

fn artifact_kind(path: &Path) -> &'static str {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "ppt" | "pptx" | "key" | "odp") {
        "presentation"
    } else if matches!(
        extension.as_str(),
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "md" | "txt" | "tex"
    ) {
        "document"
    } else if matches!(
        extension.as_str(),
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "wmv"
    ) {
        "video"
    } else if matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "tiff"
    ) {
        "image"
    } else if matches!(
        extension.as_str(),
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz"
    ) {
        "archive"
    } else if matches!(
        extension.as_str(),
        "csv" | "tsv" | "jsonl" | "parquet" | "sqlite" | "db" | "xlsx" | "xls"
    ) {
        "data"
    } else if is_source_extension(&extension) || is_source_manifest(&name) {
        "code"
    } else {
        "other"
    }
}

fn is_source_extension(extension: &str) -> bool {
    matches!(
        extension,
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "py"
            | "java"
            | "kt"
            | "kts"
            | "go"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "cs"
            | "php"
            | "rb"
            | "swift"
            | "dart"
            | "vue"
            | "svelte"
            | "html"
            | "css"
            | "scss"
            | "sql"
            | "sh"
            | "ps1"
            | "bat"
            | "toml"
            | "yaml"
            | "yml"
            | "xml"
            | "gradle"
            | "ino"
    )
}

fn is_source_manifest(name: &str) -> bool {
    matches!(
        name,
        "dockerfile"
            | "makefile"
            | "cargo.lock"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "requirements.txt"
            | "pyproject.toml"
            | "pom.xml"
    )
}

fn cover_candidate_rank(path: &Path) -> u8 {
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if contains_any(&name, &["cover", "poster", "封面", "海报"]) {
        3
    } else if contains_any(&name, &["screenshot", "screen", "截图", "demo", "展示"]) {
        2
    } else {
        1
    }
}

fn infer_project_name(source_name: &str, top_levels: &BTreeSet<String>) -> String {
    if top_levels.len() == 1 {
        let only = top_levels.iter().next().expect("one top-level entry");
        if !only.contains('.') {
            return clean_project_name(only);
        }
    }
    clean_project_name(
        Path::new(source_name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("待识别项目"),
    )
}

fn clean_project_name(value: &str) -> String {
    let cleaned = value
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.is_empty() {
        "待识别项目".to_owned()
    } else {
        cleaned
    }
}

fn slugify(value: &str, job_id: &str) -> String {
    let mut slug = String::new();
    let mut hyphen = false;
    for character in value.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            hyphen = false;
        } else if !slug.is_empty() && !hyphen {
            slug.push('-');
            hyphen = true;
        }
    }
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        format!("import-{}", &job_id[..8.min(job_id.len())])
    } else {
        slug.chars().take(72).collect()
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn ignored_generated_directory(path: &Path) -> Option<&str> {
    path.components().find_map(|component| match component {
        Component::Normal(value) => {
            let value = value.to_str()?;
            matches!(
                value.to_ascii_lowercase().as_str(),
                ".git" | "node_modules" | "target" | ".venv" | "venv" | "__pycache__"
            )
            .then_some(value)
        }
        _ => None,
    })
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn path_for_json(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn validate_links(links: &[LinkInput]) -> Result<(), AppError> {
    for link in links {
        if link.url.len() > 2_048 {
            return Err(AppError::BadRequest("链接长度超过限制".to_owned()));
        }
        let url = reqwest::Url::parse(link.url.trim())
            .map_err(|_| AppError::BadRequest(format!("链接格式不正确：{}", link.url)))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(AppError::BadRequest("链接只允许 HTTP 或 HTTPS".to_owned()));
        }
    }
    Ok(())
}

fn link_provider(url: &str) -> &'static str {
    let host = reqwest::Url::parse(url)
        .ok()
        .and_then(|value| value.host_str().map(str::to_owned))
        .unwrap_or_default()
        .to_ascii_lowercase();
    if host == "github.com" || host.ends_with(".github.com") {
        "github"
    } else if host.contains("baidu") {
        "baidu"
    } else {
        "web"
    }
}

async fn load_job(state: &AppState, id: &str) -> Result<ImportJobResponse, AppError> {
    let row = sqlx::query_as::<_, ImportJobRow>(
        "SELECT id, status, stage, progress, source_kind, source_name, analysis_engine,
                result_json, error_message, created_at, updated_at, attempt_count,
                started_at, completed_at
         FROM import_jobs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let inputs = sqlx::query_as::<_, ImportInputView>(
        "SELECT id, input_kind, provider, display_name, source_ref, mime_type, size_bytes, status
         FROM import_inputs WHERE job_id = ? ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let events = sqlx::query_as::<_, ImportJobEventView>(
        "SELECT id, event_type, status, stage, progress, message, created_at
         FROM import_job_events WHERE job_id = ? ORDER BY id ASC LIMIT 200",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let artifact_rows = sqlx::query_as::<_, ImportArtifactRow>(
        "SELECT id, relative_path, artifact_kind, mime_type, size_bytes, extractor,
                metadata_json, is_cover_candidate
         FROM import_artifacts WHERE job_id = ?
         ORDER BY is_cover_candidate DESC, artifact_kind ASC, relative_path ASC LIMIT ?",
    )
    .bind(id)
    .bind(MAX_VISIBLE_ARTIFACTS)
    .fetch_all(&state.db)
    .await?;
    let artifacts = artifact_rows
        .into_iter()
        .map(|row| ImportArtifactView {
            id: row.id,
            relative_path: row.relative_path,
            artifact_kind: row.artifact_kind,
            mime_type: row.mime_type,
            size_bytes: row.size_bytes,
            extractor: row.extractor,
            metadata: serde_json::from_str(&row.metadata_json)
                .unwrap_or_else(|_| serde_json::json!({})),
            is_cover_candidate: row.is_cover_candidate,
        })
        .collect::<Vec<_>>();
    let result = row
        .result_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|_| AppError::BadRequest("导入结果损坏".to_owned()))?;
    Ok(ImportJobResponse {
        id: row.id,
        status: row.status,
        stage: row.stage,
        progress: row.progress,
        source_kind: row.source_kind,
        source_name: row.source_name,
        analysis_engine: row.analysis_engine,
        error_message: row.error_message,
        created_at: row.created_at,
        updated_at: row.updated_at,
        attempt_count: row.attempt_count,
        started_at: row.started_at,
        completed_at: row.completed_at,
        inputs,
        artifacts,
        events,
        result,
    })
}

fn user_facing_worker_error(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("valid ZIP") {
        "文件不是有效的 ZIP 压缩包".to_owned()
    } else if message.contains("unsafe path") || message.contains("symbolic link") {
        "压缩包包含不安全路径或符号链接，已停止解析".to_owned()
    } else if message.contains("size limit") || message.contains("oversized") {
        "压缩包解压后的体积超过限制".to_owned()
    } else if message.contains("too many entries") {
        "压缩包内文件数量超过限制".to_owned()
    } else {
        "解析失败，请检查压缩包后重试".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write, path::Path};

    use tempfile::tempdir;
    use zip::{write::SimpleFileOptions, ZipWriter};

    use super::{
        analyze_context_only, artifact_kind, prepare_normalized_archive, safe_extract_and_analyze,
        UploadedInput,
    };

    #[test]
    fn categorizes_common_project_artifacts() {
        assert_eq!(artifact_kind(Path::new("src/main.rs")), "code");
        assert_eq!(artifact_kind(Path::new("docs/report.pdf")), "document");
        assert_eq!(artifact_kind(Path::new("答辩.pptx")), "presentation");
        assert_eq!(artifact_kind(Path::new("demo.mp4")), "video");
        assert_eq!(artifact_kind(Path::new("poster.png")), "image");
    }

    #[test]
    fn extracts_zip_and_builds_ai_software_fallback() {
        let root = tempdir().expect("temp root");
        let job_dir = root.path().join("job");
        std::fs::create_dir_all(job_dir.join("source")).expect("source dir");
        let file = File::create(job_dir.join("source/input.zip")).expect("zip file");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        writer
            .start_file("cotton-ai/README.md", options)
            .expect("readme");
        writer
            .write_all("基于 YOLO 和 OpenCV 的棉花病虫害识别平台".as_bytes())
            .expect("readme contents");
        writer
            .start_file("cotton-ai/src/main.py", options)
            .expect("source");
        writer
            .write_all(b"print('hello')")
            .expect("source contents");
        writer
            .start_file("cotton-ai/poster.png", options)
            .expect("poster");
        writer.write_all(b"fake image").expect("poster contents");
        writer.finish().expect("finish zip");

        let build = safe_extract_and_analyze(
            &job_dir,
            "cotton-ai.zip",
            "12345678-0000-0000-0000-000000000000",
            16 * 1024 * 1024,
            "",
            "",
            "ffprobe-not-installed-for-tests",
        )
        .expect("analyze");
        assert_eq!(build.analysis.project_draft.name, "cotton ai");
        assert_eq!(build.analysis.project_draft.primary_category, "AI 软件");
        assert!(build.analysis.project_draft.suggested_tags.is_empty());
        assert!(build
            .artifacts
            .iter()
            .any(|artifact| artifact.is_cover_candidate));
    }

    #[test]
    fn mixed_direct_files_are_normalized_and_classified() {
        let root = tempdir().expect("temp root");
        let job_dir = root.path().join("job");
        let source_dir = job_dir.join("source");
        std::fs::create_dir_all(&source_dir).expect("source dir");
        let readme = source_dir.join("README.md");
        let slides = source_dir.join("答辩.pptx");
        std::fs::write(&readme, "React Web 校园工具").expect("readme");
        std::fs::write(&slides, b"fake pptx").expect("slides");
        let uploads = vec![
            UploadedInput {
                id: "readme".to_owned(),
                display_name: "README.md".to_owned(),
                local_path: readme,
                mime_type: "text/markdown".to_owned(),
                size_bytes: 20,
                sha256: "readme".to_owned(),
            },
            UploadedInput {
                id: "slides".to_owned(),
                display_name: "答辩.pptx".to_owned(),
                local_path: slides,
                mime_type:
                    "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                        .to_owned(),
                size_bytes: 9,
                sha256: "slides".to_owned(),
            },
        ];
        prepare_normalized_archive(&job_dir, &uploads, 16 * 1024 * 1024).expect("normalize files");
        let build = safe_extract_and_analyze(
            &job_dir,
            "README.md",
            "12345678-0000-0000-0000-000000000000",
            16 * 1024 * 1024,
            "这是一个校园 Web 工具",
            "标签：Web\n负责人：张三\n来源：课程项目",
            "ffprobe-not-installed-for-tests",
        )
        .expect("analyze files");
        assert!(build
            .artifacts
            .iter()
            .any(|artifact| artifact.artifact_kind == "document"));
        assert!(build
            .artifacts
            .iter()
            .any(|artifact| artifact.artifact_kind == "presentation"));
        assert!(build
            .analysis
            .project_draft
            .suggested_tags
            .contains(&"Web".to_owned()));
        assert_eq!(
            build.analysis.project_draft.owner_name.as_deref(),
            Some("张三")
        );
        assert_eq!(
            build.analysis.project_draft.source_name.as_deref(),
            Some("课程项目")
        );
    }

    #[test]
    fn prompt_and_links_can_create_a_context_only_draft() {
        let build = analyze_context_only(
            "待识别项目",
            "12345678-0000-0000-0000-000000000000",
            "项目名：校园预约助手\nhttps://github.com/example/booking",
            "项目名：校园预约助手\nhttps://github.com/example/booking",
        );
        assert!(build.artifacts.is_empty());
        assert_eq!(
            build.analysis.project_draft.summary,
            "已收集项目简介与链接，等待进一步理解与确认。"
        );
        assert_eq!(build.analysis.project_draft.primary_category, "传统软件");
        assert!(build.analysis.project_draft.suggested_tags.is_empty());
        assert!(build.analysis.project_draft.owner_name.is_none());
        assert!(build.analysis.project_draft.source_name.is_none());
    }
}
