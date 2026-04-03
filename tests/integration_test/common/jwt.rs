use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::pkcs8::LineEnding;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde::Serialize;
use std::sync::OnceLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_KID: &str = "test-key-1";
pub const TEST_CLIENT_ID: &str = "client_test_fake";

struct TestKeyPair {
    encoding_key: EncodingKey,
    jwks_json: String,
}

static KEYPAIR: OnceLock<TestKeyPair> = OnceLock::new();

fn get_keypair() -> &'static TestKeyPair {
    KEYPAIR.get_or_init(|| {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("Failed to generate RSA key");

        // Build encoding key from PEM
        let pem = private_key
            .to_pkcs1_pem(LineEnding::LF)
            .expect("Failed to export PEM");
        let encoding_key =
            EncodingKey::from_rsa_pem(pem.as_bytes()).expect("Failed to create encoding key");

        // Build JWKS JSON from the public key
        let public_key = private_key.to_public_key();
        let n = base64_url_encode(public_key.n().to_bytes_be());
        let e = base64_url_encode(public_key.e().to_bytes_be());

        let jwks_json = serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "alg": "RS256",
                "kid": TEST_KID,
                "n": n,
                "e": e,
            }]
        })
        .to_string();

        TestKeyPair {
            encoding_key,
            jwks_json,
        }
    })
}

fn base64_url_encode(bytes: Vec<u8>) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Debug, Serialize)]
pub struct TestJwtClaims {
    pub sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
}

/// Sign a test JWT with the shared RSA keypair.
pub fn sign_test_jwt(workos_user_id: &str, org_id: Option<&str>, workos_base_url: &str) -> String {
    let kp = get_keypair();
    let now = chrono::Utc::now().timestamp() as usize;

    let claims = TestJwtClaims {
        sub: workos_user_id.into(),
        sid: Some("sess_test_123".into()),
        org_id: org_id.map(|s| s.into()),
        role: None,
        permissions: None,
        exp: now + 900,
        iat: now,
        iss: format!("{}/user_management/{TEST_CLIENT_ID}", workos_base_url.trim_end_matches('/')),
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(TEST_KID.into());

    encode(&header, &claims, &kp.encoding_key).expect("Failed to sign test JWT")
}

/// Mount a JWKS endpoint on the wiremock server that returns the test public key.
pub async fn mount_jwks_endpoint(server: &MockServer) {
    let kp = get_keypair();

    Mock::given(method("GET"))
        .and(path(format!("/sso/jwks/{TEST_CLIENT_ID}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(kp.jwks_json.clone(), "application/json"),
        )
        .expect(1..)
        .mount(server)
        .await;
}
