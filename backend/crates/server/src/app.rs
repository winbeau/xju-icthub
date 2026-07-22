use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{auth, projects, state::AppState};

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
            "/api/v1/projects/{slug}",
            get(projects::detail)
                .put(projects::update)
                .delete(projects::archive),
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
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
        routing::get,
        Json, Router,
    };
    use serde_json::{json, Value};
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    use super::build_router;
    use crate::state::AppState;

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
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(project_request("member"))
            .await
            .expect("create response");
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
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
        let app = build_router(state);

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
                    "primaryCategory": "工具项目",
                    "highestAward": null,
                    "status": "研发中",
                    "critique": "测试写入链路。",
                    "ownerName": "测试组",
                    "sourceName": "自动化测试",
                    "tags": ["软件"],
                    "resources": [{
                        "type": "github",
                        "title": "代码仓库",
                        "url": "https://github.com/example/repo"
                    }]
                })
                .to_string(),
            ))
            .unwrap()
    }

    async fn spawn_identity_service() -> String {
        let app = Router::new().route(
            "/auth/me",
            get(|headers: axum::http::HeaderMap| async move {
                let member = headers
                    .get(header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    == Some("Bearer member");
                Json(json!({
                    "sid": if member { "20211010001" } else { "20211010000" },
                    "name": "测试用户",
                    "nickname": "测试用户",
                    "role": "user",
                    "isAdmin": false,
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

    #[allow(dead_code)]
    fn _router_is_send(_: Router) {}
}
