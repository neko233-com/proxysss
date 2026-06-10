use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Mutex;

const AZURE_SCOPE: &str = "https://management.azure.com/.default";

pub struct AzureTokenProvider {
    client: Client,
    tenant_id: String,
    client_id: String,
    client_secret: String,
    cache: Arc<Mutex<Option<CachedToken>>>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl AzureTokenProvider {
    pub fn new(tenant_id: String, client_id: String, client_secret: String) -> Self {
        Self {
            client: Client::new(),
            tenant_id,
            client_id,
            client_secret,
            cache: Arc::new(Mutex::new(None)),
        }
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

        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );
        let response = self
            .client
            .post(url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("scope", AZURE_SCOPE),
            ])
            .send()
            .await
            .context("azure oauth token request failed")?
            .error_for_status()
            .context("azure oauth token returned error status")?
            .json::<TokenResponse>()
            .await
            .context("azure oauth token response decode failed")?;

        if response.access_token.trim().is_empty() {
            return Err(anyhow!("azure oauth token response missing access_token"));
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
