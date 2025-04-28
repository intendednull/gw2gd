use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

pub struct Client {
    inner: reqwest::Client,
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
    pub fn new() -> eyre::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("gw2-gold-digger"));
        Ok(Self {
            inner: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .build()?,
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
