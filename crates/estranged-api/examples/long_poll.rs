use std::collections::BTreeSet;

use clap::Parser;
use estranged_api::MaxApi;
use estranged_types::UpdateType;
use futures_util::TryStreamExt;

#[derive(Parser)]
struct Args {
    #[clap(long, env = "MAX_TOKEN")]
    access_token: String,
}

#[tokio::main]
async fn main() -> estranged_api::Result<()> {
    dotenvy::dotenv().ok();
    let Args { access_token } = Args::parse();
    let types = &BTreeSet::from([UpdateType::BotStarted, UpdateType::MessageCreated]);
    MaxApi::new(access_token)
        .update_stream(None, types)
        .try_for_each(async |update| {
            println!("{update:?}");
            Ok(())
        })
        .await?;
    Ok(())
}
