fn f() {
    let a = changed![{ 2 }, { 3 }];
    let b = changed![{ 40 }, { 42 }];
    unchanged![];
    let y = unchanged![] * changed![{ 2 }, { 5 }];
    unchanged![];
}
