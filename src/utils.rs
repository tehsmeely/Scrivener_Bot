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
    }

    impl<'a, K, V> SortedHashMap<'a, K, V> {
        pub fn new(map: &'a HashMap<K, V>, sorted_keys: Vec<&'a K>, limit: Option<usize>) -> Self {
            let limit =
                //Use the limit if provided, but it needs to be capped at max_len since we trust it as an upper bound
                limit.map_or(sorted_keys.len(), |lim| cmp::min(sorted_keys.len(), lim));
            Self {
                map,
                sorted_keys,
                limit,
                i: 0,
            }
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
}
