fn f() {
    deleted ! [ let a = 2 ; ];
    deleted ! [ let b = 40 ; ];
    delete_conflict![
        {
            let x = mv_conflict![0, { mv![0] }, { mv![0] }];
        },
        {
            let x = mv![0];
        }
    ];
    let y = changed![{ x }, { answer() }] * changed![{ 2 }, { 5 }];
    unchanged![];
}
inserted![
    fn answer() -> i32 {
        let a = 3;
        let b = 42;
        mv![0]
    }
];
