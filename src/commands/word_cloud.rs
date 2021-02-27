use crate::config::GeneralAppConfigData;
use crate::state::{StoreData, StoryKey};
use crate::utils::trait_extensions::MessageBuilderExt;
use log::error;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::http::AttachmentType;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::iter::FromIterator;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::io::ErrorKind;
use uuid::Uuid;

#[derive(Debug, EnumIter)]
enum MaskName {
    Bunny,
    D20,
    Shield,
    Wolf,
    Horse,
}
impl MaskName {
    fn to_str(&self) -> &'static str {
        match self {
            Self::Bunny => "bunny",
            Self::D20 => "d20",
            Self::Shield => "shield",
            Self::Wolf => "wolf",
            Self::Horse => "horse",
        }
    }
}
impl FromStr for MaskName {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "bunny" => Ok(Self::Bunny),
            "d20" => Ok(Self::D20),
            "shield" => Ok(Self::Shield),
            "wolf" => Ok(Self::Wolf),
            "horse" => Ok(Self::Horse),
            _ => Err(format!("Invalid mask name {}", s)),
        }
    }
}

fn error_help_text(error: &impl Display) -> String {
    format!("ERROR: Invalid Arguments: {}", error)
}
fn parse_args(
    args: &mut Args,
) -> std::result::Result<(ChannelId, Option<UserId>, Option<MaskName>), String> {
    match args.len() {
        1 => match args.single::<ChannelId>() {
            Ok(channel_name) => Ok((channel_name, None, None)),
            Err(e) => Err(error_help_text(&e)),
        },
        2 => {
            let maybe_channel_name = args.single::<ChannelId>();
            let maybe_user = args.single::<UserId>();
            match (maybe_channel_name, maybe_user) {
                (Ok(channel_name), Ok(user)) => Ok((channel_name, Some(user), None)),
                (Ok(channel_name), Err(_user_name_error)) => {
                    let maybe_mask = args.single::<MaskName>();
                    match maybe_mask {
                        Ok(mask_name) => Ok((channel_name, None, Some(mask_name))),
                        Err(_mask_error) => {
                            println!("username: {}, mask: {}", _user_name_error, _mask_error);
                            Err(error_help_text(&String::from("If only two are provided, second argument should be user mention or mask_name, this was neither")))
                        }
                    }
                }
                (Err(channel_name_error), _) => Err(error_help_text(&channel_name_error)),
            }
        }
        3 => {
            let maybe_channel_name = args.single::<ChannelId>();
            let maybe_user = args.single::<UserId>();
            let maybe_mask_name = args.single::<MaskName>();
            match (maybe_channel_name, maybe_user, maybe_mask_name) {
                (Ok(channel_name), Ok(user), Ok(mask_name)) => {
                    Ok((channel_name, Some(user), Some(mask_name)))
                }
                (Ok(_), Ok(_), Err(mask_name_error)) => Err(error_help_text(&mask_name_error)),
                (Ok(_), Err(user_name_error), _) => Err(error_help_text(&user_name_error)),
                (Err(channel_name_error), _, _) => Err(error_help_text(&channel_name_error)),
            }
        }
        _ => Err(String::from("Invalid number of args")),
    }
}

async fn wordcloud_is_enabled(ctx: &Context) -> bool {
    let config_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<GeneralAppConfigData>()
            .expect("Expected GeneralAppConfigData in TypeMap.")
            .clone()
    };
    let is_enabled = config_lock.read().unwrap().wordcloud_config.is_some();
    is_enabled
}

fn unicode_emoji(s: &str) -> ReactionType {
    ReactionType::Unicode(String::from(s))
}
async fn react_or_reply(msg: &Message, ctx: &Context) {
    match msg.react(ctx, unicode_emoji("🤖")).await {
        Ok(_) => {
            let _ = msg.react(ctx, unicode_emoji("⌚")).await.unwrap();
        }
        Err(_) => {
            let _ = msg.reply(ctx, "Making wordcloud...").await.unwrap();
        }
    }
}
#[command("gen-wordcloud")]
#[usage("<#channel name> [<@user mention>] [mask]")]
#[description(
    "Generate a wordcloud from the given channel's general stats. If a user is given (via @mention) the wordcloud if for just that user's stats. Available masks: bunny|d20|shield|wolf|horse"
)]
#[example("#war-and-peace")]
#[example("#the-fall-of-rome @Caligula wolf")]
#[example("#the-fall-of-rome wolf")]
#[bucket("global-wordcloud-bucket")]
#[only_in("guilds")] // Reminder: guild = server
async fn gen_wordcloud(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if wordcloud_is_enabled(ctx).await {
        match parse_args(&mut args) {
            Ok((channel_id, user_id, mask_name)) => {
                if let Some(server_id) = msg.guild_id {
                    react_or_reply(msg, ctx).await;
                    let story_key = (server_id, channel_id);
                    request_and_fetch_wordcloud(
                        &story_key,
                        ctx,
                        &msg.channel_id,
                        &user_id,
                        &mask_name,
                    )
                    .await
                } else {
                    Some(String::from(
                        "BUG: message had no server id, bot only supports server text channels",
                    ))
                }
            }
            Err(parse_command_error) => Some(parse_command_error),
        }
    } else {
        some_string!("Wordclouds are not enabled, sorry - Speak to your bot admin")
    };
    if let Some(reply_) = reply {
        msg.reply(ctx, reply_).await?;
    }
    Ok(())
}

