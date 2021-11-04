fn f(c: bool) -> i32 {
    let x;
    if (c) {
        x = 3;
        x = x * 2;
    } else {
        x = 1;
        x = x * 2;
    }
    x + 1
}
