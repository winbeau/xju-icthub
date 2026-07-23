use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{auth, covers, imports, projects, state::AppState, tags};

const IMPORT_REQUEST_BODY_LIMIT_BYTES: usize = 512 * 1024 * 1024;
const IMPORT_CHUNK_BODY_LIMIT_BYTES: usize = 4 * 1024 * 1024 + 64 * 1024;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/v1/auth/context", get(auth::context))
        .route(
            "/api/v1/projects",
            get(projects::list).post(projects::create),
        )
        .route("/api/v1/projects/import", post(projects::import))
        .route(
            "/api/v1/import-jobs",
            post(imports::create).layer(DefaultBodyLimit::max(IMPORT_REQUEST_BODY_LIMIT_BYTES)),
        )
        .route("/api/v1/import-jobs/chunked", post(imports::create_chunked))
        .route(
            "/api/v1/import-jobs/{id}/inputs/{input_id}/chunks",
            put(imports::upload_chunk).layer(DefaultBodyLimit::max(IMPORT_CHUNK_BODY_LIMIT_BYTES)),
        )
        .route(
            "/api/v1/import-jobs/{id}/complete",
            post(imports::complete_chunked),
        )
        .route("/api/v1/import-jobs/{id}", get(imports::detail))
        .route("/api/v1/import-jobs/{id}/cancel", post(imports::cancel))
        .route("/api/v1/import-jobs/{id}/refine", post(imports::refine))
        .route("/api/v1/tags", get(tags::list).post(tags::create))
        .route("/api/v1/tags/{id}", axum::routing::patch(tags::update))
        .route("/api/v1/tags/{id}/merge", post(tags::merge))
        .route("/api/v1/tag-suggestions", post(tags::suggest))
        .route(
            "/api/v1/projects/{slug}",
            get(projects::detail)
                .put(projects::update)
                .delete(projects::archive),
        )
        .route(
            "/api/v1/projects/{slug}/cover/generate",
            post(covers::generate),
        )
        .route(
            "/api/v1/projects/{slug}/cover",
            axum::routing::patch(covers::patch),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Write},
        sync::Arc,
        time::Duration,
    };

    use async_trait::async_trait;
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
        routing::get,
        Json, Router,
    };
    use serde_json::{json, Value};
    use tokio::net::TcpListener;
    use tower::ServiceExt;
    use zip::{write::SimpleFileOptions, ZipWriter};

    use super::build_router;
    use crate::{
        imports::{
            agent::{
                AgentImportResult, AgentNormalizedResources, AgentResource, AgentRunOutcome,
                AgentRunRequest, EvidencedValue, ImportAgentRunner,
            },
            process_one_queued_job, ImportWorkerOptions,
        },
        state::AppState,
    };

    #[tokio::test]
    async fn health_endpoint_is_public() {
        let state = AppState::for_test().await.expect("test state");
        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn lab_member_can_create_and_read_project() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state");
        let app = build_router(state.clone());

        let response = app
            .clone()
            .oneshot(project_request("member"))
            .await
            .expect("create response");
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/projects")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("list response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("response body");
        let body: Value = serde_json::from_slice(&bytes).expect("list json");
        assert_eq!(body["total"], 1);
        assert_eq!(body["items"][0]["slug"], "integration-project");
        assert_eq!(body["items"][0]["coverMode"], "text");
        assert!((4..=8).contains(
            &body["items"][0]["coverTitle"]
                .as_str()
                .unwrap()
                .chars()
                .count()
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/projects/integration-project")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("detail response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn non_member_cannot_create_project() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state");
        let response = build_router(state)
            .oneshot(project_request("user"))
            .await
            .expect("create response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn project_read_requires_a_lab_member() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state");
        let app = build_router(state.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/projects")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("anonymous list response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/projects")
                    .header(header::AUTHORIZATION, "Bearer user")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("non-member list response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/projects/any-project")
                    .header(header::AUTHORIZATION, "Bearer user")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("non-member detail response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn import_jobs_require_authentication() {
        let state = AppState::for_test().await.expect("test state");
        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/import-jobs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("import response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn lab_member_can_upload_zip_and_receive_import_preview() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state");
        let app = build_router(state.clone());
        let zip = test_project_zip();
        let boundary = "icthub-test-boundary";
        let mut multipart = Vec::new();
        write!(
            multipart,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"vision.zip\"\r\nContent-Type: application/zip\r\n\r\n"
        )
        .expect("multipart header");
        multipart.extend_from_slice(&zip);
        write!(
            multipart,
            "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"links\"\r\n\r\n[{{\"url\":\"https://github.com/example/source\"}}]\r\n--{boundary}--\r\n"
        )
        .expect("multipart footer");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/import-jobs")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(multipart))
                    .unwrap(),
            )
            .await
            .expect("create import response");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let bytes = to_bytes(response.into_body(), 128 * 1024)
            .await
            .expect("create body");
        let created: Value = serde_json::from_slice(&bytes).expect("create json");
        let id = created["id"].as_str().expect("job id");

        let worker = ImportWorkerOptions::new(50, 30);
        assert!(process_one_queued_job(&state, &worker)
            .await
            .expect("worker processes queued job"));

        let mut completed = None;
        for _ in 0..100 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/v1/import-jobs/{id}"))
                        .header(header::AUTHORIZATION, "Bearer member")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("job detail response");
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = to_bytes(response.into_body(), 512 * 1024)
                .await
                .expect("detail body");
            let detail: Value = serde_json::from_slice(&bytes).expect("detail json");
            if detail["status"] == "completed" {
                completed = Some(detail);
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let detail = completed.expect("import job completed");
        assert_eq!(
            detail["result"]["projectDraft"]["primaryCategory"],
            "AI 软件"
        );
        assert_eq!(detail["inputs"][1]["provider"], "github");
        assert_eq!(detail["inputs"][1]["status"], "pending_parser");
        assert!(detail["artifacts"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        let artifacts = detail["artifacts"].as_array().expect("artifact list");
        assert!(artifacts.iter().any(|artifact| {
            artifact["relativePath"] == "vision/docs/说明.docx"
                && artifact["extractor"] == "docx_text"
                && artifact["metadata"]["paragraphCount"] == 1
        }));
        assert!(artifacts.iter().any(|artifact| {
            artifact["relativePath"] == "vision/demo.pptx"
                && artifact["extractor"] == "pptx_text"
                && artifact["metadata"]["slideCount"] == 1
        }));
        assert!(detail["events"]
            .as_array()
            .is_some_and(|items| items.iter().any(|event| event["eventType"] == "completed")));
        assert_eq!(
            detail["analysisBundlePath"],
            "analysis/analysis-bundle.json"
        );
        let bundle_path = state
            .import_root
            .join(id)
            .join("analysis/analysis-bundle.json");
        let bundle: Value =
            serde_json::from_slice(&std::fs::read(bundle_path).expect("analysis bundle file"))
                .expect("analysis bundle JSON");
        assert_eq!(bundle["schemaVersion"], "1.0");
        assert!(bundle["artifacts"].as_array().is_some_and(|artifacts| {
            artifacts.iter().any(|artifact| {
                artifact["relativePath"] == "vision/docs/说明.docx"
                    && artifact["textExcerpt"]
                        .as_str()
                        .is_some_and(|text| text.contains("视觉项目说明"))
            })
        }));
    }

    #[tokio::test]
    async fn lab_member_can_upload_file_in_retryable_chunks() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state");
        let app = build_router(state.clone());
        let contents = b"project name: chunked upload\nReact Web utility";

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/import-jobs/chunked")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "files": [{
                                "name": "README.md",
                                "sizeBytes": contents.len(),
                                "mimeType": "text/markdown"
                            }],
                            "links": [],
                            "prompt": "这是一个 Web 日常工具"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("chunked init response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = to_bytes(response.into_body(), 128 * 1024)
            .await
            .expect("chunked init body");
        let initialized: Value = serde_json::from_slice(&bytes).expect("chunked init json");
        let job_id = initialized["job"]["id"].as_str().expect("chunked job id");
        let input_id = initialized["job"]["inputs"][0]["id"]
            .as_str()
            .expect("chunked input id");
        assert_eq!(initialized["job"]["status"], "uploading");

        let split = 17;
        for (offset, chunk) in [(0, &contents[..split]), (split, &contents[split..])] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(format!(
                            "/api/v1/import-jobs/{job_id}/inputs/{input_id}/chunks"
                        ))
                        .header(header::AUTHORIZATION, "Bearer member")
                        .header(header::CONTENT_TYPE, "application/octet-stream")
                        .header("x-upload-offset", offset.to_string())
                        .body(Body::from(chunk.to_vec()))
                        .unwrap(),
                )
                .await
                .expect("chunk response");
            assert_eq!(response.status(), StatusCode::OK);
        }

        let duplicate = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!(
                        "/api/v1/import-jobs/{job_id}/inputs/{input_id}/chunks"
                    ))
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .header("x-upload-offset", split.to_string())
                    .body(Body::from(contents[split..].to_vec()))
                    .unwrap(),
            )
            .await
            .expect("duplicate chunk response");
        assert_eq!(duplicate.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/import-jobs/{job_id}/complete"))
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .expect("chunked complete response");
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let worker = ImportWorkerOptions::new(50, 30);
        assert!(process_one_queued_job(&state, &worker)
            .await
            .expect("worker processes chunked job"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/import-jobs/{job_id}"))
                    .header(header::AUTHORIZATION, "Bearer member")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("chunked detail response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 256 * 1024)
            .await
            .expect("chunked detail body");
        let detail: Value = serde_json::from_slice(&bytes).expect("chunked detail json");
        assert_eq!(detail["status"], "completed");
        assert_eq!(detail["inputs"][0]["status"], "parsed");
    }

    #[tokio::test]
    async fn configured_agent_runs_automatically_after_local_import() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state")
            .with_import_agent(Arc::new(FakeAgentRunner));
        let app = build_router(state.clone());
        let zip = test_project_zip();
        let boundary = "icthub-auto-agent-boundary";
        let mut multipart = Vec::new();
        write!(
            multipart,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"auto-agent.zip\"\r\nContent-Type: application/zip\r\n\r\n"
        )
        .expect("multipart header");
        multipart.extend_from_slice(&zip);
        write!(
            multipart,
            "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"prompt\"\r\n\r\n负责人：张三\r\n--{boundary}--\r\n"
        )
        .expect("multipart footer");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/import-jobs")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(multipart))
                    .unwrap(),
            )
            .await
            .expect("create automatic agent job");
        let created: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 128 * 1024)
                .await
                .expect("created body"),
        )
        .expect("created JSON");
        let job_id = created["id"].as_str().expect("job id");
        let worker = ImportWorkerOptions::new(50, 30);

        assert!(process_one_queued_job(&state, &worker)
            .await
            .expect("local import worker"));
        let queued = sqlx::query_scalar::<_, String>("SELECT status FROM import_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_one(&state.db)
            .await
            .expect("queued agent status");
        assert_eq!(queued, "agent_queued");

        assert!(process_one_queued_job(&state, &worker)
            .await
            .expect("automatic agent worker"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/import-jobs/{job_id}"))
                    .header(header::AUTHORIZATION, "Bearer member")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("automatic agent detail");
        let detail: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 256 * 1024)
                .await
                .expect("detail body"),
        )
        .expect("detail JSON");
        assert_eq!(detail["status"], "completed");
        assert_eq!(detail["analysisEngine"], "codex_exec");
        assert_eq!(detail["agentRuns"][0]["status"], "completed");
    }

    #[tokio::test]
    async fn import_worker_recovers_an_expired_lease() {
        let state = AppState::for_test().await.expect("test state");
        let job_id = "expired-import-job";
        let job_dir = state.import_root.join(job_id);
        std::fs::create_dir_all(job_dir.join("source/input"))
            .expect("expired job source directory");
        std::fs::write(
            job_dir.join("source/input/README.md"),
            "项目名：恢复测试\n这是一个 Web 日常工具。",
        )
        .expect("expired job input");
        sqlx::query(
            "INSERT INTO import_jobs (
                id, status, stage, progress, source_kind, source_name, created_by_sid,
                worker_id, lease_expires_at, attempt_count
             ) VALUES (?, 'extracting', '旧 Worker 已退出', 18, 'mixed', '恢复测试',
                '20211010001', 'dead-worker', datetime('now', '-5 minutes'), 1)",
        )
        .bind(job_id)
        .execute(&state.db)
        .await
        .expect("expired job");
        sqlx::query(
            "INSERT INTO import_inputs (
                id, job_id, input_kind, provider, display_name, local_path, mime_type,
                size_bytes, sha256, sort_order, status
             ) VALUES ('expired-input', ?, 'file', 'upload', 'README.md',
                'source/input/README.md', 'text/markdown', 47, 'test', 0, 'queued')",
        )
        .bind(job_id)
        .execute(&state.db)
        .await
        .expect("expired input");

        let worker = ImportWorkerOptions::new(50, 30);
        assert!(process_one_queued_job(&state, &worker)
            .await
            .expect("recovered worker job"));
        let recovered = sqlx::query_as::<_, (String, i64, Option<String>)>(
            "SELECT status, attempt_count, worker_id FROM import_jobs WHERE id = ?",
        )
        .bind(job_id)
        .fetch_one(&state.db)
        .await
        .expect("recovered job status");
        assert_eq!(recovered.0, "completed");
        assert_eq!(recovered.1, 2);
        assert!(recovered.2.is_none());
    }

    #[tokio::test]
    async fn import_job_can_be_cancelled_and_refinement_prompt_is_saved() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state");
        sqlx::query(
            "INSERT INTO import_jobs (
                id, status, stage, progress, source_kind, source_name, created_by_sid
             ) VALUES ('cancel-job', 'extracting', '正在安全整理附件', 18, 'mixed',
                '测试附件', '20211010001')",
        )
        .execute(&state.db)
        .await
        .expect("cancel job");
        sqlx::query(
            "INSERT INTO import_jobs (
                id, status, stage, progress, source_kind, source_name, created_by_sid
             ) VALUES ('refine-job', 'completed', '等待确认', 100, 'prompt',
                '项目简介', '20211010001')",
        )
        .execute(&state.db)
        .await
        .expect("refine job");
        let app = build_router(state.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/import-jobs/cancel-job/cancel")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("cancel response");
        assert_eq!(response.status(), StatusCode::OK);
        let status = sqlx::query_scalar::<_, String>(
            "SELECT status FROM import_jobs WHERE id = 'cancel-job'",
        )
        .fetch_one(&state.db)
        .await
        .expect("cancelled status");
        assert_eq!(status, "cancelled");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/import-jobs/refine-job/refine")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "prompt": "负责人：张三\n来源：课程项目" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("refine response");
        assert_eq!(response.status(), StatusCode::OK);
        let saved_prompt = sqlx::query_scalar::<_, String>(
            "SELECT source_ref FROM import_inputs WHERE job_id = 'refine-job'
                AND display_name = '整理补充提示'",
        )
        .fetch_one(&state.db)
        .await
        .expect("saved refinement");
        assert_eq!(saved_prompt, "负责人：张三\n来源：课程项目");
    }

    #[tokio::test]
    async fn configured_agent_processes_refinement_and_persists_structured_result() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state")
            .with_import_agent(Arc::new(FakeAgentRunner));
        let job_id = "agent-refine-job";
        let analysis_dir = state.import_root.join(job_id).join("analysis");
        std::fs::create_dir_all(&analysis_dir).expect("analysis directory");
        std::fs::write(
            analysis_dir.join("analysis-bundle.json"),
            br#"{"schemaVersion":"1.0","artifacts":[]}"#,
        )
        .expect("analysis bundle");
        let fallback = json!({
            "projectDraft": {
                "name": "原始草稿", "slug": "original-draft", "summary": "本地整理草稿。",
                "primaryCategory": "传统软件", "suggestedTags": [], "ownerName": null,
                "sourceName": null, "highestAward": null, "status": "待确认"
            },
            "artifactSummary": [],
            "normalizedResources": {
                "sourceCode": [], "documents": [], "presentations": [], "videos": [], "links": []
            },
            "warnings": [],
            "agent": {
                "status": "awaiting_configuration", "mode": "deterministic_fallback",
                "message": "等待配置"
            },
            "capabilities": {
                "zipUpload": "prototype_ready", "githubLink": "input_reserved",
                "mixedFiles": "prototype_ready", "codexAgent": "awaiting_configuration",
                "githubPublish": "awaiting_credentials"
            }
        });
        sqlx::query(
            "INSERT INTO import_jobs (
                id, status, stage, progress, source_kind, source_name, created_by_sid,
                result_json, analysis_bundle_path, completed_at
             ) VALUES (?, 'completed', '等待确认', 100, 'zip', 'agent.zip', '20211010001',
                ?, 'analysis/analysis-bundle.json', CURRENT_TIMESTAMP)",
        )
        .bind(job_id)
        .bind(fallback.to_string())
        .execute(&state.db)
        .await
        .expect("agent job");
        let app = build_router(state.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/import-jobs/{job_id}/refine"))
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "prompt": "负责人：张三\n标签：计算机视觉" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("refine response");
        assert_eq!(response.status(), StatusCode::OK);
        let queued: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 128 * 1024)
                .await
                .expect("queued body"),
        )
        .expect("queued JSON");
        assert_eq!(queued["status"], "agent_queued");

        sqlx::query(
            "UPDATE import_jobs SET status = 'agent_running', worker_id = 'dead-agent-worker',
                lease_expires_at = datetime('now', '-5 minutes') WHERE id = ?",
        )
        .bind(job_id)
        .execute(&state.db)
        .await
        .expect("simulate expired agent lease");

        let worker = ImportWorkerOptions::new(50, 30);
        assert!(process_one_queued_job(&state, &worker)
            .await
            .expect("agent worker"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/import-jobs/{job_id}"))
                    .header(header::AUTHORIZATION, "Bearer member")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("agent detail");
        let detail: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 256 * 1024)
                .await
                .expect("detail body"),
        )
        .expect("detail JSON");
        assert_eq!(detail["status"], "completed");
        assert_eq!(detail["analysisEngine"], "codex_exec");
        assert_eq!(detail["agentThreadId"], "thread-fake-1");
        assert_eq!(detail["result"]["projectDraft"]["name"], "棉田智检");
        assert_eq!(
            detail["result"]["projectDraft"]["suggestedTags"][0],
            "计算机视觉"
        );
        assert_eq!(detail["agentRuns"][0]["status"], "completed");
        assert_eq!(
            detail["result"]["normalizedResources"]["documents"][0]["sourceRef"],
            "docs/项目说明.pdf"
        );
    }

    struct FakeAgentRunner;

    #[async_trait]
    impl ImportAgentRunner for FakeAgentRunner {
        fn enabled(&self) -> bool {
            true
        }

        fn runner_name(&self) -> &'static str {
            "fake_codex_exec"
        }

        fn model_name(&self) -> Option<&str> {
            Some("fake-model")
        }

        fn base_url_origin(&self) -> Option<String> {
            Some("https://api.example.test".to_owned())
        }

        async fn run(&self, request: AgentRunRequest) -> anyhow::Result<AgentRunOutcome> {
            let raw_events_path = request
                .analysis_dir
                .join(format!("agent-events-{}.jsonl", request.run_id));
            tokio::fs::write(
                &raw_events_path,
                b"{\"type\":\"thread.started\",\"thread_id\":\"thread-fake-1\"}\n",
            )
            .await?;
            Ok(AgentRunOutcome {
                thread_id: Some("thread-fake-1".to_owned()),
                raw_events_path,
                result: AgentImportResult {
                    project_name: "棉田智检".to_owned(),
                    summary: "面向棉花病虫害识别的移动巡检平台。".to_owned(),
                    primary_category: "AI 软件".to_owned(),
                    suggested_tags: vec![EvidencedValue {
                        value: "计算机视觉".to_owned(),
                        evidence: "成员补充提示明确写出标签".to_owned(),
                    }],
                    owner: Some(EvidencedValue {
                        value: "张三".to_owned(),
                        evidence: "成员补充提示明确写出负责人".to_owned(),
                    }),
                    source: None,
                    highest_award: None,
                    resources: AgentNormalizedResources {
                        source_code: Vec::new(),
                        documents: vec![AgentResource {
                            display_name: "项目说明".to_owned(),
                            source_ref: "docs/项目说明.pdf".to_owned(),
                            evidence: "分析包列出该 PDF".to_owned(),
                            confidence: 1.0,
                        }],
                        presentations: Vec::new(),
                        videos: Vec::new(),
                        links: Vec::new(),
                    },
                    warnings: Vec::new(),
                },
            })
        }
    }

    #[tokio::test]
    async fn initial_competition_tags_exist_and_member_cannot_create_formal_tag() {
        let identity_url = spawn_identity_service().await;
        let state = AppState::for_test_with_identity_url(&identity_url)
            .await
            .expect("test state");
        let competition_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tag_definitions WHERE name IN (
                '国创赛（互联网+）', '计算机设计大赛', '智能应用技术大赛'
             ) AND is_active = 1",
        )
        .fetch_one(&state.db)
        .await
        .expect("seeded tags");
        assert_eq!(competition_count, 3);

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tags")
                    .header(header::AUTHORIZATION, "Bearer member")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "name": "成员私建标签",
                            "groupName": "技术",
                            "color": null,
                            "sortOrder": 1
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("tag create response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    fn project_request(token: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/v1/projects")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "slug": "integration-project",
                    "name": "集成测试项目",
                    "summary": "验证受保护的项目创建接口。",
                    "primaryCategory": "传统软件",
                    "highestAward": null,
                    "status": "研发中",
                    "critique": "测试写入链路。",
                    "ownerName": "测试组",
                    "sourceName": "自动化测试",
                    "tags": ["Web"],
                    "resources": []
                })
                .to_string(),
            ))
            .unwrap()
    }

    async fn spawn_identity_service() -> String {
        let app = Router::new().route(
            "/auth/me",
            get(|headers: axum::http::HeaderMap| async move {
                let token = headers
                    .get(header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("");
                let member = token == "Bearer member" || token == "Bearer admin";
                let admin = token == "Bearer admin";
                Json(json!({
                    "sid": if admin { "20211019999" } else if member { "20211010001" } else { "20211010000" },
                    "name": "测试用户",
                    "nickname": "测试用户",
                    "role": if admin { "admin" } else { "user" },
                    "isAdmin": admin,
                    "isSuperAdmin": false,
                    "isLabMember": member
                }))
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("identity listener");
        let address = listener.local_addr().expect("identity address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("identity server");
        });
        format!("http://{address}")
    }

    fn test_project_zip() -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default();
        writer
            .start_file("vision/README.md", options)
            .expect("readme entry");
        writer
            .write_all("基于 YOLO 和 OpenCV 的视觉识别项目".as_bytes())
            .expect("readme");
        writer
            .start_file("vision/src/main.py", options)
            .expect("source entry");
        writer.write_all(b"print('ok')").expect("source");
        writer
            .start_file("vision/docs/说明.docx", options)
            .expect("DOCX entry");
        writer.write_all(&test_docx()).expect("DOCX bytes");
        writer
            .start_file("vision/demo.pptx", options)
            .expect("PPTX entry");
        writer.write_all(&test_pptx()).expect("PPTX bytes");
        writer.finish().expect("finish zip").into_inner()
    }

    fn test_docx() -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file("word/document.xml", SimpleFileOptions::default())
            .expect("DOCX XML entry");
        writer
            .write_all(
                r#"<w:document xmlns:w="urn:w"><w:body><w:p><w:r><w:t>视觉项目说明</w:t></w:r></w:p></w:body></w:document>"#
                    .as_bytes(),
            )
            .expect("DOCX XML");
        writer.finish().expect("finish DOCX").into_inner()
    }

    fn test_pptx() -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file("ppt/slides/slide1.xml", SimpleFileOptions::default())
            .expect("PPTX slide entry");
        writer
            .write_all(
                r#"<p:sld xmlns:p="urn:p" xmlns:a="urn:a"><p:cSld><a:p><a:r><a:t>项目展示首页</a:t></a:r></a:p></p:cSld></p:sld>"#
                    .as_bytes(),
            )
            .expect("PPTX slide XML");
        writer.finish().expect("finish PPTX").into_inner()
    }

    #[allow(dead_code)]
    fn _router_is_send(_: Router) {}
}
