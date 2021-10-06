unchanged![];
unchanged![];
unchanged![];
changed![
    {
        # [diff = mv ! [0]]
        pub struct SparseVec<T> {
            first_empty: usize,
            array: Vec<Entry<T>>,
        }
    },
    {
        # [diff = mv ! [0]]
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
                    first_empty: mv![1],
                    array: mv![2],
                }
            },
            {
                SparseVec {
                    first_empty: mv![1],
                    array: mv![2],
                    dummy: (),
                }
            }
        ]
    }
    pub fn with_capacity(capacity: usize) -> SparseVec<T> {
        changed![
            {
                SparseVec {
                    first_empty: mv![1],
                    array: mv![3],
                }
            },
            {
                SparseVec {
                    first_empty: mv![1],
                    array: mv![3],
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
        match unchanged![4] {
            Entry::Full(_) => {
                deleted![
                    use core::mem::replace;
                ];
                let old_entry = changed![{ replace(mv![4], mv![5]) }, { *mv![4] }];
                inserted ! [* mv ! [4] = mv ! [5] ;];
                unchanged![];
                unchanged![];
            }
            unchanged![] => match_arm![],
        }
    }
    unchanged![];
    unchanged![];
    unchanged![];
    unchanged![];
}
unchanged![];
unchanged![];
