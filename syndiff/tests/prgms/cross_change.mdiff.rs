changed![
    {
        mv_conflict![
            0,
            {
                fn zero() -> i32 {
                    0
                }
            },
            {
                mv![1];
            }
        ];
    },
    {
        fn i0() -> i32 {
            0
        }
    }
];
unchanged![];
changed![
    {
        mv_conflict![
            1,
            {
                fn two() -> i32 {
                    2
                }
            },
            {
                mv![0];
            }
        ];
    },
    {
        fn i2() -> i32 {
            2
        }
    }
];
