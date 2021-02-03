mod language_parsing;
mod state;
mod stats;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group},
    Args, CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::model::prelude::*;

use crate::state::{Store, StoreData, StoryData, StoryKey};
use log::info;
use serenity::http::Http;
use serenity::prelude::RwLock;
use std::collections::{HashMap, HashSet};
use std::env;
use std::future::Future;
use std::sync::Arc;

#[group]
#[commands(ping, init_channel)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let token = env::var("BOT_TOKEN").expect("token");
    let http = Http::new_with_token(&token);
    let app_info = http.get_current_application_info().await.unwrap();
    println!("{:#?}", app_info);
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!").on_mention(Some(app_info.id)))
        .group(&GENERAL_GROUP);
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // Insert the global data:
    {
        let mut data = client.data.write().await;
        data.insert::<StoreData>(Arc::new(RwLock::new(Store::default())));
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}

async fn actually_init_channel(
    text_channel: GuildChannel,
    ctx: &Context,
) -> std::result::Result<(), String> {
    //Fetch from store, if exists, refuse
    // See example https://github.com/serenity-rs/serenity/blob/current/examples/e12_global_data/src/main.rs
    let story_key: StoryKey = (text_channel.guild_id, text_channel.id);
    let story_data_exists = {
        let store_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StoreData>()
                .expect("Expected StoryData in TypeMap.")
                .clone()
        };
        let store = store_lock.read().unwrap();
        store.contains_key(&story_key)
    };
    if story_data_exists {
        return Err(format!(
            "The channel {} (id: {}) is already initialised",
            text_channel.name, text_channel.id
        ));
    }
    let mut story_data = StoryData::default();
    info!(
        "Creating new story data for server_id {}, channel id {}",
        text_channel.guild_id, text_channel.id
    );
    if let Some(mut last_msg_id) = text_channel.last_message_id {
        //Keep populating back in time until all messages are fetched
        let oldest_message = chrono::Utc::now();
        let mut fetched_messages = 0;
        loop {
            let messages: Vec<Message> = text_channel
                .messages(&ctx.http, |get_messages_builder| {
                    get_messages_builder.before(last_msg_id).limit(100)
                })
                .await?;
            if messages.len() == 0 {
                break;
            } else {
                fetched_messages += messages.len();
                // I don't know if the older message will be first or last.
                for message in messages {
                    if message.timestamp < oldest_message {
                        last_msg_id = message.id
                    }
                    story_data.update(&message);
                }
                info!("Processed {} messages so far...", fetched_messages)
            }
        }
    }

    //Insert story_data into store
    {
        let store_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StoreData>()
                .expect("Expected StoryData in TypeMap.")
                .clone()
        };
        let store = store_lock.write().await;
        store.insert(&story_key, story_data);
    };

    Ok(())
}

#[command("init-channel")]
async fn init_channel(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        if let Ok(channel_to_init) = args.single::<String>() {
            if let Some(text_channel) = get_text_channel(ctx, &server_id, &channel_to_init).await {
                match actually_init_channel(text_channel, ctx) {
                    Ok(()) => format!("Story stats initialised for {}", channel_to_init),
                    Err(error_string) => format!("Not initialised: {}", error_string),
                }
            } else {
                format!("No text channel found with name: {}", channel_to_init)
            }
        } else {
            String::from("1 Arg expected: String: Channel name")
        }
    } else {
        String::from("BUG: message had no server id, bot only supports server text channels")
    };
    msg.reply(ctx, reply).await?;
    Ok(())
}

async fn get_text_channel(
    ctx: &Context,
    server_id: &GuildId,
    target: &str,
) -> Option<GuildChannel> {
    let channels = server_id.channels(&ctx.http).await.unwrap();
    channels
        .values()
        .find(
            |server_channel| match (server_channel.kind, &server_channel.name) {
                (ChannelType::Text, name) if name == target => true,
                _ => false,
            },
        )
        .map(|gc| gc.clone())
}
