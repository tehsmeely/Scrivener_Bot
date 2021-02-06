use crate::stats::WordStats;
use log::{debug, info};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::model::user::User;
use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::sync::mpsc::channel;
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
        let mut f = File::create("state.sexp").unwrap();
        let pretty_config = PrettyConfig::new()
            .with_depth_limit(4)
            .with_indentor("\t".to_owned());
        f.write_all(
            //ron::ser::to_string_pretty::<Self>(self, pretty_config)
            //ron::ser::to_string::<Self>(self)
            serde_lexpr::to_string(self).unwrap().as_bytes(),
        )
    }

    pub fn load() -> serde_lexpr::error::Result<Self> {
        match File::open("state.sexp") {
            Ok(mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).unwrap();
                println!("{:#?}", buf);
                //ron::de::from_str(&buf)
                serde_lexpr::from_str(&buf)
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
            .filter_map(|(story_key, story_data)| {
                story_data
                    .general_stats
                    .last_message()
                    .map(|m_id| (story_key.clone(), m_id))
            })
            .collect()
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
