use std::ops::Deref;

pub struct Vec<T> {
    ptr: *mut T,
    cap: usize,
    len: usize,
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            cap: 0,
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let layout = std::alloc::Layout::array::<T>(capacity).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) } as *mut T;
        Self {
            ptr,
            cap: capacity,
            len: 0,
        }
    }

    fn grow_amortized(&mut self, additional: usize) {
        let required_capacity = self.len + additional;
        if required_capacity > self.cap {
            let new_capacity = self.cap.max(1).saturating_mul(2).max(required_capacity);
            self.grow(new_capacity);
        }
    }

    fn grow(&mut self, new_capacity: usize) {
        let layout = std::alloc::Layout::array::<T>(new_capacity).unwrap();
        let new_ptr = unsafe { std::alloc::alloc(layout) } as *mut T;
        if !self.ptr.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(self.ptr, new_ptr, self.len);
                let old_layout = std::alloc::Layout::array::<T>(self.cap).unwrap();
                std::alloc::dealloc(self.ptr as *mut u8, old_layout);
            }
        }
        self.ptr = new_ptr;
        self.cap = new_capacity;
    }

    pub fn push(&mut self, value: T) {
        let _ = self.push_mut(value);
    }

    pub fn push_mut(&mut self, value: T) -> &mut T {
        if self.len == self.cap {
            self.grow_amortized(1);
        }
        unsafe {
            let ptr = self.ptr.add(self.len);
            ptr.write(value);
            self.len += 1;
            &mut *ptr
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(self.ptr.add(self.len).read()) }
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("Index out of bounds");
        }
        unsafe {
            let ptr = self.ptr.add(index);
            let value = ptr.read();
            std::ptr::copy(ptr.add(1), ptr, self.len - index - 1);
            self.len -= 1;
            value
        }
    }

    pub fn insert(&mut self, index: usize, value: T) {
        if index > self.len {
            panic!("Index out of bounds");
        }
        if self.len == self.cap {
            self.grow_amortized(1);
        }
        unsafe {
            let ptr = self.ptr.add(index);
            std::ptr::copy(ptr, ptr.add(1), self.len - index);
            ptr.write(value);
            self.len += 1;
        }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                for i in 0..self.len {
                    self.ptr.add(i).drop_in_place();
                }
                let layout = std::alloc::Layout::array::<T>(self.cap).unwrap();
                std::alloc::dealloc(self.ptr as *mut u8, layout);
            }
        }
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec() {
        let mut vec = Vec::new();
        vec.push(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(&*vec, &[1, 2, 3]);
        assert_eq!(vec.pop(), Some(3));
        assert_eq!(&*vec, &[1, 2]);
        vec.insert(1, 4);
        assert_eq!(&*vec, &[1, 4, 2]);
        assert_eq!(vec.remove(0), 1);
        assert_eq!(&*vec, &[4, 2]);
        let mut vec = Vec::with_capacity(2);
        vec.push(5);
        vec.push(6);
        vec.push(7);
        assert_eq!(&*vec, &[5, 6, 7]);
    }
}
