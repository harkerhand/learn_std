use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

pub struct HashMap<K, V> {
    bucket_mask: usize,
    ctrl: *mut u8,
    growth_left: usize,
    items: usize,
    _marker: std::marker::PhantomData<(K, V)>,
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self {
            bucket_mask: 0,
            ctrl: std::ptr::null_mut(),
            growth_left: 0,
            items: 0,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_capacity(mut capacity: usize) -> Self {
        if capacity == 0 {
            return Self::new();
        }
        capacity = capacity.next_power_of_two().max(8);
        let bucket_mask = capacity.next_power_of_two() - 1;
        let bucket_size = (bucket_mask + 1) * std::mem::size_of::<(K, V)>();
        let ctrl_size = (bucket_mask + 1) * std::mem::size_of::<u8>();
        let mut ctrl = unsafe {
            std::alloc::alloc(
                std::alloc::Layout::from_size_align(bucket_size + ctrl_size, 1).unwrap(),
            )
        };
        ctrl = unsafe { ctrl.add(bucket_size) };
        unsafe { std::ptr::write_bytes(ctrl, 0xFF, ctrl_size) };
        Self {
            bucket_mask,
            ctrl,
            growth_left: capacity,
            items: 0,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.items * 8 >= (self.bucket_mask + 1) * 7 || self.items + 1 >= self.growth_left {
            self.resize();
        }

        let hash = self.hash(&key);

        let tag = (hash & 0x7F) as u8;
        let mut probe_index = hash & self.bucket_mask;
        loop {
            let group_bytes = self.get_group_bytes(probe_index);
            let empty_mask = self.match_group(group_bytes, 0xFF);
            if empty_mask != 0 {
                for i in 0..8 {
                    if (empty_mask & (1 << (7 - i))) != 0 {
                        let item_ptr = unsafe {
                            (self.ctrl as *mut (K, V))
                                .sub(self.bucket_mask + 1)
                                .add((probe_index + i) & self.bucket_mask)
                        };
                        unsafe {
                            *item_ptr = (key, value);
                            *self.ctrl.add((probe_index + i) & self.bucket_mask) = tag;
                        }
                        self.items += 1;
                        self.growth_left -= 1;
                        return;
                    }
                }
            }

            let delete_mask = self.match_group(group_bytes, 0xFE);
            if delete_mask != 0 {
                for i in 0..8 {
                    if (delete_mask & (1 << (7 - i))) != 0 {
                        let item_ptr = unsafe {
                            (self.ctrl as *mut (K, V))
                                .sub(self.bucket_mask + 1)
                                .add((probe_index + i) & self.bucket_mask)
                        };
                        unsafe {
                            *item_ptr = (key, value);
                            *self.ctrl.add((probe_index + i) & self.bucket_mask) = tag;
                        }
                        self.items += 1;
                        self.growth_left -= 1;
                        return;
                    }
                }
            }
            let match_mask = self.match_group(group_bytes, tag);
            if match_mask != 0 {
                for i in 0..8 {
                    if (match_mask & (1 << (7 - i))) != 0 {
                        let item_ptr = unsafe {
                            (self.ctrl as *mut (K, V))
                                .sub(self.bucket_mask + 1)
                                .add((probe_index + i) & self.bucket_mask)
                        };
                        unsafe {
                            if (*item_ptr).0 == key {
                                (*item_ptr).1 = value;
                                return;
                            }
                        }
                    }
                }
            }
            probe_index = (probe_index + 1) & self.bucket_mask;
        }
    }

    fn hash(&self, key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = self.hash(key);
        let tag = (hash & 0x7F) as u8;
        let mut probe_index = hash & self.bucket_mask;
        loop {
            let group_bytes = self.get_group_bytes(probe_index);
            let match_mask = self.match_group(group_bytes, tag);
            if match_mask != 0 {
                for i in 0..8 {
                    if (match_mask & (1 << (7 - i))) != 0 {
                        let item_ptr = unsafe {
                            (self.ctrl as *mut (K, V))
                                .sub(self.bucket_mask + 1)
                                .add((probe_index + i) & self.bucket_mask)
                        };
                        unsafe {
                            if (*item_ptr).0 == *key {
                                return Some(&(*item_ptr).1);
                            }
                        }
                    }
                }
            }
            let empty_mask = self.match_group(group_bytes, 0xFF);
            if empty_mask != 0 {
                return None;
            }

            probe_index = (probe_index + 1) & self.bucket_mask;
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = self.hash(key);
        let tag = (hash & 0x7F) as u8;
        let mut probe_index = hash & self.bucket_mask;
        loop {
            let group_bytes = self.get_group_bytes(probe_index);
            let match_mask = self.match_group(group_bytes, tag);
            if match_mask != 0 {
                for i in 0..8 {
                    if (match_mask & (1 << (7 - i))) != 0 {
                        let item_ptr = unsafe {
                            (self.ctrl as *mut (K, V))
                                .sub(self.bucket_mask + 1)
                                .add((probe_index + i) & self.bucket_mask)
                        };
                        unsafe {
                            if (*item_ptr).0 == *key {
                                let _key = std::ptr::read(&(*item_ptr).0);
                                let value = std::mem::replace(
                                    &mut (*item_ptr).1,
                                    std::mem::MaybeUninit::uninit().assume_init(),
                                );
                                *self.ctrl.add((probe_index + i) & self.bucket_mask) = 0xFE;
                                self.items -= 1;
                                return Some(value);
                            }
                        }
                    }
                }
            }
            let empty_mask = self.match_group(group_bytes, 0xFF);
            if empty_mask != 0 {
                return None;
            }

            probe_index = (probe_index + 1) & self.bucket_mask;
        }
    }

    fn resize(&mut self) {
        let new_capacity = (self.bucket_mask + 1) * 2;
        let mut new_map = HashMap::with_capacity(new_capacity);
        for i in 0..=self.bucket_mask {
            let group_ctrl = unsafe { *self.ctrl.add(i) };
            if group_ctrl != 0xFF {
                let item_ptr =
                    unsafe { (self.ctrl as *mut (K, V)).sub(self.bucket_mask + 1).add(i) };
                unsafe {
                    let (key, value) = std::ptr::read(item_ptr);
                    new_map.insert(key, value);
                    *self.ctrl.add(i) = 0xFF;
                }
            }
        }
        *self = new_map;
    }

    fn get_group_bytes(&self, index: usize) -> u64 {
        let mut group_bytes = 0u64;
        for i in 0..8 {
            group_bytes |=
                (unsafe { *self.ctrl.add((index + i) & self.bucket_mask) } as u64) << ((7 - i) * 8);
        }
        group_bytes
    }

    fn match_group(&self, group_bytes: u64, tag: u8) -> u8 {
        let match_mask = group_bytes ^ (tag as u64 * 0x0101010101010101);
        let mut result = 0u8;
        for i in 0..8 {
            if (match_mask & (0xFF << ((7 - i) * 8))) == 0 {
                result |= 1 << (7 - i);
            }
        }
        result
    }
}

impl<K, V> Drop for HashMap<K, V> {
    fn drop(&mut self) {
        for i in 0..=self.bucket_mask {
            let group_ctrl = unsafe { *self.ctrl.add(i) };
            if group_ctrl != 0xFF && group_ctrl != 0xFE {
                let item_ptr =
                    unsafe { (self.ctrl as *mut (K, V)).sub(self.bucket_mask + 1).add(i) };
                unsafe {
                    std::ptr::drop_in_place(item_ptr);
                }
            }
        }
        if !self.ctrl.is_null() {
            let bucket_size = (self.bucket_mask + 1) * std::mem::size_of::<(K, V)>();
            let ctrl_size = (self.bucket_mask + 1) * std::mem::size_of::<u8>();
            unsafe {
                std::alloc::dealloc(
                    self.ctrl.sub(bucket_size),
                    std::alloc::Layout::from_size_align(bucket_size + ctrl_size, 1).unwrap(),
                );
            }
        }
    }
}

impl<K: Display, V: Display> Display for HashMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        for i in 0..=self.bucket_mask {
            write!(f, "group {}: ", i)?;
            let group_ctrl = unsafe { *self.ctrl.add(i) };
            match group_ctrl {
                0xFF => {
                    writeln!(f, "empty")?;
                }
                0xFE => {
                    writeln!(f, "deleted")?;
                }
                _ => {
                    let (key, value) =
                        unsafe { &(*(self.ctrl as *mut (K, V)).sub(self.bucket_mask + 1).add(i)) };
                    writeln!(f, "occupied (tag: {}): {} => {}", group_ctrl, key, value)?;
                }
            }
        }
        writeln!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_map() {
        let mut map = HashMap::with_capacity(1);
        map.insert("key1".to_string(), "value1");
        map.insert("key2".to_string(), "value2");
        assert_eq!(map.get(&"key1".to_string()), Some(&"value1"));
        assert_eq!(map.get(&"key2".to_string()), Some(&"value2"));
        assert_eq!(map.get(&"key3".to_string()), None);
        assert_eq!(map.remove(&"key1".to_string()), Some("value1"));
        assert_eq!(map.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_match_group() {
        let map: HashMap<(), ()> = HashMap::with_capacity(1);
        let tag = 0x7A; // 0b01111010
        let group_bytes = 0xFF7AFEFFFE7A7AFF;
        assert_eq!(map.match_group(group_bytes, tag), 0b01000110);
    }
}
