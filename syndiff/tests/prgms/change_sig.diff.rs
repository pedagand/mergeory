changed![
    {
        fn f() -> i32 {
            metavar![0]
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
    let x = changed![{ metavar![1]() }, { metavar![1](metavar![0]) }];
    unchanged![];
}
