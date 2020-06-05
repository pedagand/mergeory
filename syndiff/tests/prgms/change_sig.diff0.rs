changed![
    {
        fn f() -> i32 {
            mv![0]
        }
    },
    {
        fn f(n: i32) -> i32 {
            n
        }
    }
];
fn g() -> i32 {
    unchanged![];
    let x = changed![{ mv![1]() }, { mv![1](mv![0]) }];
    unchanged![];
}
