use std::{borrow::Cow, fmt, str::FromStr};

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::de::DeserializeOwned;

pub const DEFAULT_PAGE_SIZE: usize = 200;

/// Error type for non-paginated `get` requests.
#[derive(thiserror::Error, Debug)]
pub enum NewClientError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to build reqwest client: {0}")]
    InvalidToken(#[from] reqwest::header::InvalidHeaderValue),
}

/// Error type for non-paginated `get` requests.
#[derive(thiserror::Error, Debug)]
pub enum GetError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Request failed: status {status}, url: {url}, body: {body}")]
    RequestFailedWithBody {
        status: reqwest::StatusCode,
        url: String,
        body: String,
    },
}

/// Error type for paginated `get_paginated` requests.
#[derive(thiserror::Error, Debug)]
pub enum PaginatedGetError {
    #[error("HTTP request error: {0}")]
    Http(reqwest::Error), // Don't use #[from] here to distinguish source easily

    #[error("Request failed: status {status}, url: {url}, body: {body}")]
    RequestFailedWithBody {
        status: reqwest::StatusCode,
        url: String,
        body: String,
    },

    #[error("Failed to parse pagination header '{header_name}': {source}")]
    HeaderParseError {
        header_name: String,
        source: Box<dyn std::error::Error + Send + Sync>, // Box to handle different parse errors
    },

    #[error("Missing required pagination header: {header_name}")]
    MissingHeaderError { header_name: String },

    #[error("Failed to deserialize response body: {0}")]
    DeserializationError(reqwest::Error), // Capture the specific deserialization error
}

/// A client for interacting with the Guild Wars 2 API.
pub struct Client {
    inner: reqwest::Client,
    #[allow(unused)]
    token: Option<Cow<'static, str>>,
    rate_limiter: rate_limiter::RateLimiter,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("inner", &self.inner)
            .field("token", &self.token.as_ref().map(|_| Cow::Borrowed("****"))) // Avoid logging token
            .finish()
    }
}

