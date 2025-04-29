use crate::client::{self, Client};

const GW2_API_DOMAIN: &str = "https://api.guildwars2.com";

pub fn build_url(endpoint: &str) -> String {
    format!("{}{}", GW2_API_DOMAIN, endpoint)
}

/// Represents a Guild Wars 2 Item ID.
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

impl std::fmt::Display for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Definitions for the /v2/commerce/listings endpoint.
/// See: https://wiki.guildwars2.com/wiki/API:2/commerce/listings
pub mod listings {
    use std::fmt::Write;

    use super::*; // Use ListingId from parent scope

    #[derive(thiserror::Error, Debug)]
    pub enum GetManyListingsError {
        #[error("max of 200 ids are allowed, got {0}")]
        TooManyListingIds(usize), // Use usize for len()
        #[error("client error: {0}")]
        ClientError(#[from] client::GetError),
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct ListingItem {
        /// The number of individual listings this object refers to (e.g. two players selling at
        /// the same price will end up in the same listing)
        pub listings: u32,
        /// The sell offer or buy order price in coins.
        pub unit_price: u32,
        /// The amount of items being sold/bought in this listing.
        pub quantity: u32,
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Listings {
        /// The item id these listings belong to. Note: The API calls this 'id' but it refers to the *Item ID*, not Listing ID.
        /// Corrected based on API docs - it's the Item ID. If you need the listing ID concept elsewhere, it's not in this response.
        pub id: super::ItemId, // Changed to use the shared ItemId
        /// The buy order listings (players wanting to buy).
        pub buys: Vec<ListingItem>,
        /// The sell offer listings (players wanting to sell).
        pub sells: Vec<ListingItem>,
    }

    /// Fetches all item IDs that have listings on the trading post.
    /// Corresponds to GET /v2/commerce/listings
    pub async fn get_all_ids(client: &Client) -> Result<Vec<ItemId>, client::GetError> {
        Ok(client.get(&build_url("/v2/commerce/listings")).await?)
    }

    /// Fetches all items that have listings on the trading post.
    /// Corresponds to paginated GET /v2/commerce/listings
    pub async fn get_all(client: &Client) -> Result<Vec<Listings>, client::PaginatedGetError> {
        Ok(client
            .get_all_pages(&build_url("/v2/commerce/listings"), Default::default())
            .await?)
    }

    /// Fetches the buy and sell listings for a single item ID.
    /// Corresponds to GET /v2/commerce/listings/{item_id}
    pub async fn get_listing(
        client: &Client,
        item_id: &super::ItemId, // Parameter should be ItemId
    ) -> Result<Listings, client::GetError> {
        client
            .get(&build_url(&format!("/v2/commerce/listings/{}", item_id)))
            .await
    }

    /// Fetches the buy and sell listings for multiple item IDs.
    /// Corresponds to GET /v2/commerce/listings?ids=...
    /// Note: The API limits the number of IDs per request to 200.
    pub async fn get_many_listings(
        client: &Client,
        item_ids: &[super::ItemId], // Parameter should be ItemId slice
    ) -> Result<Vec<Listings>, GetManyListingsError> {
        if item_ids.len() > 200 {
            return Err(GetManyListingsError::TooManyListingIds(item_ids.len()));
        }

        if item_ids.is_empty() {
            return Ok(Vec::new()); // Return empty vec if no IDs provided
        }

        // Build comma-separated string of IDs
        let param = item_ids.iter().fold(String::new(), |mut acc, id| {
            if !acc.is_empty() {
                acc.push(',');
            }

            // Use write! macro which returns a Result, handle potential formatting errors
            write!(&mut acc, "{}", id).expect("writing ItemId to String should not fail");

            acc
        });

        Ok(client
            .get(&build_url(&format!("/v2/commerce/listings?ids={}", param)))
            .await?)
    }
}

/// Definitions for the /v2/commerce/prices endpoint.
/// See: https://wiki.guildwars2.com/wiki/API:2/commerce/prices
pub mod prices {
    use std::fmt::Write;

    use super::*;

    #[derive(thiserror::Error, Debug)]
    pub enum GetManyPricesError {
        #[error("max of 200 ids are allowed, got {0}")]
        TooManyItemIds(usize), // Use usize for len()
        #[error("client error: {0}")]
        ClientError(#[from] client::GetError),
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct PriceInfo {
        /// The highest buy order or lowest sell offer price in coins.
        pub unit_price: u32,
        /// The amount of items being bought or sold at this price level.
        pub quantity: u32,
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Price {
        /// The item id.
        pub id: ItemId,
        /// Indicates whether a free-to-play account can purchase or sell this item
        /// on the trading post. Defaults to false if missing.
        #[serde(default)]
        pub whitelisted: bool,
        /// Aggregated buy order information (highest bid).
        pub buys: PriceInfo,
        /// Aggregated sell offer information (lowest offer).
        pub sells: PriceInfo,
    }

    /// Fetches all item IDs that have price information on the trading post.
    /// Corresponds to GET /v2/commerce/prices
    pub async fn get_all_ids(client: &Client) -> Result<Vec<ItemId>, client::GetError> {
        Ok(client.get(&build_url("/v2/commerce/prices")).await?)
    }

    /// Fetches all items that have price information on the trading post.
    pub async fn get_all(client: &Client) -> Result<Vec<Price>, client::PaginatedGetError> {
        Ok(client
            .get_all_pages(&build_url("/v2/commerce/prices"), Default::default())
            .await?)
    }

    /// Fetches the aggregated price information for a single item ID.
    /// Corresponds to GET /v2/commerce/prices/{id}
    pub async fn get_price(client: &Client, id: &ItemId) -> Result<Price, client::GetError> {
        client
            .get(&build_url(&format!("/v2/commerce/prices/{}", id)))
            .await
    }

    /// Fetches the aggregated price information for multiple item IDs.
    /// Corresponds to GET /v2/commerce/prices?ids=...
    /// Note: The API limits the number of IDs per request to 200.
    pub async fn get_many_prices(
        client: &Client,
        ids: &[ItemId],
    ) -> Result<Vec<Price>, GetManyPricesError> {
        if ids.len() > 200 {
            return Err(GetManyPricesError::TooManyItemIds(ids.len()));
        }

        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let param = ids.iter().fold(String::new(), |mut acc, id| {
            if !acc.is_empty() {
                acc.push(',');
            }

            write!(&mut acc, "{}", id).expect("writing ItemId to String should not fail");

            acc
        });

        Ok(client
            .get(&build_url(&format!("/v2/commerce/prices?ids={}", param)))
            .await?)
    }
}

/// Definitions for the /v2/commerce/transactions endpoint.
/// These endpoints require authentication with 'account' and 'tradingpost' permissions.
/// The client instance must be configured with a valid API key.
/// Note: These endpoints are paginated by the API. These functions currently fetch only the first page.
/// See: https://wiki.guildwars2.com/wiki/API:2/commerce/transactions
pub mod transactions {
    use super::{build_url, client, Client, ItemId};

    #[derive(serde::Deserialize, Debug)]
    pub struct Transaction {
        /// The transaction id. Note: This can be a large number.
        pub id: u64,
        /// The item id involved in the transaction.
        pub item_id: ItemId,
        /// The price of the item in coins (per item).
        pub price: u32,
        /// The quantity of the item in the transaction.
        pub quantity: u32,
        /// The date the transaction was created (ISO-8601 format string).
        /// Consider using `chrono::DateTime<chrono::Utc>` with the `chrono` feature enabled on `serde`.
        pub created: String,
        /// The date the transaction was completed (ISO-8601 format string).
        /// This field is only present for historical transactions ('history' endpoint).
        /// Consider using `chrono::DateTime<chrono::Utc>` with the `chrono` feature enabled on `serde`.
        pub purchased: Option<String>,
    }

    /// Fetches the current buy transactions (buy orders) for the account.
    /// Corresponds to GET /v2/commerce/transactions/current/buys
    /// Requires authentication: 'account', 'tradingpost' scopes.
    /// Returns the first page of results.
    pub async fn get_current_buys(
        client: &Client,
    ) -> Result<Vec<Transaction>, client::PaginatedGetError> {
        client
            .get_all_pages(
                &build_url("/v2/commerce/transactions/current/buys"),
                Default::default(),
            )
            .await
    }

    /// Fetches the current sell transactions (sell offers) for the account.
    /// Corresponds to GET /v2/commerce/transactions/current/sells
    /// Requires authentication: 'account', 'tradingpost' scopes.
    /// Returns the first page of results.
    pub async fn get_current_sells(
        client: &Client,
    ) -> Result<Vec<Transaction>, client::PaginatedGetError> {
        client
            .get_all_pages(
                &build_url("/v2/commerce/transactions/current/sells"),
                Default::default(),
            )
            .await
    }

    /// Fetches historical buy transactions (completed purchases, up to 90 days) for the account.
    /// Corresponds to GET /v2/commerce/transactions/history/buys
    /// Requires authentication: 'account', 'tradingpost' scopes.
    /// Returns the first page of results.
    pub async fn get_history_buys(
        client: &Client,
    ) -> Result<Vec<Transaction>, client::PaginatedGetError> {
        client
            .get_all_pages(
                &build_url("/v2/commerce/transactions/history/buys"),
                Default::default(),
            )
            .await
    }

    /// Fetches historical sell transactions (completed sales, up to 90 days) for the account.
    /// Corresponds to GET /v2/commerce/transactions/history/sells
    /// Requires authentication: 'account', 'tradingpost' scopes.
    /// Returns the first page of results.
    pub async fn get_history_sells(
        client: &Client,
    ) -> Result<Vec<Transaction>, client::PaginatedGetError> {
        client
            .get_all_pages(
                &build_url("/v2/commerce/transactions/history/sells"),
                Default::default(),
            )
            .await
    }
}
