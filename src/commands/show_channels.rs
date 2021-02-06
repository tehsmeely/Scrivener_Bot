use crate::state::StoreData;
use crate::MessageBuilderExt;
use log::error;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

async fn get_channels(server_id: &GuildId, ctx: &Context) -> String {
    let channel_ids = {
        let store_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StoreData>()
                .expect("Expected StoryData in TypeMap.")
                .clone()
        };
        let store = store_lock.read().unwrap();
        store.get_all_channels_in_server(server_id)
    };
    let mut builder = MessageBuilder::new();
    if channel_ids.len() == 0 {
        builder
            .push_line("No initialised story channels on this server")
            .push("use command [init-channel] to add them")
            .build()
    } else {
        let mut builder = builder.push_bold_line("Channels being watched:");
        for channel_id in channel_ids {
            builder = builder.channel(channel_id).newline();
        }
        builder.build()
    }
}

#[command("show-channels")]
async fn show_channels(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        let response = get_channels(&server_id, &ctx).await;
        if let Err(why) = msg.channel_id.say(&ctx.http, &response).await {
            error!("Error sending message: {:?}", why);
        }
        None
    } else {
        Some(String::from(
            "BUG: message had no server id, bot only supports server text channels",
        ))
    };
    if let Some(simple_response) = reply {
        msg.reply(ctx, simple_response).await?;
    }
    Ok(())
}
