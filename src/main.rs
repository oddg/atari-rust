extern crate chip8;

use chip8::Chip8;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let game = &args[1];

    let file = File::open(path::PathBuf::from(game)).expect("failed to read rom");

    let mut buffer = BufReader::new(file);
    let mut data: Vec<u8> = vec![];
    buffer.read_to_end(&mut data).expect("failed to read file");

    let game = &data[..];

    let mut emulator = Chip8::new();
    emulator.load_game(game);
    emulator.run();
}
