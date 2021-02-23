use crate::state::{ChannelData, StoreData, StoryKey};
use log::info;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

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
        store.channel_data_exists(&story_key)
    };
    if story_data_exists {
        return Err(format!(
            "The channel {} (id: {}) is already initialised",
            text_channel.name, text_channel.id
        ));
    }
    let mut channel_data = ChannelData::default();
    info!(
        "Creating new story data for server_id {}, channel id {}",
        text_channel.guild_id, text_channel.id
    );
    if let Some(mut last_msg_id) = text_channel.last_message_id {
        //Keep populating back in time until all messages are fetched
        let oldest_message = chrono::Utc::now();
        let mut fetched_messages = 0;
        {
            //Fetch the last_msg_id itself, or we miss it by just jumping in with [before(id)]
            let last_msg = text_channel.message(&ctx.http, last_msg_id).await.unwrap();
            channel_data.update(&last_msg);
        }
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
                    channel_data.update(&message);
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
        store.insert_channel_data_maybe_create_server_data(&story_key, channel_data);
    };

    Ok(())
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
            let _ = msg
                .reply(ctx, "Initialising channel, bear with me...")
                .await
                .unwrap();
        }
    }
}

// TODO: Load these from config
const ALLOWED_ROLES: [&str; 3] = ["MasterScrivener", "ScrivMaster", "ScrivAdmin"];

#[command("init-channel")]
#[usage("<#channel name>")]
#[description("Initialise a channel to generate stats for. Will backpopulate from existing messages and keep an eye out for future ones")]
#[example("#the-fall-of-rome")]
#[only_in("guilds")] // Reminder: guild = server
async fn init_channel(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let reply = if let Some(server_id) = msg.guild_id {
        match author_is_in_allowed_roles(ctx, &server_id, &msg.author).await {
            true => {
                if let Ok(channel_to_init) = args.single::<ChannelId>() {
                    //Safely assuming we can convert to a guild channel considering the [only_in] constraint
                    let channel = channel_to_init
                        .to_channel(&ctx)
                        .await
                        .unwrap()
                        .guild()
                        .unwrap();

                    if channel.kind == ChannelType::Text {
                        react_or_reply(msg, ctx).await;
                        match actually_init_channel(channel, ctx).await {
                            Ok(()) => format!("Story stats initialised for {}", channel_to_init),
                            Err(error_string) => format!("Not initialised: {}", error_string),
                        }
                    } else {
                        format!("Channel is not a text-channel, can only init normal text channels")
                    }
                } else {
                    String::from("1 Arg expected: String: Channel name")
                }
            }
            false => format!(
                "This command is only available to those with the role {}",
                ALLOWED_ROLES[0]
            ),
        }
    } else {
        String::from("BUG: message had no server id, bot only supports server text channels")
    };
    msg.reply(ctx, reply).await?;
    Ok(())
}

async fn author_is_in_allowed_roles(ctx: &Context, server_id: &GuildId, user: &User) -> bool {
    let partial_guild = server_id.to_partial_guild(ctx).await.unwrap();
    for role_name in ALLOWED_ROLES.iter() {
        if let Some(role) = partial_guild.role_by_name(role_name) {
            if let Ok(true) = user.has_role(ctx, *server_id, role).await {
                return true;
            }
        }
    }
    //Either found no roles or user did not have them (or a mix)
    false
}
