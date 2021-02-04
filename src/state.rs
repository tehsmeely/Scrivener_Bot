use crate::stats::WordStats;
use log::debug;
use serde::{Deserialize, Serialize};
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, GuildId};
use serenity::model::user::User;
use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};

pub struct StoreData;

impl TypeMapKey for StoreData {
    type Value = Arc<RwLock<Store>>;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    pub data: HashMap<StoryKey, StoryData>,
}

pub type StoryKey = (GuildId, ChannelId);

impl Store {
    pub fn dump(&self) -> std::io::Result<()> {
        let mut f = File::create("state.ron").unwrap();
        f.write_all(ron::ser::to_string::<Self>(self).unwrap().as_bytes())
    }

    pub fn load() -> ron::error::Result<Self> {
        let mut f = File::open("state.ron").unwrap();
        //Handle the case of no state file, this is reasonable to expect!
        ron::de::from_reader(f)
    }
}

// this could be a stable type since i intend to serialise this for disk storage.
// this doesn't seem to be an obvious rust pattern but we could do ocaml/sexp style
// and use an enum of v0,v1,...
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StoryData {
    pub author_stats: HashMap<User, WordStats>,
    pub general_stats: WordStats,
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
