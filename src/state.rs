use crate::stats::WordStats;
use log::debug;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, GuildId};
use serenity::model::user::User;
use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct StoreData;

impl TypeMapKey for StoreData {
    type Value = Arc<RwLock<Store>>;
}

#[derive(Debug, Default)]
pub struct Store {
    data: HashMap<StoryKey, StoryData>,
}

pub type StoryKey = (GuildId, ChannelId);

// This could be a stable type since I intend to serialise this for disk storage.
// This doesn't seem to be an obvious Rust pattern but we could do Ocaml/Sexp style
// and use an Enum of V0,V1,...
#[derive(Debug, Default)]
pub struct StoryData {
    author_stats: HashMap<User, WordStats>,
    general_stats: WordStats,
}

impl StoryData {
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
