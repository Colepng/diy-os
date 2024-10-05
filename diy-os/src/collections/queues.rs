use crate::collections::linked_lists::LinkedList;
use alloc::vec::Vec;

pub trait Queue<T> {
    fn push(&mut self, value: T);
    fn pop(&mut self) -> T;
}

pub struct VecQueue<T> {
    internal: Vec<T>,
}

impl<T> VecQueue<T> {
    pub fn new() -> Self {
        Self {
            internal: Vec::new(),
        }
    }
}

impl<T> Queue<T> for VecQueue<T> {
    fn push(&mut self, value: T) {
        self.internal.push(value);
    }

    fn pop(&mut self) -> T {
        self.internal.remove(0)
    }
}

pub struct LinkedQueue<T> {
    internal: LinkedList<T>,
}

impl<T> LinkedQueue<T> {
    pub fn new() -> Self {
        Self {
            internal: LinkedList::new(),
        }
    }

    pub fn push(&mut self, value: T) {
        self.internal.append(value);
    }

    pub fn remove_head(&mut self) -> T {
        self.internal.pop_front()
    }

    pub fn get_head(&self) -> Option<&T> {
        self.internal.get_head()
    }

    pub fn is_empty(&self) -> bool {
        self.internal.is_empty()
    }
}
