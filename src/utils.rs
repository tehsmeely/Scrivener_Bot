use serenity::utils::MessageBuilder;

pub mod helpers {
    pub fn strip_leading_trailing(s: &str, c: char) -> &str {
        let prefix_stripped: &str = match s.strip_prefix(c) {
            Some(stripped) => stripped,
            None => s,
        };
        match prefix_stripped.strip_suffix(c) {
            Some(stripped) => stripped,
            None => prefix_stripped,
        }
    }
}

pub mod iterators {
    use std::cmp;
    use std::collections::HashMap;
    use std::hash::Hash;

    #[derive(Debug)]
    pub struct SortedHashMap<'a, K, V> {
        map: &'a HashMap<K, V>,
        sorted_keys: Vec<&'a K>,
        limit: usize,
        i: usize,
        pub is_truncated: bool,
    }

    impl<'a, K, V> SortedHashMap<'a, K, V> {
        pub fn new(map: &'a HashMap<K, V>, sorted_keys: Vec<&'a K>, limit: Option<usize>) -> Self {
            if sorted_keys.len() > map.len() {
                panic!("Sorted keys cannot exceed length of map, as it is used to index it")
            }
            let limit =
                //Use the limit if provided, but it needs to be capped at max_len since we trust it as an upper bound
                limit.map_or(sorted_keys.len(), |lim| cmp::min(sorted_keys.len(), lim));
            let is_truncated = limit < sorted_keys.len();
            Self {
                map,
                sorted_keys,
                limit,
                i: 0,
                is_truncated,
            }
        }

        pub fn is_truncated(&self) -> bool {
            self.is_truncated
        }

        pub fn limit(&self) -> usize {
            self.limit
        }
    }

    impl<'a, K: Eq + Hash, V> Iterator for SortedHashMap<'a, K, V> {
        type Item = (&'a K, &'a V);

        fn next(&mut self) -> Option<Self::Item> {
            if self.i >= self.limit {
                None
            } else {
                let key = self.sorted_keys[self.i];
                let value = self.map.get(key).unwrap();
                self.i += 1;
                Some((key, value))
            }
        }
    }

    //TODO: Extension: Implement IntoIterator including some sort_by fn to generate [sorted_keys]
}

pub mod unidecode_extended {
    use serenity::framework::standard::HelpBehaviour::Strike;

    pub fn unidecode(input: &str) -> Option<String> {
        let unknown_char = "[?]";
        let mut out = String::new();
        for c in input.chars() {
            let decoded = unidecode::unidecode_char(extra_char_map(c));
            if decoded == unknown_char {
                return None;
            }
            out.push_str(decoded);
        }
        Some(out)
    }

    fn extra_char_map(c: char) -> char {
        //unidecode handles a lot of things but no smallcaps (and maybe other things)
        // hence this special case handling here. It's a bit "brute-force"y but there's not much
        // we can do :(
        match c {
            'ᴀ' => 'a',
            'ʙ' => 'b',
            'ᴄ' => 'c',
            'ᴅ' => 'd',
            'ᴇ' => 'e',
            'ꜰ' => 'f',
            'ғ' => 'f',
            'ɢ' => 'g',
            'ʜ' => 'h',
            'ɪ' => 'i',
            'ᴊ' => 'j',
            'ᴋ' => 'k',
            'ʟ' => 'l',
            'ᴍ' => 'm',
            'ɴ' => 'n',
            'ᴏ' => 'o',
            'ᴘ' => 'p',
            'ǫ' => 'q',
            'ʀ' => 'r',
            's' => 's',
            'ᴛ' => 't',
            'ᴜ' => 'u',
            'ᴠ' => 'v',
            'ᴡ' => 'w',
            'x' => 'x',
            'ʏ' => 'y',
            'ᴢ' => 'z',
            _ => c,
        }
    }
}

pub mod trait_extensions {
    use serenity::utils::MessageBuilder;

    pub trait MessageBuilderExt {
        fn newline(&mut self) -> &mut Self;
        fn apply_if<F>(&mut self, apply: bool, f: F) -> &mut Self
        where
            F: FnOnce(&mut Self) -> &mut Self;
    }

