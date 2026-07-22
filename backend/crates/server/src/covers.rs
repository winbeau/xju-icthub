use axum::{
    extract::{Path, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    auth::{AuthContext, FeiyueIdentity},
    error::AppError,
    state::AppState,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCover {
    pub cover_mode: String,
    pub cover_resource_id: Option<String>,
    pub cover_resource_url: Option<String>,
    pub cover_title: String,
    pub cover_subtitle: String,
    pub cover_keywords: Vec<String>,
    pub cover_tone: String,
    pub confidence: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverPatch {
    pub cover_mode: String,
    pub cover_resource_id: Option<String>,
    pub cover_title: String,
    pub cover_subtitle: String,
    #[serde(default)]
    pub cover_keywords: Vec<String>,
    pub cover_tone: String,
}

#[derive(Debug, FromRow)]
struct CoverProjectRow {
    id: String,
    name: String,
    summary: String,
    primary_category: String,
    cover_mode: String,
    cover_resource_id: Option<String>,
    cover_title: Option<String>,
    cover_subtitle: Option<String>,
    cover_keywords: String,
    cover_tone: String,
    cover_confidence: Option<f64>,
}

#[derive(Clone, Debug, FromRow)]
struct CoverResourceRow {
    id: String,
    resource_type: String,
    url: Option<String>,
}

pub async fn generate(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(slug): Path<String>,
) -> Result<Json<GeneratedCover>, AppError> {
    require_member(&identity)?;
    Ok(Json(generate_for_slug(&state, &slug, true).await?))
}

pub async fn patch(
    State(state): State<AppState>,
    AuthContext(identity): AuthContext,
    Path(slug): Path<String>,
    Json(input): Json<CoverPatch>,
) -> Result<Json<GeneratedCover>, AppError> {
    require_member(&identity)?;
    let mode = input.cover_mode.trim().to_lowercase();
    if !["manual", "resource", "text"].contains(&mode.as_str()) {
        return Err(AppError::BadRequest("封面模式不受支持".to_owned()));
    }
    let title = input.cover_title.trim().to_owned();
    let subtitle = input.cover_subtitle.trim().to_owned();
    if title.is_empty() || title.chars().count() > 16 || subtitle.chars().count() > 40 {
        return Err(AppError::BadRequest(
            "请检查封面标题与副标题长度".to_owned(),
        ));
    }
    let keywords = normalize_keywords(input.cover_keywords);
    let resource_url = if let Some(resource_id) = input.cover_resource_id.as_deref() {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT url FROM resources WHERE id = ? AND project_id = (
                SELECT id FROM projects WHERE slug = ? AND archived_at IS NULL
             )",
        )
        .bind(resource_id)
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("封面资源不属于该项目".to_owned()))?
    } else {
        None
    };
    if mode == "resource" && input.cover_resource_id.is_none() {
        return Err(AppError::BadRequest("资源封面必须指定项目资源".to_owned()));
    }

    let result = sqlx::query(
        "UPDATE projects SET cover_mode = ?, cover_resource_id = ?, cover_title = ?,
            cover_subtitle = ?, cover_keywords = ?, cover_tone = ?, cover_confidence = 1.0,
            cover_generated_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE slug = ? AND archived_at IS NULL",
    )
    .bind(&mode)
    .bind(&input.cover_resource_id)
    .bind(&title)
    .bind(&subtitle)
    .bind(serde_json::to_string(&keywords).unwrap_or_else(|_| "[]".to_owned()))
    .bind(input.cover_tone.trim())
    .bind(&slug)
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(GeneratedCover {
        cover_mode: mode,
        cover_resource_id: input.cover_resource_id,
        cover_resource_url: resource_url,
        cover_title: title,
        cover_subtitle: subtitle,
        cover_keywords: keywords,
        cover_tone: input.cover_tone.trim().to_owned(),
        confidence: 1.0,
    }))
}

