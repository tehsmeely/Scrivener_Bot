use crate::stats::WordStats;
use log::debug;
use serde::{Deserialize, Serialize};
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::model::user::User;
use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::sync::{Arc, RwLock};

pub struct StoreData;

impl TypeMapKey for StoreData {
    type Value = Arc<RwLock<Store>>;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    replay_needed: bool,
    queued_messages_until_replay: Vec<(StoryKey, Message)>,
    pub data: StoreInnerData,
}

type StoreInnerData = HashMap<GuildId, ServerData>;

impl Store {
    fn new(data: HashMap<GuildId, ServerData>) -> Self {
        Store {
            replay_needed: true,
            queued_messages_until_replay: Vec::new(),
            data,
        }
    }
    pub fn dump(&self) -> std::io::Result<()> {
        let mut f = File::create("state.sexp").unwrap();
        f.write_all(serde_lexpr::to_string(&self.data).unwrap().as_bytes())
    }

    pub fn load() -> serde_lexpr::error::Result<Self> {
        match File::open("state.sexp") {
            Ok(mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).unwrap();
                serde_lexpr::from_str::<StoreInnerData>(&buf).map(|data| Store::new(data))
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                serde_lexpr::error::Result::Ok(Store::default())
            }
            Err(other) => panic!("Failed opening state file: {}", other),
        }
    }

    pub fn story_keys_with_last_message(&self) -> Vec<(StoryKey, MessageId)> {
        self.data
            .iter()
            .flat_map(|(server_id, server_data)| {
                server_data
                    .channel_ids_with_last_message()
                    .iter()
                    .map(|(channel_id, message_id)| ((*server_id, *channel_id), *message_id))
                    .collect::<Vec<(StoryKey, MessageId)>>()
            })
            .collect()
    }

    pub fn get_all_channels_in_server(&self, target_server_id: &GuildId) -> Vec<ChannelId> {
        self.data
            .get(target_server_id)
            .map_or(vec![], |server| server.get_all_channel_ids())
    }

    pub fn finish_replay(&mut self) {
        let replay_queue: Vec<(StoryKey, Message)> =
            self.queued_messages_until_replay.drain(..).collect();
        for (key, message) in replay_queue {
            match self.get_channel_data_mut(&key) {
                Some(story_data) => story_data.update(&message),
                None => debug!("Message not in a channel that's been initialised"),
            }
        }
        //self.queued_messages_until_replay.clear();
        self.replay_needed = false;
    }

    pub fn process_message(&mut self, story_key: &StoryKey, message: &Message) {
        match self.replay_needed {
            true => self
                .queued_messages_until_replay
                .push((story_key.clone(), message.clone())),
            false => match self.get_channel_data_mut(&story_key) {
                Some(story_data) => story_data.update(message),
                None => debug!("Message not in a channel that's been initialised"),
            },
        }
    }

    pub fn get_unique_server_ids(&self) -> Vec<GuildId> {
        let mut guild_ids: Vec<GuildId> = self.data.keys().map(|id| id.clone()).collect();
        guild_ids.sort();
        guild_ids.dedup();
        guild_ids
    }

    pub fn get_channel_data_mut(
        &mut self,
        (server_id, channel_id): &StoryKey,
    ) -> Option<&mut ChannelData> {
        if let Some(server_data) = self.data.get_mut(server_id) {
            server_data.channels.get_mut(channel_id)
        } else {
            None
        }
    }
    pub fn get_channel_data(&self, (server_id, channel_id): &StoryKey) -> Option<&ChannelData> {
        if let Some(server_data) = self.data.get(server_id) {
            server_data.channels.get(channel_id)
        } else {
            None
        }
    }

    pub fn channel_data_exists(&self, (server_id, channel_id): &StoryKey) -> bool {
        if let Some(server_data) = self.data.get(server_id) {
            server_data.channels.contains_key(channel_id)
        } else {
            false
        }
    }

    pub fn insert_channel_data_maybe_create_server_data(
        &mut self,
        (server_id, channel_id): &StoryKey,
        channel_data: ChannelData,
    ) {
        let server_data = match self.data.get_mut(server_id) {
            Some(server_data) => server_data,
            None => {
                let server_data = ServerData::new();
                self.data.insert(*server_id, server_data);
                self.data.get_mut(server_id).unwrap()
            }
        };
        server_data.insert(channel_id, channel_data);
    }
}

pub type StoryKey = (GuildId, ChannelId);

// this could be a stable type since i intend to serialise this for disk storage.
// this doesn't seem to be an obvious rust pattern but we could do ocaml/sexp style
// and use an enum of v0,v1,...
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ChannelData {
    pub author_stats: HashMap<User, WordStats>,
    pub general_stats: WordStats,
}

impl ChannelData {
    pub fn update(&mut self, message: &Message) {
        self.general_stats.update(message);
        if let Some(word_stats) = self.author_stats.get_mut(&message.author) {
            debug!("Updating word stats for existing author");
            word_stats.update(message);
        } else {
            debug!("Inserting new word stats for new author");
            let word_stats = WordStats::new_from_message(&message);
            self.author_stats.insert(message.author.clone(), word_stats);
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServerData {
    channels: HashMap<ChannelId, ChannelData>,
}

impl ServerData {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }
    pub fn get_all_channel_ids(&self) -> Vec<ChannelId> {
        self.channels.keys().map(|x| x.clone()).collect()
    }

    pub fn channel_ids_with_last_message(&self) -> Vec<(ChannelId, MessageId)> {
        self.channels
            .iter()
            .filter_map(|(channel_id, channel_data)| {
                channel_data
                    .general_stats
                    .last_message()
                    .map(|m_id| (channel_id.clone(), m_id.clone()))
            })
            .collect()
    }

    pub fn insert(&mut self, channel_id: &ChannelId, channel_data: ChannelData) {
        self.channels.insert(*channel_id, channel_data);
    }
}