    impl MessageBuilderExt for MessageBuilder {
        fn newline(&mut self) -> &mut Self {
            self.push("\n")
        }
        fn apply_if<F>(&mut self, apply: bool, f: F) -> &mut Self
        where
            F: FnOnce(&mut Self) -> &mut Self,
        {
            if apply {
                f(self);
            }
            self
        }
    }
}

#[cfg(test)]
mod test_iter {
    use std::collections::HashMap;

    use serenity::futures::StreamExt;

    use crate::utils::iterators::SortedHashMap;

    fn make_map() -> HashMap<String, (usize, f32)> {
        let mut map = HashMap::new();
        //Tuple [0] is in order, [1] is intentionally out of order. we'll sort on [0]
        map.insert(String::from("a"), (1, 0.2));
        map.insert(String::from("b"), (2, 0.5));
        map.insert(String::from("c"), (3, 0.3));
        map.insert(String::from("d"), (4, 0.6));
        map.insert(String::from("e"), (5, 0.4));
        map
    }
    fn make_iter(
        map: &HashMap<String, (usize, f32)>,
        limit: Option<usize>,
    ) -> SortedHashMap<String, (usize, f32)> {
        let mut keys: Vec<(&String, &usize)> = map.iter().map(|(k, (v, _))| (k, v)).collect();
        keys.sort_by_key(|(_, v)| *v);
        let keys = keys.iter().map(|(k, _v)| *k).collect();
        SortedHashMap::new(&map, keys, limit)
    }

