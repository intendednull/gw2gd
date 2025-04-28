use std::borrow::Cow;

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

// TODO: Rate limiter
pub struct Client {
    inner: reqwest::Client,
    #[allow(unused)]
    token: Option<Cow<'static, str>>,
}

#[derive(thiserror::Error, Debug)]
pub enum GetError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Request failed with status {status}, url: {url}, body: {body}")]
    RequestFailedWithBody {
        status: reqwest::StatusCode,
        url: String,
        body: String,
    },
}

impl Client {
    pub fn new(token: Option<Cow<'static, str>>) -> eyre::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("gw2-gold-digger"));

        if let Some(token) = &token {
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
        }

        Ok(Self {
            inner: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .build()?,
            token,
        })
    }

    pub async fn get<Response>(&self, url: &str) -> Result<Response, GetError>
    where
        Response: serde::de::DeserializeOwned,
    {
        let response = self.inner.get(url).send().await?;

        let status = response.status();

        if !status.is_success() {
            // TODO: Parse error message from response body.
            let body = response.text().await?;
            return Err(GetError::RequestFailedWithBody {
                status,
                body,
                url: url.to_string(),
            });
        }

        Ok(response.json().await?)
    }
}
