trait Meowing {
    unchanged![];
    inserted![
        fn purr();
    ];
}
unchanged![];
impl Meowing for Cat {
    unchanged![];
    inserted![
        fn purr() {}
    ];
}
