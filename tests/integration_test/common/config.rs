use schoolnify_api::config::{
    AppConfig, AuthConfig, CorsConfig, DatabaseConfig, ServerConfig, WorkOsConfig,
};

/// Build a test AppConfig with the wiremock server URL as the WorkOS API base.
pub fn test_config(workos_base_url: &str) -> AppConfig {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://splenwilz@localhost:5432/schoolnify_test".into());

    AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            request_timeout_secs: 30,
        },
        database: DatabaseConfig {
            url: database_url,
            max_connections: 5,
            min_connections: 1,
            acquire_timeout_secs: 10,
        },
        workos: WorkOsConfig {
            api_key: "sk_test_fake_key".into(),
            client_id: "client_test_fake".into(),
            client_secret: "secret_test_fake".into(),
            redirect_uri: "http://localhost:8080/api/v1/auth/callback".into(),
            api_base_url: workos_base_url.into(),
        },
        auth: AuthConfig {
            session_cookie_name: "session_token".into(),
            refresh_cookie_name: "refresh_token".into(),
            access_token_expiry_secs: 900,
            refresh_token_expiry_days: 30,
            cookie_secure: false,
            cookie_http_only: true,
            cookie_same_site: "lax".into(),
            cookie_domain: "".into(),
            post_login_redirect_url: "http://localhost:3000".into(),
            expose_token_in_response: true,
        },
        cors: CorsConfig {
            allowed_origins: vec!["http://localhost:3000".into()],
            base_domain: "localhost".into(),
        },
    }
}
