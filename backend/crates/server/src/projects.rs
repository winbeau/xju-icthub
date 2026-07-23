use std::collections::HashSet;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Sqlite, Transaction};
use uuid::Uuid;

use crate::{
    auth::{AuthContext, FeiyueIdentity},
    covers,
    error::AppError,
    resources::{prepare_project_resources, PreparedResource},
    state::AppState,
};

pub const PROJECT_CATEGORIES: &[&str] =
    &["传统软件", "智能硬件", "AI 软件", "数字媒体", "研究成果"];
const RESOURCE_TYPES: &[&str] = &[
    "github",
    "baidu",
    "document",
    "presentation",
    "archive",
    "video",
    "image",
    "link",
];

#[derive(Debug, Deserialize)]
pub struct ProjectListQuery {
    pub q: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, FromRow)]
struct ProjectSummaryRow {
    id: String,
    slug: String,
    name: String,
    summary: String,
    primary_category: String,
    highest_award: Option<String>,
    status: String,
    cover_mode: String,
    cover_resource_id: Option<String>,
    cover_resource_url: Option<String>,
    cover_title: Option<String>,
    cover_subtitle: Option<String>,
    cover_keywords: String,
    cover_tone: String,
    cover_confidence: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub primary_category: String,
    pub highest_award: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
    pub cover_mode: String,
    pub cover_resource_id: Option<String>,
    pub cover_resource_url: Option<String>,
    pub cover_title: String,
    pub cover_subtitle: String,
    pub cover_keywords: Vec<String>,
    pub cover_tone: String,
    pub cover_confidence: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub items: Vec<ProjectSummary>,
    pub total: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResourceInput {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub title: String,
    pub url: Option<String>,
    #[serde(default)]
    pub source_import_job_id: Option<String>,
    #[serde(default)]
    pub source_artifact_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWriteInput {
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub primary_category: String,
    pub highest_award: Option<String>,
    pub status: String,
    pub critique: String,
    pub owner_name: Option<String>,
    pub source_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub resources: Vec<ProjectResourceInput>,
    #[serde(default)]
    pub cover_mode: Option<String>,
    #[serde(default)]
    pub cover_title: Option<String>,
    #[serde(default)]
    pub cover_subtitle: Option<String>,
    #[serde(default)]
    pub cover_keywords: Vec<String>,
    #[serde(default)]
    pub cover_tone: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResource {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub title: String,
    pub url: Option<String>,
    pub source_name: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub display_path: Option<String>,
    pub preview_kind: Option<String>,
    pub content_url: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, FromRow)]
struct ProjectResourceRow {
    id: String,
    resource_type: String,
    title: String,
    url: Option<String>,
    object_key: Option<String>,
    source_name: Option<String>,
    mime_type: Option<String>,
    size_bytes: Option<i64>,
    display_path: Option<String>,
    preview_kind: Option<String>,
}

#[derive(Debug, FromRow)]
struct ProjectDetailRow {
    id: String,
    slug: String,
    name: String,
    summary: String,
    primary_category: String,
    classification_status: String,
    highest_award: Option<String>,
    status: String,
    critique: String,
    owner_name: Option<String>,
    source_name: Option<String>,
    cover_mode: String,
    cover_resource_id: Option<String>,
    cover_resource_url: Option<String>,
    cover_title: Option<String>,
    cover_subtitle: Option<String>,
    cover_keywords: String,
    cover_tone: String,
    cover_confidence: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetail {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub primary_category: String,
    pub classification_status: String,
    pub highest_award: Option<String>,
    pub status: String,
    pub critique: String,
    pub owner_name: Option<String>,
    pub source_name: Option<String>,
    pub resources: Vec<ProjectResource>,
    pub tags: Vec<String>,
    pub cover_mode: String,
    pub cover_resource_id: Option<String>,
    pub cover_resource_url: Option<String>,
    pub cover_title: String,
    pub cover_subtitle: String,
    pub cover_keywords: Vec<String>,
    pub cover_tone: String,
    pub cover_confidence: f64,
}

pub async fn list(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Query(query): Query<ProjectListQuery>,
) -> Result<Json<ProjectListResponse>, AppError> {
    require_member(&identity)?;
    if let Some(category) = query.category.as_deref() {
        if !PROJECT_CATEGORIES.contains(&category) {
            return Err(AppError::BadRequest("项目类别不受支持".to_owned()));
        }
    }

    let rows = sqlx::query_as::<_, ProjectSummaryRow>(
        "SELECT p.id, p.slug, p.name, p.summary, p.primary_category, p.highest_award, p.status,
                p.cover_mode, p.cover_resource_id,
                (SELECT url FROM resources r WHERE r.id = p.cover_resource_id) cover_resource_url,
                p.cover_title, p.cover_subtitle, p.cover_keywords, p.cover_tone, p.cover_confidence
         FROM projects p
         WHERE p.archived_at IS NULL AND p.classification_status = 'classified'
         ORDER BY p.updated_at DESC, p.name ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let needle = query
        .q
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        if query
            .category
            .as_deref()
            .is_some_and(|category| category != row.primary_category)
        {
            continue;
        }
        if needle.as_deref().is_some_and(|needle| {
            !format!(
                "{} {} {}",
                row.name,
                row.summary,
                row.highest_award.as_deref().unwrap_or("")
            )
            .to_lowercase()
            .contains(needle)
        }) {
            continue;
        }
        items.push(summary_from_row(&state, row).await?);
    }

    let total = items.len();
    Ok(Json(ProjectListResponse { items, total }))
}

pub async fn detail(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(slug): Path<String>,
) -> Result<Json<ProjectDetail>, AppError> {
    require_member(&identity)?;
    Ok(Json(load_detail(&state, &slug).await?))
}

pub async fn create(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Json(input): Json<ProjectWriteInput>,
) -> Result<(StatusCode, Json<ProjectDetail>), AppError> {
    require_member(&identity)?;
    let input = input.normalized()?;
    validate_tags(&state, &input.tags).await?;
    if sqlx::query_scalar::<_, String>("SELECT id FROM projects WHERE slug = ?")
        .bind(&input.slug)
        .fetch_optional(&state.db)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(format!(
            "项目路径 {} 已存在",
            input.slug
        )));
    }

    let id = Uuid::new_v4().to_string();
    let resources = prepare_project_resources(&state, &id, &identity, &input.resources).await?;
    let mut tx = state.db.begin().await?;
    if sqlx::query_scalar::<_, String>("SELECT id FROM projects WHERE slug = ?")
        .bind(&input.slug)
        .fetch_optional(&mut *tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(format!(
            "项目路径 {} 已存在",
            input.slug
        )));
    }
    insert_project(&mut tx, &id, &input, &identity.sid).await?;
    replace_children(&mut tx, &id, &input, &resources, &identity.sid).await?;
    tx.commit().await?;
    if input.cover_mode.as_deref() != Some("manual") {
        covers::generate_for_slug(&state, &input.slug, false).await?;
    }
    Ok((
        StatusCode::CREATED,
        Json(load_detail(&state, &input.slug).await?),
    ))
}

