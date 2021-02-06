mod language_parsing;
mod state;
mod stats;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group, hook},
    Args, CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::model::prelude::*;

use crate::state::{Store, StoreData, StoryData, StoryKey};
use log::{debug, info, warn, LevelFilter};
use serenity::http::Http;
use serenity::static_assertions::_core::sync::atomic::AtomicBool;
use serenity::utils::MessageBuilder;
use simplelog::{Config, SimpleLogger};
use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use tokio::time::Duration;

#[group]
#[commands(ping, init_channel, show_stats)]
struct General;

struct Handler {
    tasks_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    // We use the cache_ready event just in case some cache operation is required in whatever use
    // case you have for this.
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("Cache built successfully!");
        if !self.tasks_running.load(Ordering::Relaxed) {
            store_replay(&ctx).await;
            let ctx = Arc::new(ctx);
            let ctx1 = Arc::clone(&ctx);
            let _ = tokio::spawn(async move {
                dictionary_update_worker(ctx1).await;
            });
            let ctx2 = Arc::clone(&ctx);
            let _ = tokio::spawn(async move {
                dump_state(ctx2).await;
            });
            self.tasks_running.swap(true, Ordering::Relaxed);
        }
    }
}

async fn store_replay(ctx: &Context) {
    let story_keys_with_last_message = {
        let store_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StoreData>()
                .expect("Expected StoryData in TypeMap.")
                .clone()
        };
        let mut store = store_lock.read().unwrap();
        store.story_keys_with_last_message()
    };
    info!("initialising store! {:#?}", story_keys_with_last_message);
    let mut new_messages = HashMap::<StoryKey, Vec<Message>>::new();
    for (story_key, last_message_id) in story_keys_with_last_message.into_iter() {
        let (_, channel_id) = story_key;
        let channel_name = channel_id.name(&ctx.cache).await.unwrap();
        info!("Checking for missed messages in {}", channel_name);
        let msgs = channel_id
            .messages(&ctx.http, |get_messages_builder| {
                get_messages_builder.after(last_message_id).limit(50)
            })
            .await
            .unwrap();
        info!("Got messages: {:?}", msgs);
        if msgs.len() > 0 {
            new_messages.insert(story_key, msgs);
        }
    }
    info!(
        "Retrieved {} messages across all channels to populate:",
        new_messages.len()
    );
    let store_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<StoreData>()
            .expect("Expected StoryData in TypeMap.")
            .clone()
    };
    let mut store = store_lock.write().unwrap();
    for (story_key, messages) in new_messages {
        //Since we got these messages from the store, we can expect the key to exist
        let story_data = store.data.get_mut(&story_key).unwrap();
        for message in messages {
            story_data.update(&message);
        }
    }
    info!("Finished initialising");
}

async fn dictionary_update_worker(_ctx: Arc<Context>) {
    loop {
        println!("Dictionary update!");
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

async fn dump_state(ctx: Arc<Context>) {
    loop {
        println!("Dumping state!");
        {
            let store_lock = {
                let data_read = ctx.data.read().await;
                data_read
                    .get::<StoreData>()
                    .expect("Expected StoryData in TypeMap.")
                    .clone()
            };
            let store = store_lock.read().unwrap();
            store.dump().unwrap();
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

#[tokio::main]
async fn main() {
    let _ = SimpleLogger::init(LevelFilter::Info, Config::default());
    log::info!("Testing log: Info");
    log::debug!("Testing log: Debug");
    log::warn!("Testing log: Warn");
    // Login with a bot token from the environment
    let token = env::var("BOT_TOKEN").expect("token");
    let http = Http::new_with_token(&token);
    let app_info = http.get_current_application_info().await.unwrap();
    println!("{:#?}", app_info);
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!").on_mention(Some(app_info.id)))
        .normal_message(on_regular_message)
        .group(&GENERAL_GROUP);
    let mut client = Client::builder(&token)
        .event_handler(Handler {
            tasks_running: AtomicBool::new(false),
        })
        .framework(framework)
        .await
        .expect("Error creating client");

    // Insert the global data:
    {
        let mut data = client.data.write().await;
        let store = match Store::load() {
            Ok(store) => store,
            Err(e) => {
                println!("Parse failed: {:#?}", e);
                panic!(e)
            }
        };
        data.insert::<StoreData>(Arc::new(RwLock::new(store)));
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

async fn update_stats_if_exist(story_key: StoryKey, ctx: &Context, message: &Message) {
    let store_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<StoreData>()
            .expect("Expected StoryData in TypeMap.")
            .clone()
    };
    let mut store = store_lock.write().unwrap();
    match store.data.get_mut(&story_key) {
        Some(mut story_data) => story_data.update(message),
        None => debug!("Message not in a channel that's been initialised"),
    }
}

#[hook]
async fn on_regular_message(ctx: &Context, message: &Message) {
    //Update a stats if this channel is initialised
    if let Some(server_id) = message.guild_id {
        let story_key = (server_id, message.channel_id);
        update_stats_if_exist(story_key, ctx, message).await;
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
        store.data.contains_key(&story_key)
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
                .await
                .unwrap();
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
        let mut store = store_lock.write().unwrap();
        store.data.insert(story_key, story_data);
    };

    Ok(())
}

#[command("init-channel")]
async fn init_channel(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        if let Ok(channel_to_init) = args.single::<String>() {
            if let Some(text_channel) = get_text_channel(ctx, &server_id, &channel_to_init).await {
                match actually_init_channel(text_channel, ctx).await {
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

trait MessageBuilderExt {
    fn newline(&mut self) -> &mut Self;
}

impl MessageBuilderExt for MessageBuilder {
    fn newline(&mut self) -> &mut Self {
        self.push("\n")
    }
}
