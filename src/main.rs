extern crate chip8;

use chip8::Chip8;

const GAME: [u8; 0] = [];

fn main() {
    let mut emulator = Chip8::new();
    emulator.load_game(&GAME);
    emulator.run();
}
