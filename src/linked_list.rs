pub struct LinkedList<T> {
    head: Option<*mut Node<T>>,
    tail: Option<*mut Node<T>>,
    len: usize,
}

struct Node<T> {
    value: T,
    next: Option<*mut Node<T>>,
    prev: Option<*mut Node<T>>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        LinkedList {
            head: None,
            tail: None,
            len: 0,
        }
    }

    pub fn push_back(&mut self, value: T) {
        let new_node = Box::into_raw(Box::new(Node {
            value,
            next: None,
            prev: self.tail,
        }));
        self.push_back_node(new_node);
    }

    pub fn push_back_mut(&mut self, value: T) -> *mut T {
        let new_node = Box::into_raw(Box::new(Node {
            value,
            next: None,
            prev: self.tail,
        }));
        self.push_back_node(new_node);
        unsafe { &mut (*new_node).value }
    }

    fn push_back_node(&mut self, new_node: *mut Node<T>) {
        if let Some(tail) = self.tail {
            unsafe { (*tail).next = Some(new_node) };
        } else {
            self.head = Some(new_node);
        }

        self.tail = Some(new_node);
        self.len += 1;
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if let Some(tail) = self.tail {
            let tail_node = unsafe { Box::from_raw(tail) };
            self.tail = tail_node.prev;

            if let Some(prev) = tail_node.prev {
                unsafe { (*prev).next = None };
            } else {
                self.head = None;
            }

            self.len -= 1;
            Some(tail_node.value)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        while self.pop_back().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linked_list() {
        let mut linked_list = LinkedList::new();
        linked_list.push_back(1);
        linked_list.push_back(2);
        linked_list.push_back(3);
        assert_eq!(linked_list.len(), 3);
        linked_list.pop_back();
        assert_eq!(linked_list.len(), 2);
        assert_eq!(linked_list.pop_back(), Some(2));
        assert_eq!(linked_list.pop_back(), Some(1));
        assert_eq!(linked_list.pop_back(), None);
    }
}
