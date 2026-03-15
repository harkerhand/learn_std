use std::{
    fmt::Display,
    hash::{Hash, Hasher},
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
        let bucket_mask = capacity - 1;
        let bucket_size = capacity * std::mem::size_of::<(K, V)>();
        let ctrl_size = (capacity + 8) * std::mem::size_of::<u8>();
        let mut ctrl = unsafe {
            std::alloc::alloc(
                std::alloc::Layout::from_size_align(
                    bucket_size + ctrl_size,
                    std::mem::align_of::<(K, V)>().max(8),
                )
                .unwrap(),
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
        let mut stride = 0;

        loop {
            let group_bytes = self.get_group_bytes(probe_index);

            let mut match_mask = self.match_group(group_bytes, tag);
            while match_mask != 0 {
                let i = match_mask.trailing_zeros() / 8;
                let index = (probe_index + i as usize) & self.bucket_mask;
                let item_ptr = unsafe {
                    (self.ctrl as *mut (K, V))
                        .sub(self.bucket_mask + 1)
                        .add(index)
                };
                unsafe {
                    if (*item_ptr).0 == key {
                        (*item_ptr).1 = value;
                        return;
                    }
                }
                match_mask &= match_mask - 1;
            }
            let free_mask =
                self.match_group(group_bytes, 0xFF) | self.match_group(group_bytes, 0xFE);
            if free_mask != 0 {
                let index =
                    (probe_index + free_mask.trailing_zeros() as usize / 8) & self.bucket_mask;
                let item_ptr = unsafe {
                    (self.ctrl as *mut (K, V))
                        .sub(self.bucket_mask + 1)
                        .add(index)
                };
                unsafe {
                    *item_ptr = (key, value);
                    self.update_ctrl(index, tag);
                }
                self.items += 1;
                self.growth_left -= 1;
                return;
            }
            stride += 8;
            probe_index = (probe_index + stride) & self.bucket_mask;
        }
    }

    fn insert_unchecked(&mut self, key: K, value: V) {
        let hash = self.hash(&key);
        let tag = (hash & 0x7F) as u8;
        let mut probe_index = hash & self.bucket_mask;
        let mut stride = 0;

        loop {
            let group_bytes = self.get_group_bytes(probe_index);
            let empty_mask = self.match_group(group_bytes, 0xFF);
            if empty_mask != 0 {
                let index =
                    (probe_index + empty_mask.trailing_zeros() as usize / 8) & self.bucket_mask;
                let item_ptr = unsafe {
                    (self.ctrl as *mut (K, V))
                        .sub(self.bucket_mask + 1)
                        .add(index)
                };
                unsafe {
                    *item_ptr = (key, value);
                    self.update_ctrl(index, tag);
                }
                return;
            }
            stride += 8;
            probe_index = (probe_index + stride) & self.bucket_mask;
        }
    }

    fn hash(&self, key: &K) -> usize {
        let mut hasher = fxhash::FxHasher::default();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = self.hash(key);
        let tag = (hash & 0x7F) as u8;
        let mut probe_index = hash & self.bucket_mask;
        let mut stride = 0;
        loop {
            let group_bytes = self.get_group_bytes(probe_index);
            let mut match_mask = self.match_group(group_bytes, tag);

            while match_mask != 0 {
                let i = match_mask.trailing_zeros() / 8;
                let index = (probe_index + i as usize) & self.bucket_mask;
                let item_ptr = unsafe {
                    (self.ctrl as *mut (K, V))
                        .sub(self.bucket_mask + 1)
                        .add(index)
                };
                unsafe {
                    if (*item_ptr).0 == *key {
                        return Some(&(*item_ptr).1);
                    }
                }
                match_mask &= match_mask - 1;
            }

            if self.match_group(group_bytes, 0xFF) != 0 {
                return None;
            }

            stride += 8;
            probe_index = (probe_index + stride) & self.bucket_mask;
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let hash = self.hash(key);
        let tag = (hash & 0x7F) as u8;
        let mut probe_index = hash & self.bucket_mask;
        let mut stride = 0;
        loop {
            let group_bytes = self.get_group_bytes(probe_index);
            let mut match_mask = self.match_group(group_bytes, tag);
            while match_mask != 0 {
                let i = match_mask.trailing_zeros() / 8;
                let index = (probe_index + i as usize) & self.bucket_mask;
                let item_ptr = unsafe {
                    (self.ctrl as *mut (K, V))
                        .sub(self.bucket_mask + 1)
                        .add(index)
                };
                unsafe {
                    if (*item_ptr).0 == *key {
                        self.update_ctrl(index, 0xFE);
                        std::ptr::drop_in_place(&mut (*item_ptr).0);
                        let value = std::ptr::read(&(*item_ptr).1);
                        self.items -= 1;
                        return Some(value);
                    }
                }
                match_mask &= match_mask - 1;
            }
            if self.match_group(group_bytes, 0xFF) != 0 {
                return None;
            }

            stride += 8;
            probe_index = (probe_index + stride) & self.bucket_mask;
        }
    }

    fn resize(&mut self) {
        let new_capacity = (self.bucket_mask + 1) * 2;
        let mut new_map = HashMap::with_capacity(new_capacity);
        for i in 0..=self.bucket_mask {
            let group_ctrl = unsafe { *self.ctrl.add(i) };
            if group_ctrl < 0x80 {
                let item_ptr =
                    unsafe { (self.ctrl as *mut (K, V)).sub(self.bucket_mask + 1).add(i) };
                unsafe {
                    let (key, value) = std::ptr::read(item_ptr);
                    new_map.insert_unchecked(key, value);
                    *self.ctrl.add(i) = 0xFF;
                }
            }
        }
        *self = new_map;
    }

    fn get_group_bytes(&self, index: usize) -> u64 {
        unsafe { (self.ctrl.add(index) as *const u64).read_unaligned() }
    }

    fn match_group(&self, group_bytes: u64, tag: u8) -> u64 {
        let tag_broadcast = (tag as u64) * 0x0101010101010101;
        let x = group_bytes ^ tag_broadcast;
        x.wrapping_sub(0x0101010101010101) & !x & 0x8080808080808080
    }

    #[inline(always)]
    fn update_ctrl(&mut self, index: usize, tag: u8) {
        unsafe {
            *self.ctrl.add(index) = tag;
            let mirror_index = (index.wrapping_sub(8) & self.bucket_mask).wrapping_add(8);
            *self.ctrl.add(mirror_index) = tag;
        }
    }
}

impl<K, V> Drop for HashMap<K, V> {
    fn drop(&mut self) {
        for i in 0..=self.bucket_mask {
            let group_ctrl = unsafe { *self.ctrl.add(i) };
            if group_ctrl < 0x80 {
                let item_ptr =
                    unsafe { (self.ctrl as *mut (K, V)).sub(self.bucket_mask + 1).add(i) };
                unsafe {
                    std::ptr::drop_in_place(item_ptr);
                }
            }
        }
        if !self.ctrl.is_null() {
            let capacity = self.bucket_mask + 1;
            let bucket_size = capacity * std::mem::size_of::<(K, V)>();
            let ctrl_size = (capacity + 8) * std::mem::size_of::<u8>();
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
        assert_eq!(map.match_group(group_bytes, tag), 0x0080000000808000);
    }
}
