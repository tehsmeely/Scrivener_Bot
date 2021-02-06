use crate::commands::helpers::get_text_channel;
use crate::state::{StoreData, StoryKey};
use crate::MessageBuilderExt;
use log::error;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::http::AttachmentType;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::fs::File;
use std::ops::Add;
use std::path::Path;
use std::time::Duration;
use tokio::io::ErrorKind;
use uuid::Uuid;

//TODO: Put these in config inside Context
const REQUEST_PATH: &str =
    "D:\\Library\\Documents\\rust\\StoryStatsWatcher\\wordcloud\\working\\in";
const IMAGE_PATH: &str = "D:\\Library\\Documents\\rust\\StoryStatsWatcher\\wordcloud\\working\\out";

#[command("gen-wordcloud")]
async fn gen_wordcloud(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        if let Ok(channel_to_init) = args.single::<String>() {
            if let Some(text_channel) = get_text_channel(ctx, &server_id, &channel_to_init).await {
                if let Ok(username) = args.single::<String>() {
                    msg.reply(ctx, "Attempting to generate a wordcloud")
                        .await
                        .unwrap();
                    let story_key = (server_id, text_channel.id);
                    request_and_fetch_wordcloud(&story_key, ctx, &msg.channel_id, &username).await
                } else {
                    Some(format!("Arg expected: String: username"))
                }
            } else {
                Some(format!(
                    "No text channel found with name: {}",
                    channel_to_init
                ))
            }
        } else {
            Some(String::from("1 Arg expected: String: Channel name"))
        }
    } else {
        Some(String::from(
            "BUG: message had no server id, bot only supports server text channels",
        ))
    };
    if let Some(reply) = reply {
        msg.reply(ctx, reply).await?;
    }
    Ok(())
}

async fn request_and_fetch_wordcloud(
    story_key: &StoryKey,
    ctx: &Context,
    send_to_channel: &ChannelId,
    user: &str,
) -> Option<String> {
    //Look up a specific user's frequencies in WordStats, dump to specific file, watch for response from the worker
    let users_stats = {
        let store_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StoreData>()
                .expect("Expected StoryData in TypeMap.")
                .clone()
        };
        let store = store_lock.read().unwrap();
        if let Some(story_data) = store.data.get(story_key) {
            let mut res = None;
            for (author, stats) in story_data.author_stats.iter() {
                if author.name == user {
                    res = Some(stats.word_frequencies.clone())
                }
            }
            res
        } else {
            return Some(format!("Channel not initialised"));
        }
    };
    if let Some(word_freqs) = users_stats {
        let request_uuid = Uuid::new_v4();
        let request_filename = format!("{}.generate.json", request_uuid);
        let request_path = format!("{}\\{}", REQUEST_PATH, request_filename);
        let expect_image_filename = format!("{}.generated.png", request_uuid);
        {
            let outfile = File::create(request_path).unwrap();
            serde_json::to_writer(&outfile, &word_freqs).unwrap();
        }
        //Get resulting image in ... some .. time
        let expecting_path_str = format!("{}\\{}", IMAGE_PATH, expect_image_filename);
        let expecting_path = Path::new(&expecting_path_str);
        let image_path = wait_for_image(&expecting_path).await;
        match image_path {
            Ok(()) => {
                let file = tokio::fs::File::from_std(File::open(expecting_path).unwrap());
                let files = vec![AttachmentType::File {
                    file: &file,
                    filename: expecting_path_str,
                }];
                send_to_channel
                    .send_files(&ctx.http, files, |create_message| {
                        create_message.content("Here's your wordcloud!")
                    })
                    .await
                    .unwrap();
                None
            }
            Err(e) => Some(format!("Failed creating image: {}", e)),
        }
    } else {
        Some(format!("User not found in channel"))
    }
}

async fn wait_for_image(expecting_path: &Path) -> tokio::io::Result<()> {
    //Wait for it to exist for up to [timeout]
    let mut elapsed_time = Duration::new(0, 0);
    let wait_time = Duration::from_millis(300);
    let timeout = Duration::from_secs(2);
    while elapsed_time < timeout {
        if expecting_path.exists() {
            return Ok(());
        }
        elapsed_time.add(wait_time);
        tokio::time::sleep(wait_time).await;
    }
    Err(tokio::io::Error::new(
        ErrorKind::Other,
        "Timed out waiting for file to appear",
    ))
}
