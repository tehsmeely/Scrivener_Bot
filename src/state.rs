use crate::stats::WordStats;
use crate::utils::iterators::helpers::sort_by_last_message_and_maybe_truncate;
use crate::utils::trait_extensions::MessageBuilderExt;
use log::debug;
use serde::{Deserialize, Serialize};
use serenity::model::channel::{GuildChannel, Message};
use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};
use serenity::model::prelude::Channel;
use serenity::model::user::User;
use serenity::prelude::TypeMapKey;
use serenity::utils::MessageBuilder;
use std::collections::{HashMap, HashSet};
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
    pub initialising_channels: HashSet<StoryKey>,
    pub data: StoreInnerData,
}

type StoreInnerData = HashMap<GuildId, ServerData>;

const STATE_FILENAME: &str = "state.sexp";

impl Store {
    fn new(data: HashMap<GuildId, ServerData>) -> Self {
        Store {
            replay_needed: true,
            queued_messages_until_replay: Vec::new(),
            initialising_channels: HashSet::new(),
            data,
        }
    }
    pub fn dump(&self) -> serde_pickle::error::Result<()> {
        let tmp_file = "state.pickle.tmp";
        let mut f = File::create(tmp_file).unwrap();
        let serialise_result = serde_pickle::to_writer(&mut f, &self.data, true);
        if serialise_result.is_ok() {
            std::fs::rename(tmp_file, STATE_FILENAME)?;
        }
        serialise_result
    }

    pub fn load() -> serde_pickle::error::Result<Self> {
        match File::open(STATE_FILENAME) {
            Ok(mut f) =>
            //ron::de::from_reader::<_, StoreInnerData>(f).map(|data| Store::new(data)),
            {
                //bincode::deserialize_from(f).map(|data| Store::new(data))
                serde_pickle::from_reader(f).map(|data| Store::new(data))
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                serde_pickle::error::Result::Ok(Store::default())
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

    pub fn get_server_data_mut(&mut self, server_id: &GuildId) -> Option<&mut ServerData> {
        self.data.get_mut(server_id)
    }
    pub fn get_server_data(&self, server_id: &GuildId) -> Option<&ServerData> {
        self.data.get(server_id)
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

    pub fn make_stats_string(
        &self,
        text_channel: &GuildChannel,
        truncate_limit: Option<usize>,
    ) -> String {
        let mut stats_iterator =
            sort_by_last_message_and_maybe_truncate(&self.author_stats, truncate_limit);
        let mut builder = MessageBuilder::new();
        let base_builder = builder
            .push("For ")
            .channel(text_channel)
            .newline()
            .push_bold_line("General")
            .push_line_safe(format!(
                "Word count: {}",
                self.general_stats.word_count
            ))
            .apply_if(stats_iterator.is_truncated(), |mb|
                mb.newline().push_line(
                    format!("Not all authors are displayed below, just the {} most recent ones. Add [-full] to see all of them",
                            stats_iterator.limit())
                )
            );
        let final_builder = stats_iterator.fold(base_builder, |builder, (author, stats)| {
            builder
                .newline()
                .user(author)
                .newline()
                .push_line_safe(format!("Word count: {}", stats.word_count))
                .push_line_safe(format!("Top words: {}", stats.top_words(10)))
        });
        final_builder.build()
    }
    pub fn get_user(&self, user: &User) -> Option<&WordStats> {
        self.author_stats.get(user)
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

    // Returns the sorted list of channel ids for a given user.
    pub fn channel_ids_by_wordcount_for_user(&self, user: &User) -> Vec<(ChannelId, usize)> {
        // Todo: Enable -recent- word count by supporting it in stats
        let mut channels_by_wordcount: Vec<(ChannelId, usize)> = self
            .channels
            .iter()
            .filter_map(|(channel_id, channel_data)| {
                channel_data
                    .get_user(user)
                    .map(|stats| (channel_id.clone(), stats.word_count.clone()))
            })
            .collect();
        channels_by_wordcount.sort_by_key(|(_id, count)| *count);
        channels_by_wordcount.reverse();
        channels_by_wordcount
    }
    pub fn make_user_stats_string(
        user: &User,
        channels_by_wordcount: Vec<(Channel, usize)>,
    ) -> String {
        let mut builder = MessageBuilder::new();
        if channels_by_wordcount.len() == 0 {
            builder
                .user(user)
                .push(" has no recorded activity in any initialised channels")
                .build()
        } else {
            let max = std::cmp::min(5, channels_by_wordcount.len());
            builder
                .push("Top ")
                .push(max)
                .push(" channels on this server for: ")
                .user(user)
                .newline()
                .push_line("By Wordcount for all time:");
            let mut i = 0;
            for (channel, word_count) in channels_by_wordcount.iter() {
                builder
                    .push(i + 1)
                    .push(": ")
                    .channel(channel)
                    .push(" -> ")
                    .push(word_count)
                    .newline();
                i += 1;
                if i >= max {
                    break;
                }
            }
            builder.build()
        }
    }
}
