changed![
    {
        fn f() -> i32 {
            metavar![0x91023ba28dcede04]
        }
    },
    {
        fn f(n: i32) -> i32 {
            n
        }
    }
];
fn g() -> i32 {
    unchanged![0x4571a14d482b8c4f];
    let x = changed![{ metavar![0xe19d5c02a17c24d6]() }, {
        metavar![0xe19d5c02a17c24d6](metavar![0x91023ba28dcede04])
    }];
    unchanged![0xaed4e1a240b38c];
}
