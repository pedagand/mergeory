trait Meowing {
    fn meow() -> String;
    fn purr();
}

struct Cat;

impl Meowing for Cat {
    fn meow() -> String {
        String::from("meow")
    }
    fn purr() {}
}
