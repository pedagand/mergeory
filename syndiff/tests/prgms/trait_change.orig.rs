trait Meowing {
    fn meow() -> String;
}

struct Cat;

impl Meowing for Cat {
    fn meow() -> String {
        String::from("meow")
    }
}
