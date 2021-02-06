use chrono::{DateTime, Utc};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serenity::model::channel::Message;
use serenity::model::id::MessageId;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

#[derive(Debug, Default, Serialize, Deserialize)]
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
            info!("Wordstats update. message: {:?}", message);
            let words = crate::language_parsing::tokenise(&message.content);
            debug!("Parsed {} words from message {}", words.len(), message.id);
            self.word_count += words.len();
            for word_ in words {
                let word = word_.to_lowercase().to_string();
                if let Some(existing_count) = self.word_frequencies.get_mut(&word) {
                    *existing_count += 1;
                } else {
                    self.word_frequencies.insert(word, 1);
                }
            }
            self.included_messages.insert(message.id);
            let should_update_last_message = match self.last_message {
                None => true,
                Some((_, last_message_time)) => message.timestamp > last_message_time,
            };
            if should_update_last_message {
                self.last_message = Some((message.id, message.timestamp));
            }
        } else {
            info!(
                "Wordstats did not update, message seen.\nmessage: {:?}",
                message
            );
        }
    }

    pub fn top_words(&self, n: usize) -> String {
        // This is a bit gross considering the possible size of [word_frequencies] but this is due
        // a major overhaul and that HashMap will be replaced by some efficient Summary type soon
        // and this whole function will need redoing then anyway
        let mut word_vec = Vec::from_iter(self.word_frequencies.iter());
        word_vec.sort_by_key(|(_, count)| *count);
        let mut sorted_words: Vec<String> = word_vec
            .iter()
            .filter_map(|(word, _)| {
                // That ref ref deref deref deref is ... ugly ...
                if STOP_WORDS.contains(&&***word) {
                    None
                } else {
                    Some((*word).clone())
                }
            })
            .collect();
        sorted_words.reverse();
        let result_len = if sorted_words.len() < n {
            sorted_words.len()
        } else {
            n
        };
        let top_words = sorted_words.get(0..result_len).unwrap();
        top_words.join(", ")
    }

    pub fn last_message(&self) -> Option<MessageId> {
        self.last_message.map(|(mid, _date)| mid)
    }
}

const STOP_WORDS: [&str; 192] = [
    "a",
    "about",
    "above",
    "after",
    "again",
    "against",
    "all",
    "also",
    "am",
    "an",
    "and",
    "any",
    "are",
    "aren't",
    "as",
    "at",
    "be",
    "because",
    "been",
    "before",
    "being",
    "below",
    "between",
    "both",
    "but",
    "by",
    "can",
    "can't",
    "cannot",
    "com",
    "could",
    "couldn't",
    "did",
    "didn't",
    "do",
    "does",
    "doesn't",
    "doing",
    "don't",
    "down",
    "during",
    "each",
    "else",
    "ever",
    "few",
    "for",
    "from",
    "further",
    "get",
    "had",
    "hadn't",
    "has",
    "hasn't",
    "have",
    "haven't",
    "having",
    "he",
    "he'd",
    "he'll",
    "he's",
    "hence",
    "her",
    "here",
    "here's",
    "hers",
    "herself",
    "him",
    "himself",
    "his",
    "how",
    "how's",
    "however",
    "http",
    "i",
    "i'd",
    "i'll",
    "i'm",
    "i've",
    "if",
    "in",
    "into",
    "is",
    "isn't",
    "it",
    "it's",
    "its",
    "itself",
    "just",
    "k",
    "let's",
    "like",
    "me",
    "more",
    "most",
    "mustn't",
    "my",
    "myself",
    "no",
    "nor",
    "not",
    "of",
    "off",
    "on",
    "once",
    "only",
    "or",
    "other",
    "otherwise",
    "ought",
    "our",
    "ours",
    "ourselves",
    "out",
    "over",
    "own",
    "r",
    "same",
    "shall",
    "shan't",
    "she",
    "she'd",
    "she'll",
    "she's",
    "should",
    "shouldn't",
    "since",
    "so",
    "some",
    "such",
    "than",
    "that",
    "that's",
    "the",
    "their",
    "theirs",
    "them",
    "themselves",
    "then",
    "there",
    "there's",
    "therefore",
    "these",
    "they",
    "they'd",
    "they'll",
    "they're",
    "they've",
    "this",
    "those",
    "through",
    "to",
    "too",
    "under",
    "until",
    "up",
    "very",
    "was",
    "wasn't",
    "we",
    "we'd",
    "we'll",
    "we're",
    "we've",
    "were",
    "weren't",
    "what",
    "what's",
    "when",
    "when's",
    "where",
    "where's",
    "which",
    "while",
    "who",
    "who's",
    "whom",
    "why",
    "why's",
    "with",
    "won't",
    "would",
    "wouldn't",
    "www",
    "you",
    "you'd",
    "you'll",
    "you're",
    "you've",
    "your",
    "yours",
    "yourself",
    "yourselves",
];
