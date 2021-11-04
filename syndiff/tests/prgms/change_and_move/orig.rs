fn test() {
    if b {
        f(0);
        x
    } else {
        f(42);
        g();
        g();
    }
}
