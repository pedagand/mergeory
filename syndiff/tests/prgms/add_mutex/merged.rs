fn f() {
    println!("Hello world!");

    let res = answer();
    println!("{}", res)
}

fn answer() -> i32 {
    mutex_lock();
    let a = 2;
    let b = 40;
    mutex_unlock();
    a + b
}