async fn request_and_fetch_wordcloud(
    story_key: &StoryKey,
    ctx: &Context,
    send_to_channel: &ChannelId,
    user: &Option<UserId>,
    mask_name: &Option<MaskName>,
) -> Option<String> {
    //Look up a specific user's frequencies in WordStats, dump to specific file, watch for response from the worker
    let response_content = MessageBuilder::new()
        .push("Word cloud for channel ")
        .channel(story_key.1)
        .apply_if(user.is_some(), |b| b.push(" for user ").user(user.unwrap()))
        .build();
    let users_stats = {
        let store_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StoreData>()
                .expect("Expected StoryData in TypeMap.")
                .clone()
        };
        let store = store_lock.read().unwrap();
        if let Some(story_data) = store.get_channel_data(story_key) {
            let mut res: Option<HashMap<String, usize>> = None;
            match user {
                Some(user_id) => {
                    for (author, stats) in story_data.author_stats.iter() {
                        if &author.id == user_id {
                            res = Some(stats.filtered_word_frequencies())
                        }
                    }
                }
                None => res = Some(story_data.general_stats.filtered_word_frequencies()),
            }
            res
        } else {
            return Some(format!("Channel not initialised"));
        }
    };
    if let Some(word_freqs) = users_stats {
        let request_uuid = Uuid::new_v4();
        let mask_name_str = match mask_name {
            Some(mask_name) => format!("{}.", mask_name.to_str()),
            None => String::from(""),
        };
        let request_filename =
            PathBuf::from(format!("{}.{}generate.json", request_uuid, mask_name_str));
        let expect_image_filename = PathBuf::from(format!("{}.generated.png", request_uuid));
        let (generated_image_path, request_path, timeout) = {
            let config_lock = {
                let data_read = ctx.data.read().await;
                data_read
                    .get::<GeneralAppConfigData>()
                    .expect("Expected StoryData in TypeMap.")
                    .clone()
            };
            let config = config_lock.read().unwrap();
            let wordcloud_config = config.wordcloud_config.as_ref().unwrap();
            let generated_image_path = wordcloud_config
                .generated_image_path
                .join(expect_image_filename);
            let request_path = wordcloud_config.request_path.join(request_filename);
            (generated_image_path, request_path, wordcloud_config.timeout)
        };
        {
            let outfile = File::create(request_path).unwrap();
            serde_json::to_writer(&outfile, &word_freqs).unwrap();
        }
        let image_arrived = wait_for_image(&generated_image_path, &timeout).await;
        match image_arrived {
            Ok(()) => {
                let file = tokio::fs::File::from_std(File::open(&generated_image_path).unwrap());
                let files = vec![AttachmentType::File {
                    file: &file,
                    filename: String::from("wordcloud.png"),
                }];
                send_to_channel
                    .send_files(&ctx.http, files, |create_message| {
                        create_message.content(response_content)
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

async fn wait_for_image(expecting_path: &Path, timeout: &Duration) -> tokio::io::Result<()> {
    //Wait for it to exist for up to [timeout]
    let mut elapsed_time = Duration::new(0, 0);
    let wait_time = Duration::from_millis(300);
    let tiny_wait = Duration::from_millis(60);
    while &elapsed_time <= timeout {
        if expecting_path.exists() {
            //Tiny wait to try and account for non-atomic file creation
            //sometimes we're a little keen to use the file when it's created but it's not ready yet
            tokio::time::sleep(tiny_wait).await;
            return Ok(());
        }
        elapsed_time += wait_time;
        tokio::time::sleep(wait_time).await;
    }
    Err(tokio::io::Error::new(
        ErrorKind::Other,
        format!("Timed out waiting for file to appear ({:.1}s)", timeout.as_secs_f32()),
    ))
}

#[test]
fn usage_matches_all_masks() {
    //commands::word_cloud::GEN_WORDCLOUD_COMMAND;
    let desc: &str = GEN_WORDCLOUD_COMMAND_OPTIONS.desc.unwrap();
    let match_ = "Available masks: ";
    let masks_index = desc.rfind(match_).unwrap();
    let (_, masks_from_desc) = desc.split_at(masks_index + match_.len());
    let all_masks_from_enum_iter: String = MaskName::iter()
        .map(|mn| String::from(mn.to_str()))
        .collect::<Vec<String>>()
        .join("|");
    assert_eq!(all_masks_from_enum_iter, masks_from_desc);
}
