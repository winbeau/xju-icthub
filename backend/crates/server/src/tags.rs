use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, FeiyueIdentity},
    error::AppError,
    state::AppState,
};

#[derive(Clone, Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TagDefinition {
    pub id: String,
    pub name: String,
    pub group_name: String,
    pub color: Option<String>,
    pub sort_order: i64,
    pub is_active: bool,
    pub merged_into_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCreateInput {
    pub name: String,
    pub group_name: String,
    pub color: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagUpdateInput {
    pub name: Option<String>,
    pub group_name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagMergeInput {
    pub target_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSuggestionInput {
    pub name: String,
    pub group_name: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSuggestionResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagListQuery {
    #[serde(default)]
    include_inactive: bool,
}

pub async fn list(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Query(query): Query<TagListQuery>,
) -> Result<Json<Vec<TagDefinition>>, AppError> {
    require_member(&identity)?;
    let include_inactive = query.include_inactive && identity.can_manage_tags();
    let tags = sqlx::query_as::<_, TagDefinition>(
        "SELECT id, name, group_name, color, sort_order, is_active, merged_into_id
         FROM tag_definitions WHERE is_active = 1 OR ? = 1
         ORDER BY group_name ASC, sort_order ASC, name ASC",
    )
    .bind(include_inactive)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(tags))
}

pub async fn create(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Json(input): Json<TagCreateInput>,
) -> Result<(StatusCode, Json<TagDefinition>), AppError> {
    require_tag_admin(&identity)?;
    let name = required(&input.name, "标签名称不能为空")?;
    let group_name = required(&input.group_name, "标签分组不能为空")?;
    let id = Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO tag_definitions (id, name, group_name, color, sort_order, created_by_sid)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&group_name)
    .bind(
        input
            .color
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(input.sort_order)
    .bind(&identity.sid)
    .execute(&state.db)
    .await;
    if let Err(sqlx::Error::Database(error)) = &result {
        if error.is_unique_violation() {
            return Err(AppError::Conflict("标签名称已存在".to_owned()));
        }
    }
    result?;
    Ok((StatusCode::CREATED, Json(load_tag(&state, &id).await?)))
}

pub async fn update(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(id): Path<String>,
    Json(input): Json<TagUpdateInput>,
) -> Result<Json<TagDefinition>, AppError> {
    require_tag_admin(&identity)?;
    let current = load_tag(&state, &id).await?;
    let name = input
        .name
        .as_deref()
        .map(|value| required(value, "标签名称不能为空"))
        .transpose()?
        .unwrap_or(current.name);
    let group_name = input
        .group_name
        .as_deref()
        .map(|value| required(value, "标签分组不能为空"))
        .transpose()?
        .unwrap_or(current.group_name);
    let color = input.color.or(current.color);
    let sort_order = input.sort_order.unwrap_or(current.sort_order);
    let is_active = input.is_active.unwrap_or(current.is_active);
    sqlx::query(
        "UPDATE tag_definitions SET name = ?, group_name = ?, color = ?, sort_order = ?,
            is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&name)
    .bind(&group_name)
    .bind(&color)
    .bind(sort_order)
    .bind(is_active)
    .bind(&id)
    .execute(&state.db)
    .await?;
    sqlx::query("UPDATE project_tags SET tag = ? WHERE tag_definition_id = ?")
        .bind(&name)
        .bind(&id)
        .execute(&state.db)
        .await?;
    Ok(Json(load_tag(&state, &id).await?))
}

pub async fn merge(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(id): Path<String>,
    Json(input): Json<TagMergeInput>,
) -> Result<StatusCode, AppError> {
    require_tag_admin(&identity)?;
    if id == input.target_id {
        return Err(AppError::BadRequest("不能合并到自身".to_owned()));
    }
    let source = load_tag(&state, &id).await?;
    let target = load_tag(&state, &input.target_id).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT OR IGNORE INTO project_tags (project_id, tag, sort_order, tag_definition_id)
         SELECT project_id, ?, sort_order, ? FROM project_tags WHERE tag_definition_id = ?",
    )
    .bind(&target.name)
    .bind(&target.id)
    .bind(&source.id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM project_tags WHERE tag_definition_id = ?")
        .bind(&source.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE tag_definitions SET is_active = 0, merged_into_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&target.id)
    .bind(&source.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn suggest(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Json(input): Json<TagSuggestionInput>,
) -> Result<(StatusCode, Json<TagSuggestionResponse>), AppError> {
    require_member(&identity)?;
    let name = required(&input.name, "建议标签名称不能为空")?;
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tag_suggestions (id, name, group_name, reason, created_by_sid)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(
        input
            .group_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(
        input
            .reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    )
    .bind(&identity.sid)
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(TagSuggestionResponse {
            id,
            status: "pending".to_owned(),
        }),
    ))
}

async fn load_tag(state: &AppState, id: &str) -> Result<TagDefinition, AppError> {
    sqlx::query_as::<_, TagDefinition>(
        "SELECT id, name, group_name, color, sort_order, is_active, merged_into_id
         FROM tag_definitions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)
}

fn required(value: &str, message: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 40 {
        Err(AppError::BadRequest(message.to_owned()))
    } else {
        Ok(value.to_owned())
    }
}

fn require_member(identity: &FeiyueIdentity) -> Result<(), AppError> {
    if identity.can_access_icthub() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn require_tag_admin(identity: &FeiyueIdentity) -> Result<(), AppError> {
    if identity.can_manage_tags() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
