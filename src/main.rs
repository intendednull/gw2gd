use gw2_gold_digger::{client::Client, gw2_api};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let client = Client::new()?;

    let ids = gw2_api::listings::get_all(&client).await?;
    println!("got {:#?} listing ids from gw2 api", ids.len());

    let listing = gw2_api::listings::get_listing(&client, &ids[0]).await?;
    println!("got {:#?} listing info from gw2 api", listing);

    let listings = gw2_api::listings::get_many_listings(&client, &ids[..10]).await?;
    println!("got {:#?} listings info from gw2 api", listings.len());

    let prices = gw2_api::prices::get_all(&client).await?;
    println!("got {:#?} prices from gw2 api", prices.len());

    let price = gw2_api::prices::get_price(&client, &prices[0]).await?;
    println!("got {:#?} price info from gw2 api", price);

    let prices = gw2_api::prices::get_many_prices(&client, &ids[..10]).await?;
    println!("got {:#?} prices info from gw2 api", prices.len());

    Ok(())
}
