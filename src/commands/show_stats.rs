use crate::state::{StoreData, StoryKey};
use crate::stats::WordStats;
use crate::utils::iterators::SortedHashMap;
use crate::MessageBuilderExt;
use chrono::{DateTime, NaiveDateTime, Utc};
use serenity::framework::standard::help_commands::with_embeds;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::hash::Hash;
use std::{cmp, collections::HashMap};

fn sort_by_last_message_and_maybe_truncate(
    stats_map: &HashMap<User, WordStats>,
    truncate_limit: Option<usize>,
) -> SortedHashMap<User, WordStats> {
    let epoch = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
    let mut ref_vector: Vec<(&User, Option<&DateTime<Utc>>)> = stats_map
        .iter()
        .map(|(user, stats)| (user, stats.last_message_time()))
        .collect();
    ref_vector.sort_by_key(|(_, d)| d.unwrap_or(&epoch));
    //We reverse it for the keys since we want newest (i.e. highest date value) first
    let sorted_keys = ref_vector.iter().rev().map(|(user, _)| *user).collect();
    SortedHashMap::new(stats_map, sorted_keys, truncate_limit)
}

async fn get_stats(channel_id: ChannelId, ctx: &Context, truncate_limit: Option<usize>) -> String {
    let text_channel = channel_id.to_channel(&ctx).await.unwrap().guild().unwrap();
    let story_key: StoryKey = (text_channel.guild_id, channel_id);
    let store_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<StoreData>()
            .expect("Expected StoryData in TypeMap.")
            .clone()
    };
    let store = store_lock.read().unwrap();
    match store.data.get(&story_key) {
        Some(story_data) => {
            let mut builder = MessageBuilder::new();
            let base_builder = builder
                .push("For ")
                .channel(text_channel)
                .newline()
                .push_bold_line("General")
                .push_line_safe(format!(
                    "Word count: {}",
                    story_data.general_stats.word_count
                ));
            let final_builder =
                sort_by_last_message_and_maybe_truncate(&story_data.author_stats, truncate_limit)
                    .fold(base_builder, |builder, (author, stats)| {
                        builder
                            .newline()
                            .user(author)
                            .newline()
                            .push_line_safe(format!("Word count: {}", stats.word_count))
                            .push_line_safe(format!("Top words: {}", stats.top_words(5)))
                    });
            final_builder.build()
        }

        None => format!("Channel not initialised, use [init-channel] to add it"),
    }
}

fn get_truncate_limit(args: &mut Args) -> Option<usize> {
    // TODO: This default should be somewhere central, pluck it out of Context when needed?
    let default = Some(5);
    if args.len() > 0 {
        match args.single::<String>() {
            Ok(s) => {
                if &s == "-full" {
                    None
                } else {
                    default
                }
            }
            Err(_) => default,
        }
    } else {
        default
    }
}

#[command("show-stats")]
#[usage("<#channel name> [-full]")]
#[description("Display stats for an initialised channel by name. Returns an error if channel hasn't been initialised. If there are lots of users the results will be truncated, provide -full to show all")]
#[example("#the-fall-of-rome")]
#[only_in("guilds")] // Reminder: guild = server
async fn show_stats(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        if let Ok(channel_id) = args.single::<ChannelId>() {
            let truncate_limit = get_truncate_limit(&mut args);
            let response = get_stats(channel_id, ctx, truncate_limit).await;
            //send it
            if let Err(why) = msg.channel_id.say(&ctx.http, &response).await {
                println!("Error sending message: {:?}", why);
            }
            None
        } else {
            Some(String::from("1 Arg expected: Channel"))
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
