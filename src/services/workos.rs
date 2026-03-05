use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use reqwest::Client;
use tokio::sync::RwLock;

use crate::config::WorkOsConfig;
use crate::errors::AppError;
use crate::models::auth::{
    WorkOsAuthResponse, WorkOsCreateUserResponse, WorkOsEmailVerificationRequired,
};

const JWKS_CACHE_TTL: Duration = Duration::from_secs(3600);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

struct JwksCache {
    data: Option<(JwkSet, Instant)>,
    fetching: bool,
}

pub struct WorkOsService {
    client: Client,
    config: WorkOsConfig,
    jwks_cache: Arc<RwLock<JwksCache>>,
}

impl WorkOsService {
    pub fn new(config: WorkOsConfig) -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            config,
            jwks_cache: Arc::new(RwLock::new(JwksCache {
                data: None,
                fetching: false,
            })),
        }
    }

    /// Create a new user in WorkOS with email and password.
    pub async fn create_user(
        &self,
        email: &str,
        password: &str,
        first_name: Option<&str>,
        last_name: Option<&str>,
    ) -> Result<WorkOsCreateUserResponse, AppError> {
        let mut body = serde_json::json!({
            "email": email,
            "password": password,
        });

        if let Some(name) = first_name {
            body["first_name"] = serde_json::Value::String(name.to_string());
        }
        if let Some(name) = last_name {
            body["last_name"] = serde_json::Value::String(name.to_string());
        }

        let response = self
            .client
            .post(format!("{}/user_management/users", self.config.api_base_url))
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("WorkOS request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, error_code = extract_workos_code(&error_body).as_deref().unwrap_or("unknown"), "WorkOS create user failed");

            if status.as_u16() == 409
                || error_body.contains("email_not_available")
                || error_body.contains("already exists")
            {
                return Err(AppError::Conflict("A user with this email already exists".into()));
            }
            if status.as_u16() == 400 {
                return Err(AppError::BadRequest(
                    extract_workos_message(&error_body)
                        .unwrap_or_else(|| "Invalid signup request".into()),
                ));
            }
            return Err(AppError::ExternalService(format!(
                "WorkOS create user failed ({status}): {error_body}"
            )));
        }

        response
            .json::<WorkOsCreateUserResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse WorkOS response: {e}")))
    }

    /// Authenticate a user with email and password via WorkOS.
    ///
    /// Returns `Ok(Ok(auth_response))` on success, or `Ok(Err(email_verification))` when
    /// email verification is required.
    pub async fn authenticate_with_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Result<WorkOsAuthResponse, WorkOsEmailVerificationRequired>, AppError> {
        let body = serde_json::json!({
            "client_id": self.config.client_id,
            "client_secret": self.config.client_secret,
            "grant_type": "password",
            "email": email,
            "password": password,
        });

        let response = self
            .client
            .post(format!(
                "{}/user_management/authenticate",
                self.config.api_base_url
            ))
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("WorkOS request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();

            // Check for email verification required (403 with specific code)
            if error_body.contains("email_verification_required")
                && let Ok(ev) =
                    serde_json::from_str::<WorkOsEmailVerificationRequired>(&error_body)
            {
                tracing::info!(email = %email, "Email verification required");
                return Ok(Err(ev));
            }

            tracing::error!(status = %status, error_code = extract_workos_code(&error_body).as_deref().unwrap_or("unknown"), "WorkOS authentication failed");

            if status.as_u16() == 401 {
                return Err(AppError::Unauthorized(
                    extract_workos_message(&error_body)
                        .unwrap_or_else(|| "Invalid email or password".into()),
                ));
            }
            return Err(AppError::ExternalService(format!(
                "WorkOS authentication failed ({status}): {error_body}"
            )));
        }

        let auth_response = response
            .json::<WorkOsAuthResponse>()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Failed to parse WorkOS response: {e}"))
            })?;

        Ok(Ok(auth_response))
    }

    /// Authenticate with an email verification code.
    pub async fn authenticate_with_email_verification(
        &self,
        code: &str,
        pending_authentication_token: &str,
    ) -> Result<WorkOsAuthResponse, AppError> {
        let body = serde_json::json!({
            "client_id": self.config.client_id,
            "client_secret": self.config.client_secret,
            "grant_type": "urn:workos:oauth:grant-type:email-verification:code",
            "code": code,
            "pending_authentication_token": pending_authentication_token,
        });

        let response = self
            .client
            .post(format!(
                "{}/user_management/authenticate",
                self.config.api_base_url
            ))
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("WorkOS request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, error_code = extract_workos_code(&error_body).as_deref().unwrap_or("unknown"), "WorkOS email verification failed");

            if status.as_u16() == 400 || status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(AppError::BadRequest(
                    extract_workos_message(&error_body)
                        .unwrap_or_else(|| "Invalid verification code".into()),
                ));
            }
            return Err(AppError::ExternalService(format!(
                "WorkOS email verification failed ({status}): {error_body}"
            )));
        }

        response
            .json::<WorkOsAuthResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse WorkOS response: {e}")))
    }

    /// Refresh an access token using a refresh token.
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<WorkOsAuthResponse, AppError> {
        let body = serde_json::json!({
            "client_id": self.config.client_id,
            "client_secret": self.config.client_secret,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        });

        let response = self
            .client
            .post(format!(
                "{}/user_management/authenticate",
                self.config.api_base_url
            ))
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("WorkOS refresh failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, error_code = extract_workos_code(&error_body).as_deref().unwrap_or("unknown"), "WorkOS token refresh failed");

            if status.as_u16() == 401 || status.as_u16() == 400 {
                return Err(AppError::Unauthorized(
                    extract_workos_message(&error_body)
                        .unwrap_or_else(|| "Failed to refresh token".into()),
                ));
            }
            return Err(AppError::ExternalService(format!(
                "WorkOS token refresh failed ({status}): {error_body}"
            )));
        }

        response
            .json::<WorkOsAuthResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse WorkOS response: {e}")))
    }

    /// Build the WorkOS authorization URL for OAuth/SSO flows.
    pub fn get_authorization_url(
        &self,
        provider: Option<&str>,
        connection_id: Option<&str>,
        organization_id: Option<&str>,
        state: Option<&str>,
    ) -> Result<String, AppError> {
        let mut url = url::Url::parse(&format!(
            "{}/user_management/authorize",
            self.config.api_base_url
        ))
        .map_err(|e| AppError::Internal(format!("Failed to build authorize URL: {e}")))?;

        {
            let mut q = url.query_pairs_mut();
            q.append_pair("client_id", &self.config.client_id);
            q.append_pair("redirect_uri", &self.config.redirect_uri);
            q.append_pair("response_type", "code");

            if let Some(provider) = provider {
                q.append_pair("provider", provider);
            }
            if let Some(conn) = connection_id {
                q.append_pair("connection_id", conn);
            }
            if let Some(org) = organization_id {
                q.append_pair("organization_id", org);
            }
            if let Some(state) = state {
                q.append_pair("state", state);
            }
        }

        Ok(url.to_string())
    }

    /// Exchange an authorization code for tokens via WorkOS.
    pub async fn authenticate_with_code(
        &self,
        code: &str,
    ) -> Result<WorkOsAuthResponse, AppError> {
        let body = serde_json::json!({
            "client_id": self.config.client_id,
            "client_secret": self.config.client_secret,
            "grant_type": "authorization_code",
            "code": code,
        });

        let response = self
            .client
            .post(format!(
                "{}/user_management/authenticate",
                self.config.api_base_url
            ))
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("WorkOS request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, error_code = extract_workos_code(&error_body).as_deref().unwrap_or("unknown"), "WorkOS code exchange failed");

            if status.as_u16() == 400 || status.as_u16() == 401 {
                return Err(AppError::BadRequest(
                    extract_workos_message(&error_body)
                        .unwrap_or_else(|| "Invalid authorization code".into()),
                ));
            }
            return Err(AppError::ExternalService(format!(
                "WorkOS code exchange failed ({status}): {error_body}"
            )));
        }

        response
            .json::<WorkOsAuthResponse>()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse WorkOS response: {e}")))
    }

    /// Fetch JWKS from WorkOS for JWT validation (cached for 1 hour).
    ///
    /// Uses a `fetching` flag to prevent thundering herd: when multiple concurrent
    /// requests find an expired cache, only one fetches while others use stale data.
    pub async fn get_jwks(&self) -> Result<JwkSet, AppError> {
        // Check cache first (read lock)
        {
            let cache = self.jwks_cache.read().await;
            if let Some((keys, fetched_at)) = &cache.data {
                if fetched_at.elapsed() < JWKS_CACHE_TTL {
                    return Ok(keys.clone());
                }
                // Cache is stale but a fetch is already in progress — return stale data
                if cache.fetching {
                    return Ok(keys.clone());
                }
            }
        }

        // Try to become the fetcher (write lock)
        {
            let mut cache = self.jwks_cache.write().await;
            // Double-check: another task may have refreshed while we waited for the lock
            if let Some((keys, fetched_at)) = &cache.data
                && fetched_at.elapsed() < JWKS_CACHE_TTL
            {
                return Ok(keys.clone());
            }
            cache.fetching = true;
        }

        let jwks_url = self.config.jwks_url();
        tracing::debug!(url = %jwks_url, "Fetching JWKS from WorkOS");

        let result = self.fetch_jwks(&jwks_url).await;

        // Update cache regardless of success/failure, clear fetching flag
        let mut cache = self.jwks_cache.write().await;
        cache.fetching = false;
        match &result {
            Ok(jwk_set) => {
                cache.data = Some((jwk_set.clone(), Instant::now()));
            }
            Err(_) => {
                // On fetch failure, keep stale data if available
            }
        }

        result
    }

    /// Force a fresh JWKS fetch, bypassing the cache TTL.
    /// Used when a JWT has a `kid` not found in the current cache (key rotation).
    pub async fn get_jwks_force_refresh(&self) -> Result<JwkSet, AppError> {
        let jwks_url = self.config.jwks_url();
        tracing::debug!(url = %jwks_url, "Force-refreshing JWKS from WorkOS");

        let result = self.fetch_jwks(&jwks_url).await?;

        let mut cache = self.jwks_cache.write().await;
        cache.data = Some((result.clone(), Instant::now()));

        Ok(result)
    }

    async fn fetch_jwks(&self, url: &str) -> Result<JwkSet, AppError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("JWKS fetch failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::ExternalService(format!(
                "JWKS fetch returned {status}"
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("JWKS parse failed: {e}")))
    }
}

/// Extract the top-level "message" field from a WorkOS error JSON body.
fn extract_workos_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("message")?.as_str().map(String::from))
}

/// Extract the top-level "code" field from a WorkOS error JSON body (PII-safe for logging).
fn extract_workos_code(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("code")?.as_str().map(String::from))
}
