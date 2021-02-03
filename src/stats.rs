use chrono::{DateTime, Utc};
use log::{debug, info};
use serenity::model::channel::Message;
use serenity::model::id::MessageId;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct WordStats {
    word_count: usize,
    word_frequencies: HashMap<String, usize>,
    last_message: (MessageId, DateTime<Utc>),
    included_messages: HashSet<MessageId>,
}

impl WordStats {
    pub fn new_from_message(message: &Message) -> Self {
        let mut t = Self::default();
        t.update(message);
        t
    }
    pub fn update(&mut self, message: &Message) {
        if !self.included_messages.contains(&message.id) {
            let words = crate::language_parsing::tokenise(&message.content);
            debug!("Parsed {} words from message {}", words.len(), message.id);
            self.word_count += words.len();
            for word_ in words {
                let word = word_.to_lowercase().to_string();
                if let Some(mut existing_count) = self.word_frequencies.get_mut(&word) {
                    existing_count += 1;
                } else {
                    self.word_frequencies.insert(word, 1);
                }
            }
            self.included_messages.insert(message.id);
            if message.timestamp > self.last_message[1] {
                self.last_message = (message.id, message.timestamp);
            }
        }
    }
}
