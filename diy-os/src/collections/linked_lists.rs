use alloc::boxed::Box;

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
}

impl<T> LinkedList<T> {
    pub const fn new() -> Self {
        Self { head: None }
    }

    pub fn prepend(&mut self, value: T) {
        let node = Node {
            value,
            next: self.head.take(),
        };

        self.head = Some(Box::new(node));
    }

    pub fn append(&mut self, value: T) {
        if self.head.is_some() {
            let mut next = self.head.as_mut().unwrap();

            while next.next.is_some() {
                next = next.next.as_mut().unwrap();
            }

            let node = Node { value, next: None };

            next.next = Some(Box::new(node));
        } else {
            self.prepend(value);
        }
    }

    pub fn pop_front(&mut self) -> T {
        let old_head = self.head.take().unwrap();

        self.head = old_head.next;

        old_head.value
    }

    pub fn get_head(&self) -> Option<&T> {
        self.head.as_ref().map(|node| &node.value)
    }

    pub fn get_head_mut(&mut self) -> Option<&mut T> {
        self.head.as_mut().map(|node| &mut node.value)
    }

    pub const fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}
