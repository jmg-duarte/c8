use std::env;
use std::fs;

mod chip8;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: chip-8 <file>");
        return;
    }
    let program = fs::read(&args[1]).expect("something went wrong when reading the file");
    println!("{:?}", program);
}
