use chrono::{DateTime, Utc};
use log::{debug, info};
use serenity::model::channel::Message;
use serenity::model::id::MessageId;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

#[derive(Debug, Default)]
pub struct WordStats {
    pub word_count: usize,
    pub word_frequencies: HashMap<String, usize>,
    last_message: Option<(MessageId, DateTime<Utc>)>,
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
                    *existing_count += 1;
                } else {
                    self.word_frequencies.insert(word, 1);
                }
            }
            self.included_messages.insert(message.id);
            if let Some((_, last_message_time)) = self.last_message {
                if message.timestamp > last_message_time {
                    self.last_message = Some((message.id, message.timestamp));
                }
            }
        }
    }

    pub fn top_words(&self, n: usize) -> String {
        // This is a bit gross considering the possible size of [word_frequencies] but this is due
        // a major overhaul and that HashMap will be replaced by some efficient Summary type soon
        // and this whole function will need redoing then anyway
        let mut word_vec = Vec::from_iter(self.word_frequencies.iter());
        word_vec.sort_by_key(|(_, count)| *count);
        let mut sorted_words: Vec<String> =
            word_vec.iter().map(|(word, _)| (*word).clone()).collect();
        sorted_words.reverse();
        let result_len = if sorted_words.len() < n {
            sorted_words.len()
        } else {
            n
        };
        let top_words = sorted_words.get(0..result_len).unwrap();
        top_words.join(", ")
    }
}
