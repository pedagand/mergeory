fn f(c: bool) -> i32 {
    unchanged![];
    if unchanged![] {
        unchanged![];
        unchanged![0] = changed![{ mv![0] }, { g(mv![0]) }] * unchanged![];
    } else {
        unchanged![];
        unchanged![0] = changed![{ mv![0] }, { g(mv![0]) }] * unchanged![];
    }
    unchanged![];
}