    #[test]
    fn iter_sorting() {
        let map = make_map();
        let mut iter = make_iter(&map, None);
        assert_eq!(iter.next(), Some((&String::from("a"), &(1, 0.2))));
        assert_eq!(iter.next(), Some((&String::from("b"), &(2, 0.5))));
        assert_eq!(iter.next(), Some((&String::from("c"), &(3, 0.3))));
        assert_eq!(iter.next(), Some((&String::from("d"), &(4, 0.6))));
        assert_eq!(iter.next(), Some((&String::from("e"), &(5, 0.4))));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_sorting_with_limit() {
        let map = make_map();
        let mut iter = make_iter(&map, Some(3));
        assert_eq!(iter.next(), Some((&String::from("a"), &(1, 0.2))));
        assert_eq!(iter.next(), Some((&String::from("b"), &(2, 0.5))));
        assert_eq!(iter.next(), Some((&String::from("c"), &(3, 0.3))));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_sorting_with_limit_too_high() {
        let map = make_map();
        let mut iter = make_iter(&map, Some(100));
        assert_eq!(iter.next(), Some((&String::from("a"), &(1, 0.2))));
        assert_eq!(iter.next(), Some((&String::from("b"), &(2, 0.5))));
        assert_eq!(iter.next(), Some((&String::from("c"), &(3, 0.3))));
        assert_eq!(iter.next(), Some((&String::from("d"), &(4, 0.6))));
        assert_eq!(iter.next(), Some((&String::from("e"), &(5, 0.4))));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_with_keys_lt_map() {
        let map = make_map();
        let actual_keys: Vec<String> = vec!["b", "c", "d"]
            .iter()
            .map(|&s| String::from(s))
            .collect();
        let keys: Vec<&String> = actual_keys.iter().map(|c| c).collect();
        let mut iter = SortedHashMap::new(&map, keys, None);
        assert_eq!(iter.next(), Some((&String::from("b"), &(2, 0.5))));
    }
    #[test]
    #[should_panic(expected = "Sorted keys cannot exceed length of map, as it is used to index it")]
    fn iter_with_keys_gt_map() {
        let map = make_map();
        let actual_keys: Vec<String> = vec!["a", "b", "c", "d", "e", "f"]
            .iter()
            .map(|&s| String::from(s))
            .collect();
        let keys: Vec<&String> = actual_keys.iter().map(|c| c).collect();
        let _iter = SortedHashMap::new(&map, keys, None);
    }

    #[test]
    fn unicode_fudging() {
        let ex1 = String::from("ᴍᴜɴɪᴄɪᴘᴀʟ");
        let ex2 = String::from("hello world");
        let ex3 = String::from("bónjourúo");
        let ex4 = "Ana bursts into the underground parking lot and narrows her eyes at the trundling cube of an automated ticket-bot speeding towards her bike like an angry hornet. The second-hand Suzuki Mirage has sleek, if slightly chipped lines that are undermined somewhat by the bulky Noodle King insulation box tied to the back. It is a fast vehicle, and only it's well-worn appearance is enough to make it not too unlikely for a delivery driver. Brightly coloured holostickers and graphics cluster around the front of the short screen, and it looked like the ticket-bot wants to add another. Ana speeds up and kicks the hapless robot in the side, rushing to start the vehicle and get out as the bot splutters an indignant *Tʜɪs ʙᴏᴛ ɪs. ᴘʀᴏᴘᴇʀᴛʏ ᴏғ ᴛʜᴇ. ᴍᴜɴᴄɪᴘɪᴀʟ ᴄᴏᴜɴᴄɪʟ ᴀɴᴅ. ᴠᴀɴᴅᴀʟɪsᴍ. ᴍᴀʏ ɪɴᴄᴜʀ ᴀ ғɪɴᴇ*.\n\nFlicking her rigger interface from a pocket inside her jacket, Ana clips it in below the handles of the bike as she slides into traffic, turning to grab her helmet as the AR interface provided by her glasses lets her drive the bike with a flick of her eyes. It's only a short ride from the apartment building, and she quickly finds a convenient spot off the motorway junction for a swift departure, opening the Noodle King box for the drones. There were small ones, no bigger than a soyrizo ball or the size of her palm, but today she'd want something a little larger. The Lockheed Optic-X2 had cost her three times as much as the bike, and for good reason. No bigger than a cyberdeck when fully folded, it neatly fitted inside a noodle box with a little padding and-\n\n- the waft of fragrant steam is unexpected as she breaks the seal on the branded packaging. *Oh no. Oh fuck oh fuck oh fuck.* Udon noodles with sweet and sour Royal soyChik'n. With extra onions. Ana's stomach drops like a stone, a sick churning feeling inside. *Fuuuuuuuuuuuuck*. She starts the bike again with a grim expression.\n**Oculus** 19:51: *Drone issue. Won't be long.*";
        let exp4 = "Ana bursts into the underground parking lot and narrows her eyes at the trundling cube of an automated ticket-bot speeding towards her bike like an angry hornet. The second-hand Suzuki Mirage has sleek, if slightly chipped lines that are undermined somewhat by the bulky Noodle King insulation box tied to the back. It is a fast vehicle, and only it's well-worn appearance is enough to make it not too unlikely for a delivery driver. Brightly coloured holostickers and graphics cluster around the front of the short screen, and it looked like the ticket-bot wants to add another. Ana speeds up and kicks the hapless robot in the side, rushing to start the vehicle and get out as the bot splutters an indignant *This bot is. property of the. muncipial council and. vandalism. may incur a fine*.\n\nFlicking her rigger interface from a pocket inside her jacket, Ana clips it in below the handles of the bike as she slides into traffic, turning to grab her helmet as the AR interface provided by her glasses lets her drive the bike with a flick of her eyes. It's only a short ride from the apartment building, and she quickly finds a convenient spot off the motorway junction for a swift departure, opening the Noodle King box for the drones. There were small ones, no bigger than a soyrizo ball or the size of her palm, but today she'd want something a little larger. The Lockheed Optic-X2 had cost her three times as much as the bike, and for good reason. No bigger than a cyberdeck when fully folded, it neatly fitted inside a noodle box with a little padding and-\n\n- the waft of fragrant steam is unexpected as she breaks the seal on the branded packaging. *Oh no. Oh fuck oh fuck oh fuck.* Udon noodles with sweet and sour Royal soyChik'n. With extra onions. Ana's stomach drops like a stone, a sick churning feeling inside. *Fuuuuuuuuuuuuck*. She starts the bike again with a grim expression.\n**Oculus** 19:51: *Drone issue. Won't be long.*";
        let out1 = crate::utils::unidecode_extended::unidecode(&ex1);
        let out2 = crate::utils::unidecode_extended::unidecode(&ex2);
        let out3 = crate::utils::unidecode_extended::unidecode(&ex3);
        let out4 = crate::utils::unidecode_extended::unidecode(ex4);
        assert_eq!(out1, Some(String::from("municipal")));
        assert_eq!(out2, Some(String::from("hello world")));
        assert_eq!(out3, Some(String::from("bonjouruo")));
        assert_eq!(out4, Some(String::from(exp4)));
    }
}
