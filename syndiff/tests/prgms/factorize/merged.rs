fn f() {
    let y = answer() * 5;
    println!("{}", y)
}

fn answer() -> i32 {
    let a = 3;
    let b = 42;
    a + b
}
