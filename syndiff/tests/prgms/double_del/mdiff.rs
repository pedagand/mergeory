DELETE_CONFLICT![«fn f() -> i32 {
    let mut a = 0;
    a += 1;
    a
}» -/> «fn f() -> i32 {
    let mut a = 0;
    a
}»]DELETED![
]