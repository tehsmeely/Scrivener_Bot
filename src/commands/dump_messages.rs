use crate::state::StoreData;
use crate::utils::trait_extensions::MessageBuilderExt;
use log::error;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::fs::File;

async fn actually_dump_messages(
    ctx: &Context,
    channel: GuildChannel,
    last_message_id: MessageId,
    filename: &str,
) {
    let mut messages: Vec<String> = vec![];
    let last_msg = channel.message(&ctx.http, last_message_id).await.unwrap();
    messages.push(last_msg.content);
    messages.extend(
        channel
            .messages(&ctx.http, |get_messages_builder| {
                get_messages_builder.before(last_message_id).limit(50)
            })
            .await
            .unwrap()
            .iter()
            .map(|m: &Message| m.content.clone()),
    );
    let dump_file = File::create(filename).unwrap();
    serde_json::to_writer_pretty(dump_file, &messages).unwrap();
}

#[command("dump-messages")]
async fn dump_messages(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = match args.single::<ChannelId>() {
        Ok(channel_id) => {
            let channel = channel_id.to_channel(ctx).await.unwrap().guild().unwrap();
            if channel.kind == ChannelType::Text {
                if let Some(last_message_id) = channel.last_message_id {
                    let filename = format!("{}.messages.json", channel.name);
                    actually_dump_messages(ctx, channel, last_message_id, &filename).await;
                    format!("Dumped messages to file {}", filename)
                } else {
                    String::from("Channel has no messages in it")
                }
            } else {
                String::from("Channel is not text channel")
            }
        }
        Err(e) => format!("Expected one arg of channel mention. Error: {}", e),
    };
    msg.reply(ctx, reply).await?;
    Ok(())
}
