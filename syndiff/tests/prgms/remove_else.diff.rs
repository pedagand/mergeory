fn f(c: bool) {
    changed![
        {
            if mv![0] {
            } else {
            }
        },
        { if mv![0] {} }
    ];
    unchanged![];
}
