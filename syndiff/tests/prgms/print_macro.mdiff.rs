fn main() {
    changed![{ println!("{}", 42) }, {
        conflict![{ println!("answer = {}", 42) }, { println!("{}", 21 * 2) }]
    }];
}
