use std::{
    io,
    path::{Component, Path as FsPath},
};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{
        header::{
            CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_SECURITY_POLICY,
            CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS,
        },
        HeaderValue, Response,
    },
    Json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, FeiyueIdentity},
    error::AppError,
    projects::ProjectResourceInput,
    state::AppState,
};

const PREVIEW_TOKEN_MINUTES: i64 = 10;

#[derive(Clone, Debug)]
pub(crate) struct PreparedResource {
    pub id: String,
    pub resource_type: String,
    pub title: String,
    pub url: Option<String>,
    pub object_key: Option<String>,
    pub source_name: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub display_path: Option<String>,
    pub preview_kind: Option<String>,
    pub entry_path: Option<String>,
    pub source_import_job_id: Option<String>,
    pub source_artifact_id: Option<String>,
    pub sha256: Option<String>,
    pub is_existing: bool,
}

#[derive(Debug, FromRow)]
struct StoredResourceRow {
    id: String,
    object_key: Option<String>,
    source_name: Option<String>,
    mime_type: Option<String>,
    size_bytes: Option<i64>,
    display_path: Option<String>,
    preview_kind: Option<String>,
    entry_path: Option<String>,
    source_import_job_id: Option<String>,
    source_artifact_id: Option<String>,
    sha256: Option<String>,
}

#[derive(Debug, FromRow)]
struct ImportArtifactRow {
    relative_path: String,
    artifact_kind: String,
    mime_type: Option<String>,
    size_bytes: i64,
    created_by_sid: String,
    job_status: String,
}

#[derive(Debug, FromRow)]
struct ResourceFileRow {
    object_key: String,
    entry_path: Option<String>,
    mime_type: Option<String>,
    preview_kind: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTicket {
    url: String,
    expires_at: String,
}

pub(crate) async fn prepare_project_resources(
    state: &AppState,
    project_id: &str,
    identity: &FeiyueIdentity,
    inputs: &[ProjectResourceInput],
) -> Result<Vec<PreparedResource>, AppError> {
    let mut prepared = Vec::with_capacity(inputs.len());
    for input in inputs {
        if let Some(id) = input.id.as_deref() {
            let stored = sqlx::query_as::<_, StoredResourceRow>(
                "SELECT id, object_key, source_name, mime_type, size_bytes, display_path,
                        preview_kind, entry_path, source_import_job_id, source_artifact_id, sha256
                 FROM resources WHERE id = ? AND project_id = ?",
            )
            .bind(id)
            .bind(project_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::BadRequest("项目资源不存在或不属于当前项目".to_owned()))?;
            prepared.push(PreparedResource {
                id: stored.id,
                resource_type: input.resource_type.clone(),
                title: input.title.clone(),
                url: input.url.clone(),
                object_key: stored.object_key,
                source_name: stored.source_name,
                mime_type: stored.mime_type,
                size_bytes: stored.size_bytes,
                display_path: stored.display_path,
                preview_kind: stored.preview_kind,
                entry_path: stored.entry_path,
                source_import_job_id: stored.source_import_job_id,
                source_artifact_id: stored.source_artifact_id,
                sha256: stored.sha256,
                is_existing: true,
            });
            continue;
        }

        let resource_id = Uuid::new_v4().to_string();
        let source_pair = input
            .source_import_job_id
            .as_deref()
            .zip(input.source_artifact_id.as_deref());
        if let Some((job_id, artifact_id)) = source_pair {
            prepared.push(
                materialize_import_artifact(
                    state,
                    project_id,
                    &resource_id,
                    identity,
                    input,
                    job_id,
                    artifact_id,
                )
                .await?,
            );
        } else {
            prepared.push(PreparedResource {
                id: resource_id,
                resource_type: input.resource_type.clone(),
                title: input.title.clone(),
                url: input.url.clone(),
                object_key: None,
                source_name: None,
                mime_type: None,
                size_bytes: None,
                display_path: None,
                preview_kind: None,
                entry_path: None,
                source_import_job_id: None,
                source_artifact_id: None,
                sha256: None,
                is_existing: false,
            });
        }
    }
    Ok(prepared)
}

async fn materialize_import_artifact(
    state: &AppState,
    project_id: &str,
    resource_id: &str,
    identity: &FeiyueIdentity,
    input: &ProjectResourceInput,
    job_id: &str,
    artifact_id: &str,
) -> Result<PreparedResource, AppError> {
    let artifact = sqlx::query_as::<_, ImportArtifactRow>(
        "SELECT a.relative_path, a.artifact_kind, a.mime_type, a.size_bytes,
                j.created_by_sid, j.status job_status
         FROM import_artifacts a
         JOIN import_jobs j ON j.id = a.job_id
         WHERE a.id = ? AND a.job_id = ?",
    )
    .bind(artifact_id)
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::BadRequest("导入附件不存在".to_owned()))?;
    if artifact.created_by_sid != identity.sid && !identity.is_superadmin() {
        return Err(AppError::Forbidden);
    }
    if !matches!(
        artifact.job_status.as_str(),
        "completed" | "agent_queued" | "agent_running"
    ) {
        return Err(AppError::Conflict("导入任务尚未整理完成".to_owned()));
    }

