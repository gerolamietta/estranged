use std::collections::BTreeSet;

use clap::Parser;
use estranged_api::MaxApi;
use estranged_types::{UpdateType, Updates};

#[derive(Parser)]
struct Args {
    #[clap(long, env = "MAX_TOKEN")]
    access_token: String,
}

#[tokio::main]
async fn main() -> estranged_api::Result<()> {
    dotenvy::dotenv().ok();
    let Args { access_token } = Args::parse();
    let api = MaxApi::new(access_token);
    let types = &BTreeSet::from([UpdateType::BotStarted, UpdateType::MessageCreated]);
    let Updates { updates, marker } = api.updates(Some(1), None, types).await?;
    updates.iter().for_each(|update| println!("{update:?}"));
    println!("{marker:?}");
    let Updates { updates, .. } = api.updates(Some(1), marker, types).await?;
    updates.iter().for_each(|update| println!("{update:?}"));
    Ok(())
}
