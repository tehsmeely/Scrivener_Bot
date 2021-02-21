use crate::state::{ServerData, Store, StoreData, StoryKey};
use crate::stats::WordStats;
use crate::utils::iterators::SortedHashMap;
use crate::utils::trait_extensions::MessageBuilderExt;
use chrono::{DateTime, NaiveDateTime, Utc};
use serenity::framework::standard::help_commands::with_embeds;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::hash::Hash;
use std::{cmp, collections::HashMap};

async fn make_server_summary(ctx: &Context, user_id: &UserId, server_id: &GuildId) -> String {
    let user = user_id.to_user(ctx).await.unwrap();
    let mut channel_ids_with_counts = {
        let store_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StoreData>()
                .expect("Expected StoreData in TypeMap.")
                .clone()
        };
        let store = store_lock.read().unwrap();
        let server = store.get_server_data(server_id);
        match server {
            Some(server_data) => server_data.channel_ids_by_wordcount_for_user(&user),
            None => return format!("There are no initialised channels on this server"),
        }
    };
    let mut channels_with_counts = vec![];
    for (channel_id, wc) in channel_ids_with_counts.drain(..) {
        let channel = channel_id.to_channel(ctx).await.unwrap();
        channels_with_counts.push((channel, wc));
    }
    ServerData::make_user_stats_string(&user, channels_with_counts)
}

#[command("server-summary")]
#[usage("<@ user mention>")]
#[description("Display stats for a given user across all initialised channels on this server")]
#[example("@Caligula")]
#[only_in("guilds")] // Reminder: guild = server
async fn server_summary(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        match parse_args(&mut args) {
            Ok(user) => Some(make_server_summary(ctx, &user, &server_id).await),
            Err(e) => Some(e),
        }
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

fn parse_args(args: &mut Args) -> std::result::Result<(UserId), String> {
    match args.len() {
        1 => match args.single::<UserId>() {
            Ok(user_id) => Ok(user_id),
            Err(e) => Err(format!(
                "Error with command arguments, try [help server-summary]\nError:{}",
                e,
            )),
        },
        _ => Err(String::from(
            "Invalid number of args, try [help server-summary]",
        )),
    }
}
