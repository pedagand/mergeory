unchanged![];
unchanged![];
unchanged![];
changed![
    {
        #[doc = "\n * A `SparseVec` is a dynamic array of items of type `T` which allow holes\n * inside its structure. New items are stored preferentially in existing holes\n * instead of making the array bigger.\n * This allows fast deletion without modification of the indices of other\n * items.\n "]
        pub struct SparseVec<T> {
            first_empty: usize,
            array: Vec<Entry<T>>,
        }
    },
    {
        #[doc = "\n * A `SparseVec` is a dynamic array of items of type `T` which allow holes\n * inside its structure. New items are stored preferentially in existing holes\n * instead of making the array bigger.\n * This allows fast deletion without modification of the indices of other\n * items.\n "]
        pub struct SparseVec<T> {
            first_empty: usize,
            array: Vec<Entry<T>>,
            dummy: (),
        }
    }
];
impl<T> SparseVec<T> {
    pub fn new() -> SparseVec<T> {
        changed![
            {
                SparseVec {
                    first_empty: mv![0],
                    array: mv![1],
                }
            },
            {
                SparseVec {
                    first_empty: mv![0],
                    array: mv![1],
                    dummy: (),
                }
            }
        ]
    }
    pub fn with_capacity(capacity: usize) -> SparseVec<T> {
        changed![
            {
                SparseVec {
                    first_empty: mv![0],
                    array: mv![2],
                }
            },
            {
                SparseVec {
                    first_empty: mv![0],
                    array: mv![2],
                    dummy: (),
                }
            }
        ]
    }
    unchanged![];
    unchanged![];
    unchanged![];
    pub fn remove(&mut self, index: usize) -> Option<T> {
        unchanged![];
        match unchanged![3] {
            Entry::Full(_) => {
                deleted![
                    use core::mem::replace;
                ];
                let old_entry = changed![{ replace(mv![3], mv![4]) }, { *mv![3] }];
                inserted ! [* mv ! [3] = mv ! [4] ;];
                unchanged![];
                unchanged![];
            }
            Entry::Empty(_) => unchanged![],
        }
    }
    unchanged![];
    unchanged![];
    unchanged![];
    unchanged![];
}
unchanged![];
unchanged![];