pub async fn generate_for_slug(
    state: &AppState,
    slug: &str,
    preserve_manual: bool,
) -> Result<GeneratedCover, AppError> {
    let project = sqlx::query_as::<_, CoverProjectRow>(
        "SELECT id, name, summary, primary_category, cover_mode, cover_resource_id,
                cover_title, cover_subtitle, cover_keywords, cover_tone, cover_confidence
         FROM projects WHERE slug = ? AND archived_at IS NULL",
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if preserve_manual && project.cover_mode == "manual" {
        let resource_url = resource_url(state, project.cover_resource_id.as_deref()).await?;
        return Ok(GeneratedCover {
            cover_mode: project.cover_mode,
            cover_resource_id: project.cover_resource_id,
            cover_resource_url: resource_url,
            cover_title: project
                .cover_title
                .unwrap_or_else(|| fallback_title(&project.name, &project.primary_category)),
            cover_subtitle: project
                .cover_subtitle
                .unwrap_or_else(|| truncate(&project.summary, 20)),
            cover_keywords: parse_keywords(&project.cover_keywords),
            cover_tone: project.cover_tone,
            confidence: project.cover_confidence.unwrap_or(1.0),
        });
    }

    let resources = sqlx::query_as::<_, CoverResourceRow>(
        "SELECT id, resource_type, url FROM resources WHERE project_id = ? ORDER BY created_at ASC",
    )
    .bind(&project.id)
    .fetch_all(&state.db)
    .await?;
    let tags = sqlx::query_scalar::<_, String>(
        "SELECT tag FROM project_tags WHERE project_id = ? ORDER BY sort_order ASC, tag ASC LIMIT 3",
    )
    .bind(&project.id)
    .fetch_all(&state.db)
    .await?;
    let candidate = select_resource(&resources);
    let cover_mode = if candidate.is_some() {
        "resource"
    } else {
        "text"
    }
    .to_owned();
    let cover_resource_id = candidate.map(|resource| resource.id.clone());
    let cover_resource_url = candidate.and_then(|resource| resource.url.clone());
    let cover_title = fallback_title(&project.name, &project.primary_category);
    let cover_subtitle = truncate(&project.summary, 20);
    let cover_keywords = if tags.is_empty() {
        vec![project.primary_category.clone()]
    } else {
        tags
    };
    let cover_tone = tone_for_category(&project.primary_category).to_owned();
    let confidence = if candidate.is_some() { 0.92 } else { 0.74 };

    sqlx::query(
        "UPDATE projects SET cover_mode = ?, cover_resource_id = ?, cover_title = ?,
            cover_subtitle = ?, cover_keywords = ?, cover_tone = ?, cover_confidence = ?,
            cover_generated_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&cover_mode)
    .bind(&cover_resource_id)
    .bind(&cover_title)
    .bind(&cover_subtitle)
    .bind(serde_json::to_string(&cover_keywords).unwrap_or_else(|_| "[]".to_owned()))
    .bind(&cover_tone)
    .bind(confidence)
    .bind(&project.id)
    .execute(&state.db)
    .await?;

    Ok(GeneratedCover {
        cover_mode,
        cover_resource_id,
        cover_resource_url,
        cover_title,
        cover_subtitle,
        cover_keywords,
        cover_tone,
        confidence,
    })
}

fn select_resource(resources: &[CoverResourceRow]) -> Option<&CoverResourceRow> {
    resources
        .iter()
        .filter(|resource| resource_priority(resource) < 20)
        .min_by_key(|resource| resource_priority(resource))
}

fn resource_priority(resource: &CoverResourceRow) -> u8 {
    let url = resource.url.as_deref().unwrap_or("").to_lowercase();
    if resource.resource_type == "image" || is_image_url(&url) {
        1
    } else if resource.resource_type == "video" {
        2
    } else if resource.resource_type == "presentation" || url.ends_with(".pptx") {
        3
    } else if resource.resource_type == "github" {
        4
    } else if resource.resource_type == "document" {
        5
    } else {
        20
    }
}

fn is_image_url(url: &str) -> bool {
    [".png", ".jpg", ".jpeg", ".webp", ".gif", ".avif"]
        .iter()
        .any(|extension| {
            url.split(['?', '#'])
                .next()
                .unwrap_or(url)
                .ends_with(extension)
        })
}

fn fallback_title(name: &str, category: &str) -> String {
    let curated = [
        ("棉花", "棉田智检"),
        ("病虫害", "病害智检"),
        ("机器人", "智能巡检"),
        ("会议纪要", "智能纪要"),
        ("预约", "便捷预约"),
        ("归档", "实验归档"),
        ("预测", "智能预测"),
    ];
    if let Some((_, title)) = curated.iter().find(|(keyword, _)| name.contains(keyword)) {
        return (*title).to_owned();
    }
    let cleaned = name
        .replace("基于", "")
        .replace("面向", "")
        .replace("平台", "")
        .replace("系统", "")
        .replace("项目", "");
    let chinese: String = cleaned
        .chars()
        .filter(|character| ('\u{4e00}'..='\u{9fff}').contains(character))
        .take(8)
        .collect();
    if chinese.chars().count() >= 4 {
        chinese
    } else {
        match category {
            "智能硬件" => "智能硬件",
            "AI 软件" => "智能应用",
            "数字媒体" => "数字创意",
            "研究成果" => "研究成果",
            _ => "软件项目",
        }
        .to_owned()
    }
}

fn tone_for_category(category: &str) -> &'static str {
    match category {
        "智能硬件" => "amber",
        "AI 软件" => "violet",
        "数字媒体" => "cyan",
        "研究成果" => "emerald",
        _ => "slate",
    }
}

fn truncate(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn normalize_keywords(keywords: Vec<String>) -> Vec<String> {
    keywords
        .into_iter()
        .map(|keyword| keyword.trim().to_owned())
        .filter(|keyword| !keyword.is_empty())
        .take(3)
        .collect()
}

fn parse_keywords(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

async fn resource_url(state: &AppState, id: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(id) = id else { return Ok(None) };
    Ok(
        sqlx::query_scalar::<_, Option<String>>("SELECT url FROM resources WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .flatten(),
    )
}

fn require_member(identity: &FeiyueIdentity) -> Result<(), AppError> {
    if identity.can_access_icthub() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::{fallback_title, resource_priority, select_resource, CoverResourceRow};

    #[test]
    fn text_fallback_has_agent_title_length() {
        let title = fallback_title("实验室服务门户", "传统软件");
        assert!((4..=8).contains(&title.chars().count()));
    }

    #[test]
    fn image_candidate_precedes_other_resources() {
        let resources = vec![
            CoverResourceRow {
                id: "video".into(),
                resource_type: "video".into(),
                url: Some("https://example.com/demo.mp4".into()),
            },
            CoverResourceRow {
                id: "image".into(),
                resource_type: "image".into(),
                url: Some("https://example.com/cover.png".into()),
            },
        ];
        assert_eq!(
            select_resource(&resources).map(|item| item.id.as_str()),
            Some("image")
        );
        assert!(resource_priority(&resources[1]) < resource_priority(&resources[0]));
    }
}
