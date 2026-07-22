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
    error::AppError,
    state::AppState,
};

const PROJECT_CATEGORIES: &[&str] = &["互联网+", "计算机设计大赛", "论文", "工具项目", "其他"];
const RESOURCE_TYPES: &[&str] = &["github", "baidu", "document", "archive", "video", "link"];

#[derive(Debug, Deserialize)]
pub struct ProjectListQuery {
    pub q: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub primary_category: String,
    pub highest_award: Option<String>,
    pub status: String,
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
    #[serde(rename = "type")]
    pub resource_type: String,
    pub title: String,
    pub url: Option<String>,
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectImportRequest {
    pub items: Vec<ProjectWriteInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectImportResponse {
    pub created: usize,
    pub updated: usize,
    pub total: usize,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResource {
    pub id: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "resource_type")]
    pub resource_type: String,
    pub title: String,
    pub url: Option<String>,
}

#[derive(Debug, FromRow)]
struct ProjectDetailRow {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub primary_category: String,
    pub highest_award: Option<String>,
    pub status: String,
    pub critique: String,
    pub owner_name: Option<String>,
    pub source_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetail {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub primary_category: String,
    pub highest_award: Option<String>,
    pub status: String,
    pub critique: String,
    pub owner_name: Option<String>,
    pub source_name: Option<String>,
    pub resources: Vec<ProjectResource>,
    pub tags: Vec<String>,
}

pub async fn list(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Query(query): Query<ProjectListQuery>,
) -> Result<Json<ProjectListResponse>, AppError> {
    require_member(&identity)?;
    let mut items = sqlx::query_as::<_, ProjectSummary>(
        "SELECT id, slug, name, summary, primary_category, highest_award, status
         FROM projects WHERE archived_at IS NULL ORDER BY updated_at DESC, name ASC",
    )
    .fetch_all(&state.db)
    .await?;

    if let Some(category) = query.category {
        items.retain(|item| item.primary_category == category);
    }
    if let Some(needle) = query
        .q
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
    {
        items.retain(|item| {
            format!(
                "{} {} {}",
                item.name,
                item.summary,
                item.highest_award.as_deref().unwrap_or("")
            )
            .to_lowercase()
            .contains(&needle)
        });
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
    let mut tx = state.db.begin().await?;

    let exists = sqlx::query_scalar::<_, String>("SELECT id FROM projects WHERE slug = ?")
        .bind(&input.slug)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_some() {
        return Err(AppError::Conflict(format!(
            "项目路径 {} 已存在",
            input.slug
        )));
    }

    let id = Uuid::new_v4().to_string();
    insert_project(&mut tx, &id, &input, &identity.sid).await?;
    replace_children(&mut tx, &id, &input, &identity.sid).await?;
    tx.commit().await?;

    let project = load_detail(&state, &input.slug).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

pub async fn update(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(current_slug): Path<String>,
    Json(input): Json<ProjectWriteInput>,
) -> Result<Json<ProjectDetail>, AppError> {
    require_member(&identity)?;
    let input = input.normalized()?;
    let mut tx = state.db.begin().await?;

    let id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM projects WHERE slug = ? AND archived_at IS NULL",
    )
    .bind(&current_slug)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    let slug_owner = sqlx::query_scalar::<_, String>("SELECT id FROM projects WHERE slug = ?")
        .bind(&input.slug)
        .fetch_optional(&mut *tx)
        .await?;
    if slug_owner.as_deref().is_some_and(|owner| owner != id) {
        return Err(AppError::Conflict(format!(
            "项目路径 {} 已存在",
            input.slug
        )));
    }

    update_project(&mut tx, &id, &input).await?;
    replace_children(&mut tx, &id, &input, &identity.sid).await?;
    tx.commit().await?;

    Ok(Json(load_detail(&state, &input.slug).await?))
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

pub async fn import(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Json(request): Json<ProjectImportRequest>,
) -> Result<Json<ProjectImportResponse>, AppError> {
    require_member(&identity)?;
    if request.items.is_empty() {
        return Err(AppError::BadRequest("导入内容不能为空".to_owned()));
    }
    if request.items.len() > 200 {
        return Err(AppError::BadRequest("单次最多导入 200 个项目".to_owned()));
    }

    let mut slugs = HashSet::new();
    let mut items = Vec::with_capacity(request.items.len());
    for item in request.items {
        let item = item.normalized()?;
        if !slugs.insert(item.slug.clone()) {
            return Err(AppError::BadRequest(format!(
                "导入内容中项目路径 {} 重复",
                item.slug
            )));
        }
        items.push(item);
    }

    let mut created = 0;
    let mut updated = 0;
    let mut tx = state.db.begin().await?;
    for input in &items {
        let existing = sqlx::query_scalar::<_, String>("SELECT id FROM projects WHERE slug = ?")
            .bind(&input.slug)
            .fetch_optional(&mut *tx)
            .await?;
        if let Some(id) = existing {
            update_project(&mut tx, &id, input).await?;
            replace_children(&mut tx, &id, input, &identity.sid).await?;
            updated += 1;
        } else {
            let id = Uuid::new_v4().to_string();
            insert_project(&mut tx, &id, input, &identity.sid).await?;
            replace_children(&mut tx, &id, input, &identity.sid).await?;
            created += 1;
        }
    }
    tx.commit().await?;

    Ok(Json(ProjectImportResponse {
        created,
        updated,
        total: items.len(),
    }))
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

        let mut seen_tags = HashSet::new();
        self.tags = self
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_owned())
            .filter(|tag| !tag.is_empty())
            .filter(|tag| seen_tags.insert(tag.to_lowercase()))
            .take(20)
            .collect();

        for resource in &mut self.resources {
            resource.resource_type = resource.resource_type.trim().to_lowercase();
            resource.title = resource.title.trim().to_owned();
            resource.url = normalize_optional(resource.url.take());
            if !RESOURCE_TYPES.contains(&resource.resource_type.as_str()) {
                return Err(AppError::BadRequest(format!(
                    "资源类型 {} 不受支持",
                    resource.resource_type
                )));
            }
            if resource.title.is_empty() {
                return Err(AppError::BadRequest("资源标题不能为空".to_owned()));
            }
        }
        Ok(self)
    }
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
            id, slug, name, summary, primary_category, status, critique, highest_award,
            owner_name, source_name, created_by_sid
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        "UPDATE projects SET
            slug = ?, name = ?, summary = ?, primary_category = ?, status = ?, critique = ?,
            highest_award = ?, owner_name = ?, source_name = ?, updated_at = CURRENT_TIMESTAMP,
            archived_at = NULL
         WHERE id = ?",
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
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn replace_children(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &str,
    input: &ProjectWriteInput,
    actor_sid: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM project_tags WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM resources WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut **tx)
        .await?;

    for (index, tag) in input.tags.iter().enumerate() {
        sqlx::query("INSERT INTO project_tags (project_id, tag, sort_order) VALUES (?, ?, ?)")
            .bind(project_id)
            .bind(tag)
            .bind(index as i64)
            .execute(&mut **tx)
            .await?;
    }

    for resource in &input.resources {
        sqlx::query(
            "INSERT INTO resources (
                id, project_id, resource_type, title, url, availability, created_by_sid
             ) VALUES (?, ?, ?, ?, ?, 'available', ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(project_id)
        .bind(&resource.resource_type)
        .bind(&resource.title)
        .bind(&resource.url)
        .bind(actor_sid)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn load_detail(state: &AppState, slug: &str) -> Result<ProjectDetail, AppError> {
    let row = sqlx::query_as::<_, ProjectDetailRow>(
        "SELECT id, slug, name, summary, primary_category, highest_award, status,
                critique, owner_name, source_name
         FROM projects WHERE slug = ? AND archived_at IS NULL",
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let resources = sqlx::query_as::<_, ProjectResource>(
        "SELECT id, resource_type, title, url FROM resources
         WHERE project_id = ? ORDER BY created_at ASC, title ASC",
    )
    .bind(&row.id)
    .fetch_all(&state.db)
    .await?;
    let tags = sqlx::query_scalar::<_, String>(
        "SELECT tag FROM project_tags WHERE project_id = ? ORDER BY sort_order ASC, tag ASC",
    )
    .bind(&row.id)
    .fetch_all(&state.db)
    .await?;

    Ok(ProjectDetail {
        id: row.id,
        slug: row.slug,
        name: row.name,
        summary: row.summary,
        primary_category: row.primary_category,
        highest_award: row.highest_award,
        status: row.status,
        critique: row.critique,
        owner_name: row.owner_name,
        source_name: row.source_name,
        resources,
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::{ProjectResourceInput, ProjectWriteInput};

    fn valid_input() -> ProjectWriteInput {
        ProjectWriteInput {
            slug: "lab-tool".to_owned(),
            name: "实验室工具".to_owned(),
            summary: "用于测试项目写入。".to_owned(),
            primary_category: "工具项目".to_owned(),
            highest_award: None,
            status: "研发中".to_owned(),
            critique: String::new(),
            owner_name: None,
            source_name: None,
            tags: vec!["软件".to_owned(), " 软件 ".to_owned()],
            resources: vec![ProjectResourceInput {
                resource_type: "github".to_owned(),
                title: "代码".to_owned(),
                url: Some("https://github.com/example/repo".to_owned()),
            }],
        }
    }

    #[test]
    fn normalizes_and_deduplicates_project_input() {
        let input = valid_input().normalized().expect("valid project");
        assert_eq!(input.tags, vec!["软件"]);
        assert_eq!(input.resources[0].resource_type, "github");
    }

    #[test]
    fn rejects_invalid_slug() {
        let mut input = valid_input();
        input.slug = "中文 路径".to_owned();
        assert!(input.normalized().is_err());
    }
}
