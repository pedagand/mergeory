fn f() {
    println!("Hello world!");

    mutex_lock();
    let a = 2;
    let b = 40;
    mutex_unlock();

    let res = a + b;
    println!("{}", res)
}