impl Client {
    /// Creates a new API client.
    ///
    /// # Arguments
    ///
    /// * `token` - An optional API token (bearer token).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built or if the token is invalid for the header.
    pub fn new(token: Option<Cow<'static, str>>) -> Result<Self, NewClientError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("gw2gd")); // Example user agent

        if let Some(token_ref) = &token {
            let mut auth_value = HeaderValue::from_str(&format!("Bearer {}", token_ref))?;
            auth_value.set_sensitive(true); // Mark the token as sensitive
            headers.insert(AUTHORIZATION, auth_value);
        }

        let inner = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            inner,
            token,
            rate_limiter: rate_limiter::RateLimiter::new(300, 5.0),
        })
    }

    /// Performs a standard GET request without pagination.
    ///
    /// # Type Parameters
    ///
    /// * `Response`: The type to deserialize the JSON response into.
    ///
    /// # Arguments
    ///
    /// * `url`: The full URL for the API endpoint.
    ///
    /// # Errors
    ///
    /// Returns `GetError` variants for network issues or non-successful API responses.
    pub async fn get<Response>(&self, url: &str) -> Result<Response, GetError>
    where
        Response: DeserializeOwned,
    {
        self.rate_limiter.acquire(1).await;

        let response = self.inner.get(url).send().await?; // Propagates reqwest::Error via #[from]

        let status = response.status();

        if !status.is_success() {
            // TODO: Parse the error message if possible
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("Failed to read error body: {}", e));
            return Err(GetError::RequestFailedWithBody {
                status,
                body,
                url: url.to_string(),
            });
        }

        Ok(response.json().await?)
    }

    /// Performs a GET request to a paginated endpoint.
    ///
    /// # Type Parameters
    ///
    /// * `Response`: The type to deserialize the JSON response *data* into (typically a `Vec<T>`).
    ///
    /// # Arguments
    ///
    /// * `base_url`: The base URL for the paginated endpoint (without query parameters).
    /// * `params`: The pagination parameters (`page`, `page_size`) for the request.
    ///
    /// # Errors
    ///
    /// Returns `PaginatedGetError` variants for network issues, non-successful API responses,
    /// missing or invalid pagination headers, or JSON deserialization failures.
    pub async fn get_paginated<Response>(
        &self,
        base_url: &str,
        params: PaginationParams,
    ) -> Result<Paginated<Response>, PaginatedGetError>
    where
        Response: DeserializeOwned,
    {
        self.rate_limiter.acquire(1).await;

        let paginated_url = if base_url.contains('?') {
            format!("{}&{}", base_url, params.to_query_string())
        } else {
            format!("{}?{}", base_url, params.to_query_string())
        };

        let response = self
            .inner
            .get(&paginated_url)
            .send()
            .await
            .map_err(PaginatedGetError::Http)?; // Map reqwest::Error explicitly

        let status = response.status();
        let headers = response.headers().clone(); // Clone headers for potential error reporting

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("Failed to read error body: {}", e));

            return Err(PaginatedGetError::RequestFailedWithBody {
                status,
                body,
                url: paginated_url,
            });
        }

        // Helper function to parse required headers
        fn parse_required_header<T: FromStr>(
            headers: &HeaderMap,
            header_name: &'static str,
        ) -> Result<T, PaginatedGetError>
        where
            <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
        {
            let header_value =
                headers
                    .get(header_name)
                    .ok_or_else(|| PaginatedGetError::MissingHeaderError {
                        header_name: header_name.to_string(),
                    })?;

            let s = header_value.to_str().map_err(|e| {
                PaginatedGetError::HeaderParseError {
                    header_name: header_name.to_string(),
                    source: Box::new(e), // Box the original error
                }
            })?;

            s.parse::<T>()
                .map_err(|e| PaginatedGetError::HeaderParseError {
                    header_name: header_name.to_string(),
                    source: Box::new(e), // Box the original error
                })
        }

        // Extract pagination metadata - return specific errors if headers are missing/invalid
        let metadata = PaginationMetadata {
            page_size: parse_required_header(&headers, "X-Page-Size")?,
            page_total: parse_required_header(&headers, "X-Page-Total")?,
            result_count: parse_required_header(&headers, "X-Result-Count")?,
            result_total: parse_required_header(&headers, "X-Result-Total")?,
        };

        // Deserialize the JSON body *after* successfully processing headers
        let data = response
            .json()
            .await
            .map_err(PaginatedGetError::DeserializationError)?; // Map deserialization error

        Ok(Paginated { data, metadata })
    }

    /// Helper method to fetch all pages for a given paginated endpoint.
    ///
    /// This method repeatedly calls `get_paginated` until all pages are fetched.
    /// It aggregates the data from each page. Note that this can result in many API calls.
    ///
    /// # Type Parameters
    ///
    /// * `Item`: The type of a single item within the paginated response data (e.g., `Transaction`).
    ///
    /// # Arguments
    ///
    /// * `base_url`: The base URL for the paginated endpoint.
    /// * `initial_page_size`: The page size to use for requests.
    ///
    /// # Errors
    ///
    /// Returns `PaginatedGetError` if any of the underlying page requests fail.
    pub async fn get_all_pages<Item>(
        &self,
        base_url: &str,
        params: PaginationParams,
    ) -> Result<Vec<Item>, PaginatedGetError>
    where
        Vec<Item>: DeserializeOwned, // Ensure the target Vec<Item> can be deserialized
    {
        let mut all_items = Vec::new();
        let mut current_params = params;

        tracing::trace!(
            "Fetching first page from {} with params: {:?}",
            base_url,
            current_params
        );

        let first_response: Paginated<Vec<Item>> =
            self.get_paginated(base_url, current_params).await?;

        all_items.extend(first_response.data);

        for page in 1..first_response.metadata.page_total {
            current_params = current_params.next();

            tracing::trace!(
                "Fetching page {} from {} with params: {:?}",
                page,
                base_url,
                current_params
            );

            let response: Paginated<Vec<Item>> =
                self.get_paginated(base_url, current_params).await?;

            all_items.extend(response.data);
        }

        Ok(all_items)
    }
}

/// Parameters for paginated API requests.
#[derive(Debug, Clone, Copy)]
pub struct PaginationParams {
    /// The page number (0-indexed).
    pub page: usize,
    /// Number of items per page (max typically 200 for GW2 API).
    pub page_size: usize,
}

impl Default for PaginationParams {
    /// Defaults to the first page with a size of 200.
    fn default() -> Self {
        Self {
            page: 0,
            page_size: DEFAULT_PAGE_SIZE, // Common GW2 API max page size
        }
    }
}

impl PaginationParams {
    /// Creates new pagination parameters.
    pub fn new(page: usize, page_size: usize) -> Self {
        Self { page, page_size }
    }