    let relative_path = FsPath::new(&artifact.relative_path);
    if !is_safe_relative_path(relative_path) {
        return Err(AppError::BadRequest("导入附件路径不安全".to_owned()));
    }
    let job_root = state.import_root.join(job_id);
    let source_root = if artifact.relative_path.starts_with("analysis/previews/") {
        job_root.clone()
    } else {
        job_root.join("extracted")
    };
    let source_path = source_root.join(relative_path);
    ensure_file_within(&source_path, &source_root)?;

    let resource_root = state.project_root.join(project_id).join(resource_id);
    if resource_root.exists() {
        return Err(AppError::Conflict(
            "项目资源存储编号冲突，请重试".to_owned(),
        ));
    }
    std::fs::create_dir_all(&resource_root)?;

    let source_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("resource")
        .to_owned();
    let html_bundle = artifact.artifact_kind == "presentation"
        && source_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("html"));
    let (object_key, entry_path, preview_kind, size_bytes, sha256) = if html_bundle {
        let bundle_source = source_path
            .parent()
            .ok_or_else(|| AppError::BadRequest("HTML 演示目录无效".to_owned()))?;
        let bundle_target = resource_root.join("bundle");
        copy_directory(bundle_source, &bundle_target)?;
        let entry = source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("index.html")
            .to_owned();
        (
            storage_key(project_id, resource_id, "bundle"),
            Some(entry),
            Some("html_bundle".to_owned()),
            Some(artifact.size_bytes),
            Some(sha256_file(&source_path).await?),
        )
    } else {
        let extension = source_path
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| format!(".{}", value.to_ascii_lowercase()))
            .unwrap_or_default();
        let stored_name = format!("original{extension}");
        let target = resource_root.join(&stored_name);
        tokio::fs::copy(&source_path, &target).await?;
        (
            storage_key(project_id, resource_id, &stored_name),
            None,
            Some(infer_preview_kind(
                &source_path,
                artifact.mime_type.as_deref(),
            )),
            Some(tokio::fs::metadata(&target).await?.len() as i64),
            Some(sha256_file(&target).await?),
        )
    };

    Ok(PreparedResource {
        id: resource_id.to_owned(),
        resource_type: input.resource_type.clone(),
        title: input.title.clone(),
        url: None,
        object_key: Some(object_key),
        source_name: Some(source_name),
        mime_type: artifact.mime_type.or_else(|| {
            mime_guess::from_path(&source_path)
                .first_raw()
                .map(str::to_owned)
        }),
        size_bytes,
        display_path: Some(input.title.clone()),
        preview_kind,
        entry_path,
        source_import_job_id: Some(job_id.to_owned()),
        source_artifact_id: Some(artifact_id.to_owned()),
        sha256,
        is_existing: false,
    })
}

pub async fn content(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path((slug, resource_id)): Path<(String, String)>,
) -> Result<Response<Body>, AppError> {
    require_member(&identity)?;
    let resource = load_resource_file(&state, &slug, &resource_id).await?;
    serve_resource_file(&state, &resource, false).await
}

pub async fn import_content(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path((job_id, artifact_id)): Path<(String, String)>,
) -> Result<Response<Body>, AppError> {
    require_member(&identity)?;
    let artifact = load_import_artifact(&state, &job_id, &artifact_id, &identity).await?;
    let path = import_artifact_path(&state, &job_id, &artifact.relative_path)?;
    let mime_type = artifact
        .mime_type
        .as_deref()
        .or_else(|| mime_guess::from_path(&path).first_raw())
        .unwrap_or("application/octet-stream");
    stream_file(&path, mime_type, false).await
}

