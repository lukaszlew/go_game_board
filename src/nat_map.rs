use crate::types::Nat;
use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct NatMap<const SIZE: usize, N: Nat, T> {
    data: [T; SIZE],
    _phantom: std::marker::PhantomData<N>,
}

impl<const SIZE: usize, N: Nat, T: Default + Clone> NatMap<SIZE, N, T> {
    pub fn new() -> Self {
        Self {
            data: [(); SIZE].map(|_| T::default()),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<const SIZE: usize, N: Nat, T: Clone> NatMap<SIZE, N, T> {
    pub fn new_with(value: T) -> Self {
        Self {
            data: [(); SIZE].map(|_| value.clone()),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<const SIZE: usize, N: Nat, T> Index<N> for NatMap<SIZE, N, T> {
    type Output = T;

    fn index(&self, idx: N) -> &Self::Output {
        let index: usize = idx.into();
        &self.data[index as usize]
    }
}

impl<const SIZE: usize, N: Nat, T> IndexMut<N> for NatMap<SIZE, N, T> {
    fn index_mut(&mut self, idx: N) -> &mut Self::Output {
        let index: usize = idx.into();
        &mut self.data[index as usize]
    }
}

impl<const SIZE: usize, N: Nat, T: Clone> Default for NatMap<SIZE, N, T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new()
    }
}
