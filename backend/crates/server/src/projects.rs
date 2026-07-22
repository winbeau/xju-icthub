use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{error::AppError, state::AppState};

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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub items: Vec<ProjectSummary>,
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
    Query(query): Query<ProjectListQuery>,
) -> Result<Json<ProjectListResponse>, AppError> {
    let mut items = sqlx::query_as::<_, ProjectSummary>(
        "SELECT id, slug, name, summary, primary_category, highest_award
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
        .filter(|v| !v.is_empty())
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
    Path(slug): Path<String>,
) -> Result<Json<ProjectDetail>, AppError> {
    let row = sqlx::query_as::<_, ProjectDetailRow>(
        "SELECT id, slug, name, summary, primary_category, highest_award, status,
                critique, NULL as owner_name, source_name
         FROM projects WHERE slug = ? AND archived_at IS NULL",
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let mut project = ProjectDetail {
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
        resources: Vec::new(),
        tags: Vec::new(),
    };

    project.resources = sqlx::query_as::<_, ProjectResource>(
        "SELECT id, resource_type, title, url FROM resources WHERE project_id = ? ORDER BY created_at ASC",
    )
    .bind(&project.id)
    .fetch_all(&state.db)
    .await?;
    project.tags = Vec::new();
    Ok(Json(project))
}
