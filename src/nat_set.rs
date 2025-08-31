use crate::types::Nat;

pub struct NatSet<const SIZE: usize, T: Nat> {
    marked: [bool; SIZE],
    _phantom: std::marker::PhantomData<T>,
}

impl<const SIZE: usize, T: Nat> NatSet<SIZE, T> {
    pub fn new() -> Self {
        NatSet {
            marked: [false; SIZE],
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn clear(&mut self) {
        self.marked.fill(false);
    }

    pub fn mark(&mut self, item: T) {
        let index: usize = item.into();
        self.marked[index as usize] = true;
    }

    pub fn is_marked(&self, item: T) -> bool {
        let index: usize = item.into();
        self.marked[index as usize]
    }
}
