use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub workos: WorkOsConfig,
    pub auth: AuthConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub request_timeout_secs: u64,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &"[REDACTED]")
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .field("acquire_timeout_secs", &self.acquire_timeout_secs)
            .finish()
    }
}

#[derive(Deserialize, Clone)]
pub struct WorkOsConfig {
    pub api_key: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub api_base_url: String,
}

impl std::fmt::Debug for WorkOsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkOsConfig")
            .field("api_key", &"[REDACTED]")
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("redirect_uri", &self.redirect_uri)
            .field("api_base_url", &self.api_base_url)
            .finish()
    }
}

impl WorkOsConfig {
    pub fn jwks_url(&self) -> String {
        format!("{}/sso/jwks/{}", self.api_base_url, self.client_id)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub session_cookie_name: String,
    pub refresh_cookie_name: String,
    pub access_token_expiry_secs: u64,
    pub refresh_token_expiry_days: i64,
    pub cookie_secure: bool,
    pub cookie_http_only: bool,
    pub cookie_same_site: String,
    pub cookie_domain: String,
    pub post_login_redirect_url: String,
    /// DEV ONLY: Include access_token in JSON response body. Set to false in production.
    pub expose_token_in_response: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CorsConfig {
    #[serde(deserialize_with = "deserialize_string_or_vec")]
    pub allowed_origins: Vec<String>,
    /// Base domain for dynamic CORS subdomain matching (e.g. "schoolnify.com")
    pub base_domain: String,
}

/// Accepts either a JSON array of strings or a comma-separated string.
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a string or a sequence of strings")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            Ok(value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut vec = Vec::new();
            while let Some(val) = seq.next_element::<String>()? {
                vec.push(val);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let run_env = std::env::var("RUN_ENV").unwrap_or_else(|_| "development".into());

        let config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{run_env}")).required(false))
            .add_source(
                Environment::with_prefix("APP")
                    .separator("__"),
            )
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        Ok(app_config)
    }
}