pub async fn download(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path((slug, resource_id)): Path<(String, String)>,
) -> Result<Response<Body>, AppError> {
    require_member(&identity)?;
    let resource = load_resource_file(&state, &slug, &resource_id).await?;
    serve_resource_file(&state, &resource, true).await
}

pub async fn create_preview(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path((slug, resource_id)): Path<(String, String)>,
) -> Result<Json<PreviewTicket>, AppError> {
    require_member(&identity)?;
    let resource = load_resource_file(&state, &slug, &resource_id).await?;
    if resource.preview_kind.as_deref() != Some("html_bundle") {
        return Err(AppError::BadRequest(
            "该资源不需要 HTML 演示令牌".to_owned(),
        ));
    }
    sqlx::query("DELETE FROM resource_preview_tokens WHERE expires_at <= CURRENT_TIMESTAMP")
        .execute(&state.db)
        .await?;
    let raw_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let token_hash = hash_token(&raw_token);
    let expires_at =
        sqlx::query_scalar::<_, String>("SELECT datetime('now', '+' || ? || ' minutes')")
            .bind(PREVIEW_TOKEN_MINUTES)
            .fetch_one(&state.db)
            .await?;
    sqlx::query(
        "INSERT INTO resource_preview_tokens
            (token_hash, resource_id, created_by_sid, expires_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(token_hash)
    .bind(resource_id)
    .bind(identity.sid)
    .bind(&expires_at)
    .execute(&state.db)
    .await?;
    let entry_path = resource.entry_path.as_deref().unwrap_or("index.html");
    Ok(Json(PreviewTicket {
        url: public_preview_url(
            &state,
            &format!("/api/v1/resource-previews/{raw_token}/{entry_path}"),
        ),
        expires_at,
    }))
}

pub async fn create_import_preview(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path((job_id, artifact_id)): Path<(String, String)>,
) -> Result<Json<PreviewTicket>, AppError> {
    require_member(&identity)?;
    let artifact = load_import_artifact(&state, &job_id, &artifact_id, &identity).await?;
    let path = import_artifact_path(&state, &job_id, &artifact.relative_path)?;
    let is_html = artifact.artifact_kind == "presentation"
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("html"));
    if !is_html {
        return Err(AppError::BadRequest(
            "该导入附件不需要 HTML 演示令牌".to_owned(),
        ));
    }
    sqlx::query("DELETE FROM import_artifact_preview_tokens WHERE expires_at <= CURRENT_TIMESTAMP")
        .execute(&state.db)
        .await?;
    let raw_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let expires_at =
        sqlx::query_scalar::<_, String>("SELECT datetime('now', '+' || ? || ' minutes')")
            .bind(PREVIEW_TOKEN_MINUTES)
            .fetch_one(&state.db)
            .await?;
    sqlx::query(
        "INSERT INTO import_artifact_preview_tokens
            (token_hash, job_id, artifact_id, created_by_sid, expires_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(hash_token(&raw_token))
    .bind(&job_id)
    .bind(&artifact_id)
    .bind(&identity.sid)
    .bind(&expires_at)
    .execute(&state.db)
    .await?;
    let entry = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("index.html");
    Ok(Json(PreviewTicket {
        url: public_preview_url(
            &state,
            &format!("/api/v1/import-previews/{raw_token}/{entry}"),
        ),
        expires_at,
    }))
}

pub async fn preview_index(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response<Body>, AppError> {
    serve_preview_asset(&state, &token, None).await
}

pub async fn preview_asset(
    State(state): State<AppState>,
    Path((token, path)): Path<(String, String)>,
) -> Result<Response<Body>, AppError> {
    serve_preview_asset(&state, &token, Some(path)).await
}

pub async fn import_preview_asset(
    State(state): State<AppState>,
    Path((token, path)): Path<(String, String)>,
) -> Result<Response<Body>, AppError> {
    let artifact = sqlx::query_as::<_, (String, String)>(
        "SELECT t.job_id, a.relative_path
         FROM import_artifact_preview_tokens t
         JOIN import_artifacts a ON a.id = t.artifact_id AND a.job_id = t.job_id
         WHERE t.token_hash = ? AND t.expires_at > CURRENT_TIMESTAMP",
    )
    .bind(hash_token(&token))
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let entry_path = import_artifact_path(&state, &artifact.0, &artifact.1)?;
    let bundle_root = entry_path.parent().ok_or(AppError::NotFound)?;
    let relative_path = FsPath::new(&path);
    if !is_safe_relative_path(relative_path) {
        return Err(AppError::NotFound);
    }
    let file_path = bundle_root.join(relative_path);
    ensure_file_within(&file_path, bundle_root).map_err(|_| AppError::NotFound)?;
    let mime_type = mime_guess::from_path(&file_path)
        .first_raw()
        .unwrap_or("application/octet-stream");
    let mut response = stream_file(&file_path, mime_type, false).await?;
    apply_html_preview_headers(&mut response);
    Ok(response)
}

async fn serve_preview_asset(
    state: &AppState,
    token: &str,
    requested_path: Option<String>,
) -> Result<Response<Body>, AppError> {
    let resource = sqlx::query_as::<_, ResourceFileRow>(
        "SELECT r.object_key, r.entry_path, r.mime_type, r.preview_kind
         FROM resource_preview_tokens t
         JOIN resources r ON r.id = t.resource_id
         WHERE t.token_hash = ? AND t.expires_at > CURRENT_TIMESTAMP",
    )
    .bind(hash_token(token))
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if resource.preview_kind.as_deref() != Some("html_bundle") {
        return Err(AppError::NotFound);
    }
    let object_key = FsPath::new(&resource.object_key);
    if !is_safe_relative_path(object_key) {
        return Err(AppError::NotFound);
    }
    let bundle_root = state.project_root.join(object_key);
    let relative = requested_path
        .filter(|value| !value.is_empty())
        .or(resource.entry_path)
        .ok_or(AppError::NotFound)?;
    let relative_path = FsPath::new(&relative);
    if !is_safe_relative_path(relative_path) {
        return Err(AppError::NotFound);
    }
    let file_path = bundle_root.join(relative_path);
    ensure_file_within(&file_path, &bundle_root).map_err(|_| AppError::NotFound)?;
    let mime_type = mime_guess::from_path(&file_path)
        .first_raw()
        .unwrap_or("application/octet-stream");
    let mut response = stream_file(&file_path, mime_type, false).await?;
    apply_html_preview_headers(&mut response);
    Ok(response)
}

async fn load_import_artifact(
    state: &AppState,
    job_id: &str,
    artifact_id: &str,
    identity: &FeiyueIdentity,
) -> Result<ImportArtifactRow, AppError> {
    let artifact = sqlx::query_as::<_, ImportArtifactRow>(
        "SELECT a.relative_path, a.artifact_kind, a.mime_type, a.size_bytes,
                j.created_by_sid, j.status job_status
         FROM import_artifacts a
         JOIN import_jobs j ON j.id = a.job_id
         WHERE a.id = ? AND a.job_id = ?",
    )
    .bind(artifact_id)
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if artifact.created_by_sid != identity.sid && !identity.is_superadmin() {
        return Err(AppError::Forbidden);
    }
    Ok(artifact)
}

fn import_artifact_path(
    state: &AppState,
    job_id: &str,
    relative_path: &str,
) -> Result<std::path::PathBuf, AppError> {
    let relative = FsPath::new(relative_path);
    if !is_safe_relative_path(relative) {
        return Err(AppError::NotFound);
    }
    let job_root = state.import_root.join(job_id);
    let source_root = if relative_path.starts_with("analysis/previews/") {
        job_root
    } else {
        job_root.join("extracted")
    };
    let path = source_root.join(relative);
    ensure_file_within(&path, &source_root).map_err(|_| AppError::NotFound)?;
    Ok(path)
}

fn public_preview_url(state: &AppState, path: &str) -> String {
    state
        .preview_public_base_url
        .as_ref()
        .as_deref()
        .map(|base| format!("{base}{path}"))
        .unwrap_or_else(|| path.to_owned())
}

fn apply_html_preview_headers(response: &mut Response<Body>) {
    response.headers_mut().insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self' data: blob:; script-src 'self' 'unsafe-inline' 'unsafe-eval' blob:; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' data: blob:; font-src 'self' data:; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors https://icthub.top http://127.0.0.1:* http://localhost:*",
        ),
    );
}

async fn load_resource_file(
    state: &AppState,
    slug: &str,
    resource_id: &str,
) -> Result<ResourceFileRow, AppError> {
    sqlx::query_as::<_, ResourceFileRow>(
        "SELECT r.object_key, r.entry_path, r.mime_type, r.preview_kind
         FROM resources r
         JOIN projects p ON p.id = r.project_id
         WHERE p.slug = ? AND p.archived_at IS NULL AND r.id = ? AND r.object_key IS NOT NULL",
    )
    .bind(slug)
    .bind(resource_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)
}

async fn serve_resource_file(
    state: &AppState,
    resource: &ResourceFileRow,
    attachment: bool,
) -> Result<Response<Body>, AppError> {
    let object_key = FsPath::new(&resource.object_key);
    if !is_safe_relative_path(object_key) {
        return Err(AppError::NotFound);
    }
    let mut path = state.project_root.join(object_key);
    if path.is_dir() {
        let entry = resource.entry_path.as_deref().ok_or(AppError::NotFound)?;
        let entry_path = FsPath::new(entry);
        if !is_safe_relative_path(entry_path) {
            return Err(AppError::NotFound);
        }
        path = path.join(entry_path);
    }
    ensure_file_within(&path, state.project_root.as_ref()).map_err(|_| AppError::NotFound)?;
    let mime_type = resource
        .mime_type
        .as_deref()
        .or_else(|| mime_guess::from_path(&path).first_raw())
        .unwrap_or("application/octet-stream");
    stream_file(&path, mime_type, attachment).await
}

async fn stream_file(
    path: &FsPath,
    mime_type: &str,
    attachment: bool,
) -> Result<Response<Body>, AppError> {
    let file = tokio::fs::File::open(path).await?;
    let size = file.metadata().await?.len();
    let mut response = Response::new(Body::from_stream(ReaderStream::new(file)));
    response.headers_mut().insert(
        CONTENT_TYPE,
        safe_header(mime_type, "application/octet-stream"),
    );
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, safe_header(&size.to_string(), "0"));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    response
        .headers_mut()
        .insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    if attachment {
        response
            .headers_mut()
            .insert(CONTENT_DISPOSITION, HeaderValue::from_static("attachment"));
    }
    Ok(response)
}

fn require_member(identity: &FeiyueIdentity) -> Result<(), AppError> {
    if identity.can_access_icthub() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn storage_key(project_id: &str, resource_id: &str, leaf: &str) -> String {
    format!("{project_id}/{resource_id}/{leaf}")
}

fn infer_preview_kind(path: &FsPath, mime_type: Option<&str>) -> String {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "pdf" => "pdf",
        "docx" => "docx",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image",
        "mp4" | "webm" | "mov" | "m4v" => "video",
        "txt" | "md" | "rs" | "ts" | "tsx" | "js" | "jsx" | "json" | "toml" | "yaml" | "yml"
        | "py" | "java" | "c" | "cpp" | "h" | "hpp" | "css" | "html" => "code",
        _ if mime_type.is_some_and(|value| value.starts_with("image/")) => "image",
        _ => "download",
    }
    .to_owned()
}

fn copy_directory(source: &FsPath, target: &FsPath) -> io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTML bundle contains a symbolic link",
            ));
        }
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn ensure_file_within(path: &FsPath, root: &FsPath) -> io::Result<()> {
    let canonical_root = std::fs::canonicalize(root)?;
    let canonical_path = std::fs::canonicalize(path)?;
    if !canonical_path.starts_with(canonical_root) || !canonical_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "resource path escaped its storage root",
        ));
    }
    Ok(())
}

fn is_safe_relative_path(path: &FsPath) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path.components().all(|component| {
            matches!(component, Component::Normal(_))
                && component.as_os_str() != "."
                && component.as_os_str() != ".."
        })
}

async fn sha256_file(path: &FsPath) -> Result<String, AppError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn safe_header(value: &str, fallback: &'static str) -> HeaderValue {
    HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static(fallback))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{infer_preview_kind, is_safe_relative_path};

    #[test]
    fn storage_paths_reject_traversal() {
        assert!(is_safe_relative_path(Path::new(
            "project/resource/original.pdf"
        )));
        assert!(!is_safe_relative_path(Path::new("../secret")));
        assert!(!is_safe_relative_path(Path::new("C:/secret")));
    }

    #[test]
    fn common_documents_get_preview_kinds() {
        assert_eq!(infer_preview_kind(Path::new("report.pdf"), None), "pdf");
        assert_eq!(infer_preview_kind(Path::new("report.docx"), None), "docx");
        assert_eq!(infer_preview_kind(Path::new("poster.png"), None), "image");
    }
}
