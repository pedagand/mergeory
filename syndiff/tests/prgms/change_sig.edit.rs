fn f(n: i32) -> i32 {
    n
}

fn g() -> i32 {
    let factor = 2;
    let x = f(42);
    factor * x
}
