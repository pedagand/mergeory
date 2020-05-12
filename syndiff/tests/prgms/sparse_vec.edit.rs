use alloc::vec::Vec;
use core::ops::{Index, IndexMut};

enum Entry<T> {
    Full(T),
    Empty(usize),
}

/**
 * A `SparseVec` is a dynamic array of items of type `T` which allow holes
 * inside its structure. New items are stored preferentially in existing holes
 * instead of making the array bigger.
 * This allows fast deletion without modification of the indices of other
 * items.
 */
pub struct SparseVec<T> {
    first_empty: usize,
    array: Vec<Entry<T>>,
    dummy: (),
}

impl<T> SparseVec<T> {
    pub fn new() -> SparseVec<T> {
        SparseVec {
            first_empty: 0,
            array: Vec::new(),
            dummy: (),
        }
    }

    pub fn with_capacity(capacity: usize) -> SparseVec<T> {
        SparseVec {
            first_empty: 0,
            array: Vec::with_capacity(capacity),
            dummy: (),
        }
    }

    pub fn capacity(&self) -> usize {
        self.array.capacity()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.array.reserve(additional)
    }

    pub fn insert(&mut self, element: T) -> usize {
        let entry_id = self.first_empty;
        if entry_id == self.array.len() {
            self.first_empty += 1;
            self.array.push(Entry::Full(element));
        } else {
            if let Entry::Empty(next_empty) = self.array[entry_id] {
                self.array[entry_id] = Entry::Full(element);
                self.first_empty = next_empty;
            } else {
                panic!("non empty entry pointed by first_empty");
            }
        }
        entry_id
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let entry = &mut self.array[index];
        match entry {
            Entry::Full(_) => {
                let old_entry = *entry;
                *entry = Entry::Empty(self.first_empty);
                self.first_empty = index;

                if let Entry::Full(element) = old_entry {
                    Some(element)
                } else {
                    unreachable!()
                }
            }
            Entry::Empty(_) => None,
        }
    }

    pub fn contains(&self, index: usize) -> bool {
        if index >= self.array.len() {
            return false;
        }
        match self.array[index] {
            Entry::Full(_) => true,
            Entry::Empty(_) => false,
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.array.len() {
            return None;
        }
        match self.array[index] {
            Entry::Full(ref element) => Some(element),
            Entry::Empty(_) => None,
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.array.len() {
            return None;
        }
        match self.array[index] {
            Entry::Full(ref mut element) => Some(element),
            Entry::Empty(_) => None,
        }
    }

    pub fn clear(&mut self) {
        self.array.clear();
        self.first_empty = 0;
    }
}

impl<T> Index<usize> for SparseVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        match self.array[index] {
            Entry::Full(ref element) => element,
            Entry::Empty(_) => panic!("free space at specified index"),
        }
    }
}

impl<T> IndexMut<usize> for SparseVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        match self.array[index] {
            Entry::Full(ref mut element) => element,
            Entry::Empty(_) => panic!("free space at specified index"),
        }
    }
}
