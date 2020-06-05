fn f() {
    deleted ! [ mv ! [ 0 ] ; ];
    deleted ! [ mv ! [ 1 ] ; ];
    deleted ! [ let x = mv ! [ 2 ] ; ];
    let y = changed![{ x }, { answer() }] * unchanged![];
    unchanged![];
}
inserted![
    fn answer() -> i32 {
        mv![0];
        mv![1];
        mv![2]
    }
];
