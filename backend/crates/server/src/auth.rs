use std::future::Future;

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
    response::Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{error::AppError, state::AppState};

#[derive(Clone)]
pub struct FeiyueIdentityClient {
    client: Client,
    base_url: String,
}

impl FeiyueIdentityClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    pub async fn me(&self, bearer: &str) -> Result<FeiyueIdentity, AppError> {
        let response = self
            .client
            .get(format!("{}/auth/me", self.base_url))
            .header(AUTHORIZATION, bearer)
            .send()
            .await
            .map_err(|_| AppError::IdentityUnavailable)?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AppError::Unauthorized);
        }
        if !response.status().is_success() {
            return Err(AppError::IdentityUnavailable);
        }
        response
            .json::<FeiyueIdentity>()
            .await
            .map_err(|_| AppError::IdentityUnavailable)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeiyueIdentity {
    pub sid: String,
    pub name: String,
    pub nickname: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub is_super_admin: bool,
    #[serde(default)]
    pub is_lab_member: bool,
}

impl FeiyueIdentity {
    pub fn is_superadmin(&self) -> bool {
        self.is_super_admin || self.role == "superadmin"
    }

    pub fn can_access_icthub(&self) -> bool {
        self.is_lab_member || self.is_superadmin()
    }

    pub fn can_manage_projects(&self) -> bool {
        self.can_access_icthub()
    }

    pub fn can_manage_tags(&self) -> bool {
        self.is_admin || self.is_superadmin() || self.role == "admin"
    }
}

#[derive(Clone, Debug)]
pub struct AuthContext(pub FeiyueIdentity);

impl FromRequestParts<AppState> for AuthContext {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let bearer = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        async move {
            let bearer = bearer.ok_or(AppError::Unauthorized)?;
            Ok(Self(state.identity.me(&bearer).await?))
        }
    }
}

pub async fn context(AuthContext(identity): AuthContext) -> Result<Json<FeiyueIdentity>, AppError> {
    if !identity.can_access_icthub() {
        return Err(AppError::Forbidden);
    }
    Ok(Json(identity))
}

#[cfg(test)]
mod tests {
    use super::FeiyueIdentity;

    fn identity(role: &str, is_lab_member: bool) -> FeiyueIdentity {
        FeiyueIdentity {
            sid: "20211010001".to_owned(),
            name: "测试".to_owned(),
            nickname: "测试".to_owned(),
            role: role.to_owned(),
            is_admin: role != "user",
            is_super_admin: role == "superadmin",
            is_lab_member,
        }
    }

    #[test]
    fn only_members_or_superadmins_can_manage_projects() {
        assert!(!identity("user", false).can_access_icthub());
        assert!(identity("user", true).can_access_icthub());
        assert!(identity("superadmin", false).can_access_icthub());
        assert!(!identity("user", true).can_manage_tags());
        assert!(identity("admin", true).can_manage_tags());
    }
}
