use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

pub struct Client {
    inner: reqwest::Client,
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

    pub async fn get<Response>(&self, url: &str) -> eyre::Result<Response>
    where
        Response: serde::de::DeserializeOwned,
    {
        let response = self.inner.get(url).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            return Err(eyre::eyre!(
                "Request failed with status {}, url: {}, body: {}",
                status,
                url,
                body
            ));
        }

        Ok(response.json().await?)
    }
}
