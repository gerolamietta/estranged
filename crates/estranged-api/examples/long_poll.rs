use std::collections::BTreeSet;

use clap::Parser;
use estranged_api::MaxApi;
use estranged_types::{
    Message, MessageBody, NewMessageBody, Recipient, SendResult, UpdateKind, UpdateType, User,
};
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
    let api = MaxApi::new(access_token);
    api.update_stream(None, types)
        .try_for_each(async |update| {
            println!("{update:?}");
            if let UpdateKind::MessageCreated { message, .. } = update.kind
                && let Message {
                    body: MessageBody { text, .. },
                    sender: Some(User { is_bot: false, .. }),
                    recipient:
                        Recipient {
                            chat_id, user_id, ..
                        },
                    ..
                } = *message
                && let Some(text) = text
                && text == "/echo"
            {
                let SendResult { message } = api
                    .send(
                        user_id,
                        chat_id,
                        None,
                        &NewMessageBody {
                            text: Some("echo".into()),
                            ..Default::default()
                        },
                    )
                    .await?;
                println!("{message:?}");
            }
            Ok(())
        })
        .await?;
    Ok(())
}
