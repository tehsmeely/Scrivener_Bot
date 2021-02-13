use crate::state::StoreData;
use crate::utils::trait_extensions::MessageBuilderExt;
use crate::ADMINONLY_CHECK;
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
#[usage("<#channel name>")]
#[description("Dumps the last up to 50 messages from a channel to a file, locally to the server. For debug reasons")]
#[example("#the-fall-of-rome")]
#[checks("AdminOnly")]
async fn dump_messages(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = match args.len() {
        1 => match args.single::<ChannelId>() {
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
        },
        2 => {
            let maybe_server_name = args.single::<String>();
            let maybe_channel_name = args.single::<String>();
            match (maybe_server_name, maybe_channel_name) {
                (Ok(server_name_), Ok(channel_name_)) => {
                    let channel_name =
                        crate::utils::helpers::strip_leading_trailing(&channel_name_, '"');
                    let server_name =
                        crate::utils::helpers::strip_leading_trailing(&server_name_, '"');
                    match dump_server_channel_from_store(ctx, server_name, channel_name).await {
                        Ok(()) => String::from("Done"),
                        Err(e) => format!("Failed: {}", e),
                    }
                }
                _ => String::from("Invalid arguments"),
            }
        }
        _ => String::from("1 Arg expected"),
    };
    msg.reply(ctx, reply).await?;
    Ok(())
}

async fn dump_server_channel_from_store(
    ctx: &Context,
    server_name: &str,
    channel_name: &str,
) -> std::result::Result<(), &'static str> {
    log::info!("Dumping {}:{}", server_name, channel_name);
    let server_id: GuildId = {
        let server_ids = {
            let store_lock = {
                let data_read = ctx.data.read().await;
                data_read
                    .get::<StoreData>()
                    .expect("Expected StoryData in TypeMap.")
                    .clone()
            };
            let store = store_lock.read().unwrap();
            store.get_unique_server_ids()
        };
        let mut res = None;
        for server_id in server_ids {
            let server = server_id.name(ctx).await.unwrap();
            log::info!("Have server named: {}", server);
            if server == server_name {
                res = Some(server_id);
            }
        }
        match res {
            Some(server_id) => server_id,
            None => return Err("No server found with name"),
        }
    };
    let channel = {
        let channels: std::collections::HashMap<ChannelId, GuildChannel> =
            server_id.channels(ctx).await.unwrap();
        let mut res = None;
        for (_id, channel) in channels.iter() {
            if &channel.name == channel_name && channel.kind == ChannelType::Text {
                res = Some(channel.clone())
            }
        }
        match res {
            Some(channel) => channel,
            None => return Err("No channel found with name"),
        }
    };
    let filename = format!("{}.{}.dump.json", server_name, channel_name);
    if let Some(last_message_id) = channel.last_message_id {
        actually_dump_messages(ctx, channel, last_message_id, &filename).await
    }
    Ok(())
}
