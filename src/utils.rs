use serenity::utils::MessageBuilder;

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
}
