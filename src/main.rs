use gw2_gold_digger::{client::Client, gw2_api};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let client = Client::new()?;
    let ids = gw2_api::listings::get_all(&client).await?;
    let listings = gw2_api::listings::get_many_listings(&client, &ids[..10]).await?;
    println!("got {:#?} listings from gw2 api", listings.len());

    Ok(())
}
