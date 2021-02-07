use crate::commands::helpers::get_text_channel;
use crate::state::{StoreData, StoryKey};
use crate::MessageBuilderExt;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

async fn get_stats(text_channel: GuildChannel, ctx: &Context) -> String {
    let story_key: StoryKey = (text_channel.guild_id, text_channel.id);
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
                .channel(text_channel.id)
                .newline()
                .push_bold_line("General")
                .push_line_safe(format!(
                    "Word count: {}",
                    story_data.general_stats.word_count
                ));
            let final_builder =
                story_data
                    .author_stats
                    .iter()
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

        None => format!("Channel not initialised, call [init-channel] to add it"),
    }
}

#[command("show-stats")]
#[usage("<channel name>")]
#[description("Display stats for an initialised channel by name. Returns an error if channel hasn't been initialised")]
#[example("the-fall-of-rome")]
#[only_in("guilds")] // Reminder: guild = server
async fn show_stats(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        if let Ok(channel_name) = args.single::<String>() {
            if let Some(text_channel) = get_text_channel(ctx, &server_id, &channel_name).await {
                let response = get_stats(text_channel, ctx).await;
                //send it`
                if let Err(why) = msg.channel_id.say(&ctx.http, &response).await {
                    println!("Error sending message: {:?}", why);
                }
                None
            } else {
                Some(format!("No text channel found with name: {}", channel_name))
            }
        } else {
            Some(String::from("1 Arg expected: String: Channel name"))
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
