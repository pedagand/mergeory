insert_order_conflict![
    {
        mv![0];
    },
    {
        mv![1];
    }
];
changed![
    {
        mv_conflict![
            2,
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
        mv![2];
    }
];
deleted ! [ mv ! [ 0 ] ; ];
deleted ! [ mv_conflict ! [ 1 , { fn two ( ) -> i32 { 2 } } , { mv ! [ 2 ] ; } ] ; ];