pub async fn update(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(current_slug): Path<String>,
    Json(input): Json<ProjectWriteInput>,
) -> Result<Json<ProjectDetail>, AppError> {
    require_member(&identity)?;
    let input = input.normalized()?;
    validate_tags(&state, &input.tags).await?;
    let id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM projects WHERE slug = ? AND archived_at IS NULL",
    )
    .bind(&current_slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if sqlx::query_scalar::<_, String>("SELECT id FROM projects WHERE slug = ?")
        .bind(&input.slug)
        .fetch_optional(&state.db)
        .await?
        .as_deref()
        .is_some_and(|owner| owner != id)
    {
        return Err(AppError::Conflict(format!(
            "项目路径 {} 已存在",
            input.slug
        )));
    }
    let resources = prepare_project_resources(&state, &id, &identity, &input.resources).await?;
    let mut tx = state.db.begin().await?;
    update_project(&mut tx, &id, &input).await?;
    replace_children(&mut tx, &id, &input, &resources, &identity.sid).await?;
    tx.commit().await?;
    if input.cover_mode.as_deref() != Some("manual") {
        covers::generate_for_slug(&state, &input.slug, false).await?;
    }
    Ok(Json(load_detail(&state, &input.slug).await?))
}

pub async fn commit_import(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(job_id): Path<String>,
    Json(input): Json<ProjectWriteInput>,
) -> Result<(StatusCode, Json<ProjectDetail>), AppError> {
    require_member(&identity)?;
    let input = input.normalized()?;
    validate_tags(&state, &input.tags).await?;
    let job = sqlx::query_as::<_, (String, String)>(
        "SELECT created_by_sid, status FROM import_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if job.0 != identity.sid && !identity.is_superadmin() {
        return Err(AppError::Forbidden);
    }
    if !matches!(
        job.1.as_str(),
        "completed" | "agent_queued" | "agent_running"
    ) {
        return Err(AppError::Conflict("导入任务尚未整理完成".to_owned()));
    }

    let committed_project_id = sqlx::query_scalar::<_, String>(
        "SELECT project_id FROM project_import_commits WHERE job_id = ?",
    )
    .bind(&job_id)
    .fetch_optional(&state.db)
    .await?;
    let slug_project_id = sqlx::query_scalar::<_, String>("SELECT id FROM projects WHERE slug = ?")
        .bind(&input.slug)
        .fetch_optional(&state.db)
        .await?;
    let (project_id, created) = match (committed_project_id, slug_project_id) {
        (Some(committed), Some(slug_owner)) if committed != slug_owner => {
            return Err(AppError::Conflict(format!(
                "项目路径 {} 已被其他项目使用",
                input.slug
            )));
        }
        (Some(committed), _) => (committed, false),
        (None, Some(existing)) => (existing, false),
        (None, None) => (Uuid::new_v4().to_string(), true),
    };
    let resources =
        prepare_project_resources(&state, &project_id, &identity, &input.resources).await?;
    let mut tx = state.db.begin().await?;
    if created {
        insert_project(&mut tx, &project_id, &input, &identity.sid).await?;
    } else {
        update_project(&mut tx, &project_id, &input).await?;
    }
    replace_children(&mut tx, &project_id, &input, &resources, &identity.sid).await?;
    sqlx::query(
        "INSERT INTO project_import_commits (job_id, project_id, committed_by_sid)
         VALUES (?, ?, ?)
         ON CONFLICT(job_id) DO UPDATE SET project_id = excluded.project_id,
             committed_by_sid = excluded.committed_by_sid, committed_at = CURRENT_TIMESTAMP",
    )
    .bind(&job_id)
    .bind(&project_id)
    .bind(&identity.sid)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    if input.cover_mode.as_deref() != Some("manual") {
        covers::generate_for_slug(&state, &input.slug, false).await?;
    }
    Ok((
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(load_detail(&state, &input.slug).await?),
    ))
}

