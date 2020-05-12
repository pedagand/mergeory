unchanged![0xb800bc29dbbb7cad];
unchanged![0x42319d7f0fb975e0];
unchanged![0x8be7c35743599f97];
changed![
    {
        #[doc = "/**\n * A `SparseVec` is a dynamic array of items of type `T` which allow holes\n * inside its structure. New items are stored preferentially in existing holes\n * instead of making the array bigger.\n * This allows fast deletion without modification of the indices of other\n * items.\n */"]
        pub struct SparseVec<T> {
            first_empty: usize,
            array: Vec<Entry<T>>,
        }
    },
    {
        #[doc = "/**\n * A `SparseVec` is a dynamic array of items of type `T` which allow holes\n * inside its structure. New items are stored preferentially in existing holes\n * instead of making the array bigger.\n * This allows fast deletion without modification of the indices of other\n * items.\n */"]
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
                    first_empty: metavar![0xa8c5ece631aca36],
                    array: metavar![0xc2b3ca8810763028],
                }
            },
            {
                SparseVec {
                    first_empty: metavar![0xa8c5ece631aca36],
                    array: metavar![0xc2b3ca8810763028],
                    dummy: (),
                }
            }
        ]
    }
    pub fn with_capacity(capacity: usize) -> SparseVec<T> {
        changed![
            {
                SparseVec {
                    first_empty: metavar![0xa8c5ece631aca36],
                    array: metavar![0xe3918999064c5b9e],
                }
            },
            {
                SparseVec {
                    first_empty: metavar![0xa8c5ece631aca36],
                    array: metavar![0xe3918999064c5b9e],
                    dummy: (),
                }
            }
        ]
    }
    unchanged![0xd18e746078c1a70d];
    unchanged![0x306c98f7342d1f0b];
    unchanged![0x87e9640b4948548d];
    pub fn remove(&mut self, index: usize) -> Option<T> {
        unchanged![0x579d22dffe11b5fb];
        match unchanged![0xc221cd9e526f8bc5] {
            Entry::Full(_) => {
                deleted![
                    use core::mem::replace;
                ];
                let old_entry = changed![
                    { replace(metavar![0xc221cd9e526f8bc5], metavar![0xb18e3be023b428f8]) },
                    { *metavar![0xc221cd9e526f8bc5] }
                ];
                inserted ! [ * metavar ! [ 0xc221cd9e526f8bc5 ] = metavar ! [ 0xb18e3be023b428f8 ] ; ];
                unchanged![0x4fe86435608f71a2];
                unchanged![0xc97277ded6b65060];
            }
            Entry::Empty(_) => unchanged![0x6c093692ac0a6dcb],
        }
    }
    unchanged![0x711d5298d08058b6];
    unchanged![0xc3b2e307117a893e];
    unchanged![0x32a2d78252b91212];
    unchanged![0xc188a7afd17eaf2b];
}
unchanged![0x42e20c2411b2783f];
unchanged![0x16ef8715f0082c14];
