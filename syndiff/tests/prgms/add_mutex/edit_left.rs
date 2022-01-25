fn f() {
    println!("Hello world!");

    let res = answer();
    println!("{}", res)
}

fn answer() -> i32 {
    let a = 2;
    let b = 40;
    a + b
}
