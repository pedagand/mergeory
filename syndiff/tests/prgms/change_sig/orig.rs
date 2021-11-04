fn f() -> i32 {
    42
}

fn g() -> i32 {
    let factor = 2;
    let x = f();
    factor * x
}
