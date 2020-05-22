fn f(c: bool) {
    changed![
        {
            if metavar![0] {
            } else {
            }
        },
        { if metavar![0] {} }
    ];
    unchanged![];
}
