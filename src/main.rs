use gw2gd::{api, client::Client};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let token = "A9E6E9AB-CB07-894F-B1CE-2EC2CB292AA35C71B0AF-D72F-4B7E-9AAA-8B93D8C4AB53";
    let client = Client::new(Some(token.into()))?;

    let ids = api::listings::get_all_ids(&client).await?;
    println!("got {:#?} listing ids from gw2 api", ids.len());

    let listing = api::listings::get_listing(&client, &ids[0]).await?;
    println!("got {:#?} listing info from gw2 api", listing.id);

    let listings = api::listings::get_many_listings(&client, &ids[..10]).await?;
    println!("got {:#?} listings info from gw2 api", listings.len());

    let prices = api::prices::get_all_ids(&client).await?;
    println!("got {:#?} prices from gw2 api", prices.len());

    let price = api::prices::get_price(&client, &prices[0]).await?;
    println!("got {:#?} price info from gw2 api", price);

    let prices = api::prices::get_many_prices(&client, &ids[..10]).await?;
    println!("got {:#?} prices info from gw2 api", prices.len());

    let current_buys = api::transactions::get_current_buys(&client).await?;
    println!("got {:#?} current buys from gw2 api", current_buys.len());

    let current_sells = api::transactions::get_current_sells(&client).await?;
    println!("got {:#?} current sells from gw2 api", current_sells.len());

    // let all = gw2_api::listings::get_all(&client).await?;
    // println!("got {:#?} listings from gw2 api", all);

    Ok(())
}
