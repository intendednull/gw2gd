const GW2_API_DOMAIN: &str = "https://api.guildwars2.com";

pub fn build_url(endpoint: &str) -> String {
    format!("{}{}", GW2_API_DOMAIN, endpoint)
}

pub mod listings {
    use std::fmt::Write;

    use super::build_url;
    use crate::client::Client;

    #[derive(serde::Deserialize, Debug)]
    pub struct ListingId(pub u32);

    impl std::fmt::Display for ListingId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
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
        pub id: ListingId,
        pub buys: Vec<ListingItem>,
        pub sells: Vec<ListingItem>,
    }

    pub async fn get_all(client: &Client) -> eyre::Result<Vec<ListingId>> {
        client.get(&build_url("/v2/commerce/listings")).await
    }

    pub async fn get_listing(client: &Client, id: &ListingId) -> eyre::Result<Listings> {
        client
            .get(&build_url(&format!("/v2/commerce/listings/{}", id)))
            .await
    }

    pub async fn get_many_listings(
        client: &Client,
        ids: &[ListingId],
    ) -> eyre::Result<Vec<Listings>> {
        eyre::ensure!(
            ids.len() <= 200,
            "max of 200 ids are allowed, got {}",
            ids.len()
        );
        let param = ids.iter().fold(String::new(), |mut acc, id| {
            if acc.is_empty() {
                id.to_string()
            } else {
                write!(&mut acc, ",{}", id).unwrap();
                acc
            }
        });
        client
            .get(&build_url(&format!("/v2/commerce/listings?ids={}", param)))
            .await
    }
}
