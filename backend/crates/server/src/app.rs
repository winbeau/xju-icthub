use axum::{routing::get, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{auth, projects, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/v1/auth/context", get(auth::context))
        .route("/api/v1/projects", get(projects::list))
        .route("/api/v1/projects/{slug}", get(projects::detail))
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
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
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

    #[allow(dead_code)]
    fn _router_is_send(_: Router) {}
}
