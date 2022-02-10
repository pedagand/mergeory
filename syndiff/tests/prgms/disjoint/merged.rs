use std::fs::read;
use std::process::exit;

fn answer() -> u32 {
    42
}

fn main() {
    let origin_src = read_file("file");
    println!("file content:\n{}", String::from_utf8_lossy(&origin_src));
    println!("Answer = {}", answer());
}

fn read_file(filename: &str) -> Vec<u8> {
    read(filename).unwrap_or_else(|err| {
        eprintln!("Unable to read {}: {}", filename, err);
        exit(-1)
    })
}