    /// Creates parameters for the first page with a specific size.
    pub fn first(page_size: usize) -> Self {
        Self { page: 0, page_size }
    }

    /// Formats the parameters as a query string fragment (without leading '?').
    pub fn to_query_string(&self) -> String {
        format!("page={}&page_size={}", self.page, self.page_size)
    }

    /// Gets the parameters for the next page.
    pub fn next(&self) -> PaginationParams {
        PaginationParams {
            page: self.page + 1,
            page_size: self.page_size,
        }
    }
}

/// Metadata extracted from paginated API response headers.
#[derive(Debug, Clone, Copy)]
pub struct PaginationMetadata {
    /// Number of entries per page (obtained from response).
    pub page_size: usize,
    /// Total number of pages available (obtained from response).
    pub page_total: usize,
    /// Number of results in the current response (obtained from response).
    pub result_count: usize,
    /// Total number of results across all pages (obtained from response).
    pub result_total: usize,
}

/// Represents a paginated response, containing both the data and pagination metadata.
#[derive(Debug, Clone)]
pub struct Paginated<T> {
    /// The actual data returned by the API for the current page.
    pub data: T,
    /// Pagination metadata extracted from response headers.
    pub metadata: PaginationMetadata,
}

pub mod rate_limiter {
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    use tracing::instrument;

    /// A lazy token bucket rate limiter for async Rust code.
    /// Not thread-safe - designed for use in a single task.
    pub struct RateLimiter {
        /// Maximum capacity of tokens
        capacity: u32,
        /// Rate at which tokens refill (tokens per second)
        refill_rate: f64,
        /// Available tokens (lazily calculated when needed)
        available_tokens: Cell<f64>,
        /// Last time tokens were calculated
        last_update: Cell<Instant>,
    }

    impl RateLimiter {
        /// Create a new rate limiter with the given capacity and refill rate
        pub fn new(capacity: u32, tokens_per_second: f64) -> Self {
            tracing::info!(capacity, tokens_per_second, "Creating new AsyncRateLimiter");
            RateLimiter {
                capacity,
                refill_rate: tokens_per_second,
                available_tokens: Cell::new(0.),
                last_update: Cell::new(Instant::now()),
            }
        }

        /// Calculate current token count based on elapsed time
        fn calculate_current_tokens(&self) {
            let now = Instant::now();
            let last = self.last_update.get();
            let elapsed = now.duration_since(last).as_secs_f64();

            if elapsed > 0.0 {
                // Calculate new tokens based on elapsed time
                let new_tokens = self.refill_rate * elapsed;
                let current = self.available_tokens.get();

                // Update available tokens (capped at capacity)
                let updated = (current + new_tokens).min(self.capacity as f64);

                tracing::trace!(
                    elapsed_secs = elapsed,
                    new_tokens,
                    before = current,
                    after = updated,
                    "Refreshed token bucket"
                );

                self.available_tokens.set(dbg!(updated));
                self.last_update.set(now);
            }
        }

        /// Try to acquire tokens immediately without waiting
        /// Returns true if successful, false if not enough tokens
        #[instrument(skip(self), fields(capacity = self.capacity, available = self.available()))]
        pub fn try_acquire(&self, tokens: u32) -> bool {
            self.calculate_current_tokens();

            let available = self.available_tokens.get();
            if available < tokens as f64 {
                tracing::info!(requested = tokens, available, "Rate limit exceeded");
                return false;
            }

            self.available_tokens.set(available - tokens as f64);
            tracing::trace!(
                tokens,
                remaining = self.available_tokens.get(),
                "Tokens acquired"
            );
            true
        }

        /// Acquire specified number of tokens, waiting if necessary
        pub async fn acquire(&self, tokens: u32) {
            self.calculate_current_tokens();

            let available = self.available_tokens.get();
            if available >= tokens as f64 {
                // We have enough tokens available
                self.available_tokens.set(available - tokens as f64);
                tracing::trace!(tokens, "Tokens acquired immediately");
                return;
            }

            // Calculate tokens needed and wait time
            let tokens_needed = tokens as f64 - available;
            let wait_time = Duration::from_secs_f64(tokens_needed / self.refill_rate);

            tracing::trace!(
                tokens,
                tokens_needed,
                wait_time_ms = wait_time.as_millis(),
                "Waiting for token refill"
            );

            // Use all currently available tokens
            self.available_tokens.set(0.0);

            // Wait for remaining tokens to become available
            tokio::time::sleep(wait_time).await;

            // Update time after waiting
            self.last_update.set(Instant::now());
            tracing::trace!(tokens, "Tokens acquired after waiting");
        }

