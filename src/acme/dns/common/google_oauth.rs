use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const GOOGLE_DNS_SCOPE: &str = "https://www.googleapis.com/auth/ndev.clouddns.readwrite";

pub struct GoogleTokenProvider {
    client: Client,
    client_email: String,
    private_key: EncodingKey,
    token_uri: String,
    cache: Arc<Mutex<Option<CachedToken>>>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct ServiceAccountJson {
    client_email: String,
    private_key: String,
    token_uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServiceAccountClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl GoogleTokenProvider {
    pub fn from_service_account_json(json: &str) -> Result<Self> {
        let parsed: ServiceAccountJson =
            serde_json::from_str(json).context("failed to parse google service account json")?;
        if parsed.client_email.trim().is_empty() || parsed.private_key.trim().is_empty() {
            return Err(anyhow!(
                "google service account json requires client_email and private_key"
            ));
        }
        let private_key = EncodingKey::from_rsa_pem(parsed.private_key.as_bytes())
            .context("failed to parse google service account private key")?;
        Ok(Self {
            client: Client::new(),
            client_email: parsed.client_email,
            private_key,
            token_uri: parsed
                .token_uri
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "https://oauth2.googleapis.com/token".to_string()),
            cache: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn access_token(&self) -> Result<String> {
        {
            let cache = self.cache.lock().await;
            if let Some(token) = cache.as_ref() {
                if token.expires_at > Instant::now() + Duration::from_secs(60) {
                    return Ok(token.access_token.clone());
                }
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let claims = ServiceAccountClaims {
            iss: &self.client_email,
            scope: GOOGLE_DNS_SCOPE,
            aud: &self.token_uri,
            iat: now,
            exp: now + 3600,
        };
        let jwt = encode(&Header::new(Algorithm::RS256), &claims, &self.private_key)
            .context("failed to sign google service account jwt")?;

        let response = self
            .client
            .post(&self.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt.as_str()),
            ])
            .send()
            .await
            .context("google oauth token request failed")?
            .error_for_status()
            .context("google oauth token returned error status")?
            .json::<TokenResponse>()
            .await
            .context("google oauth token response decode failed")?;

        if response.access_token.trim().is_empty() {
            return Err(anyhow!("google oauth token response missing access_token"));
        }

        let expires_at =
            Instant::now() + Duration::from_secs(response.expires_in.saturating_sub(120).max(60));
        let access_token = response.access_token;
        {
            let mut cache = self.cache.lock().await;
            *cache = Some(CachedToken {
                access_token: access_token.clone(),
                expires_at,
            });
        }
        Ok(access_token)
    }
}