pub async fn archive(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(slug): Path<String>,
) -> Result<StatusCode, AppError> {
    require_member(&identity)?;
    let result = sqlx::query(
        "UPDATE projects SET archived_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE slug = ? AND archived_at IS NULL",
    )
    .bind(slug)
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn require_member(identity: &FeiyueIdentity) -> Result<(), AppError> {
    if identity.can_access_icthub() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

impl ProjectWriteInput {
    fn normalized(mut self) -> Result<Self, AppError> {
        self.slug = self.slug.trim().to_lowercase();
        self.name = self.name.trim().to_owned();
        self.summary = self.summary.trim().to_owned();
        self.primary_category = self.primary_category.trim().to_owned();
        self.status = self.status.trim().to_owned();
        self.critique = self.critique.trim().to_owned();
        self.highest_award = normalize_optional(self.highest_award);
        self.owner_name = normalize_optional(self.owner_name);
        self.source_name = normalize_optional(self.source_name);
        self.cover_mode = normalize_optional(self.cover_mode).map(|value| value.to_lowercase());
        self.cover_title = normalize_optional(self.cover_title);
        self.cover_subtitle = normalize_optional(self.cover_subtitle);
        self.cover_tone = normalize_optional(self.cover_tone);

        if self.name.is_empty() || self.summary.is_empty() {
            return Err(AppError::BadRequest("项目名和简介不能为空".to_owned()));
        }
        if self.name.chars().count() > 120 || self.summary.chars().count() > 500 {
            return Err(AppError::BadRequest("项目名或简介过长".to_owned()));
        }
        if !valid_slug(&self.slug) {
            return Err(AppError::BadRequest(
                "项目路径只能包含小写字母、数字和连字符".to_owned(),
            ));
        }
        if !PROJECT_CATEGORIES.contains(&self.primary_category.as_str()) {
            return Err(AppError::BadRequest("项目类别不受支持".to_owned()));
        }
        if self.status.is_empty() {
            return Err(AppError::BadRequest("项目状态不能为空".to_owned()));
        }
        if self
            .cover_mode
            .as_deref()
            .is_some_and(|mode| !["manual", "resource", "text"].contains(&mode))
        {
            return Err(AppError::BadRequest("封面模式不受支持".to_owned()));
        }

        let mut seen_tags = HashSet::new();
        self.tags = self
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_owned())
            .filter(|tag| !tag.is_empty())
            .filter(|tag| seen_tags.insert(tag.to_lowercase()))
            .take(20)
            .collect();
        self.cover_keywords = self
            .cover_keywords
            .into_iter()
            .map(|keyword| keyword.trim().to_owned())
            .filter(|keyword| !keyword.is_empty())
            .take(3)
            .collect();

        for resource in &mut self.resources {
            resource.id = normalize_optional(resource.id.take());
            resource.resource_type = resource.resource_type.trim().to_lowercase();
            resource.title = resource.title.trim().to_owned();
            resource.url = normalize_optional(resource.url.take());
            resource.source_import_job_id =
                normalize_optional(resource.source_import_job_id.take());
            resource.source_artifact_id = normalize_optional(resource.source_artifact_id.take());
            if !RESOURCE_TYPES.contains(&resource.resource_type.as_str()) {
                return Err(AppError::BadRequest(format!(
                    "资源类型 {} 不受支持",
                    resource.resource_type
                )));
            }
            if resource.title.is_empty() {
                return Err(AppError::BadRequest("资源标题不能为空".to_owned()));
            }
            if resource.source_import_job_id.is_some() != resource.source_artifact_id.is_some() {
                return Err(AppError::BadRequest(
                    "导入资源必须同时提供任务编号和附件编号".to_owned(),
                ));
            }
            if resource.id.is_some() && resource.source_artifact_id.is_some() {
                return Err(AppError::BadRequest(
                    "已有资源不能重复绑定导入附件".to_owned(),
                ));
            }
        }
        Ok(self)
    }
}

async fn validate_tags(state: &AppState, tags: &[String]) -> Result<(), AppError> {
    for tag in tags {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tag_definitions WHERE name = ? COLLATE NOCASE AND is_active = 1",
        )
        .bind(tag)
        .fetch_one(&state.db)
        .await?;
        if exists == 0 {
            return Err(AppError::BadRequest(format!(
                "标签“{tag}”不是启用中的正式标签，请先提交标签建议"
            )));
        }
    }
    Ok(())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

async fn insert_project(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    input: &ProjectWriteInput,
    actor_sid: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO projects (
            id, slug, name, summary, primary_category, classification_status, status, critique,
            highest_award, owner_name, source_name, created_by_sid, cover_mode, cover_title,
            cover_subtitle, cover_keywords, cover_tone
         ) VALUES (?, ?, ?, ?, ?, 'classified', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&input.slug)
    .bind(&input.name)
    .bind(&input.summary)
    .bind(&input.primary_category)
    .bind(&input.status)
    .bind(&input.critique)
    .bind(&input.highest_award)
    .bind(&input.owner_name)
    .bind(&input.source_name)
    .bind(actor_sid)
    .bind(input.cover_mode.as_deref().unwrap_or("text"))
    .bind(&input.cover_title)
    .bind(&input.cover_subtitle)
    .bind(serde_json::to_string(&input.cover_keywords).unwrap_or_else(|_| "[]".to_owned()))
    .bind(input.cover_tone.as_deref().unwrap_or("slate"))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn update_project(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    input: &ProjectWriteInput,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE projects SET slug = ?, name = ?, summary = ?, primary_category = ?,
            classification_status = 'classified', status = ?, critique = ?, highest_award = ?,
            owner_name = ?, source_name = ?, cover_mode = ?, cover_resource_id = NULL,
            cover_title = ?, cover_subtitle = ?, cover_keywords = ?, cover_tone = ?,
            updated_at = CURRENT_TIMESTAMP, archived_at = NULL WHERE id = ?",
    )
    .bind(&input.slug)
    .bind(&input.name)
    .bind(&input.summary)
    .bind(&input.primary_category)
    .bind(&input.status)
    .bind(&input.critique)
    .bind(&input.highest_award)
    .bind(&input.owner_name)
    .bind(&input.source_name)
    .bind(input.cover_mode.as_deref().unwrap_or("text"))
    .bind(&input.cover_title)
    .bind(&input.cover_subtitle)
    .bind(serde_json::to_string(&input.cover_keywords).unwrap_or_else(|_| "[]".to_owned()))
    .bind(input.cover_tone.as_deref().unwrap_or("slate"))
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn replace_children(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &str,
    input: &ProjectWriteInput,
    resources: &[PreparedResource],
    actor_sid: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM project_tags WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut **tx)
        .await?;

    for (index, tag) in input.tags.iter().enumerate() {
        let definition_id = sqlx::query_scalar::<_, String>(
            "SELECT id FROM tag_definitions WHERE name = ? COLLATE NOCASE AND is_active = 1",
        )
        .bind(tag)
        .fetch_one(&mut **tx)
        .await?;
        sqlx::query(
            "INSERT INTO project_tags (project_id, tag, sort_order, tag_definition_id) VALUES (?, ?, ?, ?)",
        )
        .bind(project_id).bind(tag).bind(index as i64).bind(definition_id)
        .execute(&mut **tx).await?;
    }
    let existing_ids =
        sqlx::query_scalar::<_, String>("SELECT id FROM resources WHERE project_id = ?")
            .bind(project_id)
            .fetch_all(&mut **tx)
            .await?;
    let retained = resources
        .iter()
        .map(|resource| resource.id.as_str())
        .collect::<HashSet<_>>();
    for id in existing_ids {
        if !retained.contains(id.as_str()) {
            sqlx::query("DELETE FROM resources WHERE id = ? AND project_id = ?")
                .bind(id)
                .bind(project_id)
                .execute(&mut **tx)
                .await?;
        }
    }
    for resource in resources {
        if resource.is_existing {
            sqlx::query(
                "UPDATE resources SET resource_type = ?, title = ?, url = ?, availability = 'available'
                 WHERE id = ? AND project_id = ?",
            )
            .bind(&resource.resource_type)
            .bind(&resource.title)
            .bind(&resource.url)
            .bind(&resource.id)
            .bind(project_id)
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO resources (
                    id, project_id, resource_type, title, url, object_key, source_name,
                    availability, created_by_sid, mime_type, size_bytes, display_path,
                    preview_kind, entry_path, source_import_job_id, source_artifact_id, sha256
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, 'available', ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&resource.id)
            .bind(project_id)
            .bind(&resource.resource_type)
            .bind(&resource.title)
            .bind(&resource.url)
            .bind(&resource.object_key)
            .bind(&resource.source_name)
            .bind(actor_sid)
            .bind(&resource.mime_type)
            .bind(resource.size_bytes)
            .bind(&resource.display_path)
            .bind(&resource.preview_kind)
            .bind(&resource.entry_path)
            .bind(&resource.source_import_job_id)
            .bind(&resource.source_artifact_id)
            .bind(&resource.sha256)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

async fn summary_from_row(
    state: &AppState,
    row: ProjectSummaryRow,
) -> Result<ProjectSummary, AppError> {
    let tags = load_tags(state, &row.id, 3).await?;
    Ok(ProjectSummary {
        id: row.id,
        slug: row.slug,
        name: row.name,
        summary: row.summary,
        primary_category: row.primary_category,
        highest_award: row.highest_award,
        status: row.status,
        tags,
        cover_mode: row.cover_mode,
        cover_resource_id: row.cover_resource_id,
        cover_resource_url: row.cover_resource_url,
        cover_title: row.cover_title.unwrap_or_else(|| "项目封面".to_owned()),
        cover_subtitle: row.cover_subtitle.unwrap_or_default(),
        cover_keywords: parse_keywords(&row.cover_keywords),
        cover_tone: row.cover_tone,
        cover_confidence: row.cover_confidence.unwrap_or(0.0),
    })
}

async fn load_detail(state: &AppState, slug: &str) -> Result<ProjectDetail, AppError> {
    let row = sqlx::query_as::<_, ProjectDetailRow>(
        "SELECT p.id, p.slug, p.name, p.summary, p.primary_category, p.classification_status,
                p.highest_award, p.status, p.critique, p.owner_name, p.source_name, p.cover_mode,
                p.cover_resource_id, (SELECT url FROM resources r WHERE r.id = p.cover_resource_id) cover_resource_url,
                p.cover_title, p.cover_subtitle, p.cover_keywords, p.cover_tone, p.cover_confidence
         FROM projects p WHERE p.slug = ? AND p.archived_at IS NULL",
    )
    .bind(slug).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;
    let resource_rows = sqlx::query_as::<_, ProjectResourceRow>(
        "SELECT id, resource_type, title, url, object_key, source_name, mime_type, size_bytes,
                display_path, preview_kind
         FROM resources WHERE project_id = ? ORDER BY created_at ASC, title ASC",
    )
    .bind(&row.id)
    .fetch_all(&state.db)
    .await?;
    let resources = resource_rows
        .into_iter()
        .map(|resource| {
            let has_object = resource.object_key.is_some();
            ProjectResource {
                id: resource.id.clone(),
                resource_type: resource.resource_type,
                title: resource.title,
                url: resource.url,
                source_name: resource.source_name,
                mime_type: resource.mime_type,
                size_bytes: resource.size_bytes,
                display_path: resource.display_path,
                preview_kind: resource.preview_kind,
                content_url: has_object.then(|| {
                    format!(
                        "/api/v1/projects/{}/resources/{}/content",
                        row.slug, resource.id
                    )
                }),
                download_url: has_object.then(|| {
                    format!(
                        "/api/v1/projects/{}/resources/{}/download",
                        row.slug, resource.id
                    )
                }),
            }
        })
        .collect();
    let tags = load_tags(state, &row.id, 20).await?;
    Ok(ProjectDetail {
        id: row.id,
        slug: row.slug,
        name: row.name,
        summary: row.summary,
        primary_category: row.primary_category,
        classification_status: row.classification_status,
        highest_award: row.highest_award,
        status: row.status,
        critique: row.critique,
        owner_name: row.owner_name,
        source_name: row.source_name,
        resources,
        tags,
        cover_mode: row.cover_mode,
        cover_resource_id: row.cover_resource_id,
        cover_resource_url: row.cover_resource_url,
        cover_title: row.cover_title.unwrap_or_else(|| "项目封面".to_owned()),
        cover_subtitle: row.cover_subtitle.unwrap_or_default(),
        cover_keywords: parse_keywords(&row.cover_keywords),
        cover_tone: row.cover_tone,
        cover_confidence: row.cover_confidence.unwrap_or(0.0),
    })
}

async fn load_tags(
    state: &AppState,
    project_id: &str,
    limit: i64,
) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT tag FROM project_tags WHERE project_id = ? ORDER BY sort_order ASC, tag ASC LIMIT ?",
    )
    .bind(project_id).bind(limit).fetch_all(&state.db).await?)
}

fn parse_keywords(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{ProjectResourceInput, ProjectWriteInput, PROJECT_CATEGORIES};

    fn valid_input() -> ProjectWriteInput {
        ProjectWriteInput {
            slug: "lab-tool".to_owned(),
            name: "实验室工具".to_owned(),
            summary: "用于测试项目写入。".to_owned(),
            primary_category: "传统软件".to_owned(),
            highest_award: None,
            status: "研发中".to_owned(),
            critique: String::new(),
            owner_name: None,
            source_name: None,
            tags: vec!["Web".to_owned(), " Web ".to_owned()],
            resources: vec![ProjectResourceInput {
                id: None,
                resource_type: "github".to_owned(),
                title: "代码".to_owned(),
                url: Some("https://github.com/example/repo".to_owned()),
                source_import_job_id: None,
                source_artifact_id: None,
            }],
            cover_mode: None,
            cover_title: None,
            cover_subtitle: None,
            cover_keywords: vec![],
            cover_tone: None,
        }
    }

    #[test]
    fn categories_are_exactly_the_five_product_categories() {
        assert_eq!(
            PROJECT_CATEGORIES,
            ["传统软件", "智能硬件", "AI 软件", "数字媒体", "研究成果"]
        );
        for category in PROJECT_CATEGORIES {
            let mut input = valid_input();
            input.primary_category = (*category).to_owned();
            assert!(input.normalized().is_ok());
        }
        let mut old = valid_input();
        old.primary_category = "工具项目".to_owned();
        assert!(old.normalized().is_err());
    }

    #[test]
    fn normalizes_and_deduplicates_project_input() {
        let input = valid_input().normalized().expect("valid project");
        assert_eq!(input.tags, vec!["Web"]);
        assert_eq!(input.resources[0].resource_type, "github");
    }

    #[test]
    fn rejects_invalid_slug() {
        let mut input = valid_input();
        input.slug = "中文 路径".to_owned();
        assert!(input.normalized().is_err());
    }
}