        /// Acquire tokens with a timeout
        /// Returns true if tokens were acquired, false if timeout reached
        pub async fn acquire_with_timeout(&self, tokens: u32, timeout: Duration) -> bool {
            self.calculate_current_tokens();

            let available = self.available_tokens.get();
            if available >= tokens as f64 {
                // We have enough tokens available
                self.available_tokens.set(available - tokens as f64);
                tracing::trace!(tokens, "Tokens acquired immediately with timeout");
                return true;
            }

            // Calculate how long we'd need to wait
            let tokens_needed = tokens as f64 - available;
            let required_wait = Duration::from_secs_f64(tokens_needed / self.refill_rate);

            if required_wait > timeout {
                tracing::trace!(
                    required_wait_ms = required_wait.as_millis(),
                    timeout_ms = timeout.as_millis(),
                    "Timeout too short for required wait"
                );
                return false; // Would exceed timeout
            }

            // Use all available tokens and wait
            self.available_tokens.set(0.0);

            tracing::trace!(
                tokens,
                wait_time_ms = required_wait.as_millis(),
                "Waiting for token refill with timeout"
            );

            tokio::time::sleep(required_wait).await;
            self.last_update.set(Instant::now());
            tracing::trace!(tokens, "Tokens acquired after waiting with timeout");

            true
        }

        /// Get current available tokens (for debugging/testing)
        pub fn available(&self) -> f64 {
            self.calculate_current_tokens();
            self.available_tokens.get()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tokio::time::{sleep, Duration, Instant};

        fn assert_float_eq(a: f64, b: f64, epsilon: f64) {
            assert!(
                (a - b).abs() < epsilon,
                "Values are not equal: {} != {}",
                a,
                b
            );
        }

        #[tokio::test]
        async fn test_try_acquire_success() {
            let limiter = RateLimiter::new(5, 9.0);
            tokio::time::sleep(Duration::from_secs(1)).await;
            assert!(limiter.try_acquire(3));
            assert_float_eq(limiter.available(), 2., 0.01);
        }

        #[tokio::test]
        async fn test_try_acquire_failure() {
            let limiter = RateLimiter::new(5, 1.0);
            assert!(!limiter.try_acquire(6));
            assert_float_eq(limiter.available(), 0., 0.01);
        }

        #[tokio::test]
        async fn test_acquire_immediate() {
            let limiter = RateLimiter::new(5, 100.);
            tokio::time::sleep(Duration::from_millis(200)).await;
            limiter.acquire(3).await;
            assert_float_eq(limiter.available(), 2.0, 0.01);
        }

        #[tokio::test]
        async fn test_acquire_wait() {
            let limiter = RateLimiter::new(5, 1.0);
            limiter.try_acquire(5);
            let start = Instant::now();
            limiter.acquire(1).await;
            let elapsed = start.elapsed();
            assert!(elapsed >= Duration::from_secs(1));
            assert_float_eq(limiter.available(), 0.0, 0.01);
        }

        #[tokio::test]
        async fn test_acquire_with_timeout_success() {
            let limiter = RateLimiter::new(5, 1.0);
            limiter.try_acquire(5);
            let result = limiter
                .acquire_with_timeout(1, Duration::from_secs(2))
                .await;
            assert!(result);
            assert_float_eq(limiter.available(), 0.0, 0.01);
        }

        #[tokio::test]
        async fn test_acquire_with_timeout_failure() {
            let limiter = RateLimiter::new(5, 1.0);
            limiter.try_acquire(5);
            let result = limiter
                .acquire_with_timeout(2, Duration::from_secs(1))
                .await;
            assert!(!result);
            assert_float_eq(limiter.available(), 0.0, 0.01);
        }

        #[tokio::test]
        async fn test_available_tokens_refill() {
            let limiter = RateLimiter::new(5, 1.0);
            limiter.try_acquire(5);
            assert_float_eq(limiter.available(), 0.0, 0.01);
            sleep(Duration::from_secs(3)).await;
            let available = limiter.available();
            assert_float_eq(available, 3.0, 0.01);
        }
    }
}
