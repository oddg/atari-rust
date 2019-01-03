#[allow(dead_code)]
extern crate rand;

use rand::Rng;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

struct Screen([bool; WIDTH * HEIGHT]);

impl Screen {
    fn new() -> Self {
        Screen([false; WIDTH * HEIGHT])
    }

    fn get(&self, x: usize, y: usize) -> bool {
        self.0[x + y * WIDTH]
    }

    fn set(&mut self, x: usize, y: usize, v: bool) {
        self.0[x + y * WIDTH] = v;
    }

    fn clear(&mut self) {
        for x in 0..WIDTH {
            for y in 0..HEIGHT {
                self.set(x, y, false)
            }
        }
    }

    // Draw a sprite at coordinate (x,y). Returns whether a pixel has been erased.
    fn draw(&mut self, x: usize, y: usize, sprite: &[u8]) -> bool {
        let mut flipped = false;
        for (j, row) in sprite.iter().enumerate() {
            for i in 0..8 {
                let x = (x + i) % WIDTH;
                let y = (y + j) % HEIGHT;
                let new = ((row >> (7 - i)) & 1u8) == 1;
                let old = self.get(x, y);
                if new && old {
                    flipped = true;
                }
                self.set(x, y, new ^ old);
            }
        }
        flipped
    }
}

pub struct Chip8 {
    memory: [u8; 4096],
    v: [u8; 16],
    i: u16,
    pc: u16,
    screen: Screen,
    delay_timer: u8,
    stack: [u16; 16],
    sp: u16,
    key: [bool; 16],
}

impl Chip8 {
    pub fn new() -> Self {
        Chip8 {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0,
            screen: Screen::new(),
            delay_timer: 0,
            stack: [0; 16],
            sp: 0,
            key: [false; 16],
        }
    }

    pub fn load_game(&mut self, game: &[u8]) {
        unimplemented!()
    }

    pub fn run(&mut self) {
        loop {
            self.emulate_cycle();
            self.update_display();
            self.set_key();
        }
    }

    fn emulate_cycle(&mut self) {
        let opcode = u16::from(self.memory[self.pc as usize] << 8)
            + u16::from(self.memory[(self.pc + 1) as usize]);
        self.pc += 2;

        let nnn = opcode & 0x0FFF;
        let nn = (opcode & 0x00FF) as u8;
        let n = (opcode & 0x000F) as u8;
        let x = (opcode & 0x0F00) >> 8;
        let y = (opcode & 0x00F0) >> 4;

        match opcode & 0xF000 {
            0x0000 => match opcode & 0x0FFF {
                0x00E0 => self.screen.clear(),
                0x00EE => {
                    self.sp -= 1;
                    self.pc = self.stack[self.sp as usize];
                }
                _ => panic!("Unexpected opcode: {:x}", opcode),
            },
            0x1000 => self.pc = nnn,
            0x2000 => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            }
            0x3000 => {
                if self.v[x as usize] == nn {
                    self.pc += 2
                }
            }
            0x4000 => {
                if self.v[x as usize] != nn {
                    self.pc += 2
                }
            }
            0x5000 => {
                if self.v[x as usize] == self.v[y as usize] {
                    self.pc += 2
                }
            }
            0x6000 => self.v[x as usize] = nn,
            0x7000 => self.v[x as usize] += nn,
            0x8000 => match n {
                0 => self.v[x as usize] = self.v[y as usize],
                1 => self.v[x as usize] |= self.v[y as usize],
                2 => self.v[x as usize] &= self.v[y as usize],
                3 => self.v[x as usize] ^= self.v[y as usize],

                4 => {
                    let (v, f) = self.v[x as usize].overflowing_add(self.v[y as usize]);
                    self.v[x as usize] = v;
                    self.v[0xF as usize] = if f { 1 } else { 0 };
                }
                5 => {
                    let (v, f) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
                    self.v[x as usize] = v;
                    self.v[0xF as usize] = if f { 0 } else { 1 };
                }
                6 => {
                    self.v[0xF as usize] = self.v[x as usize] & 0x1;
                    self.v[x as usize] >>= 1;
                }
                7 => {
                    let (v, f) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
                    self.v[x as usize] = v;
                    self.v[0xF as usize] = if f { 0 } else { 1 };
                }
                0xE => {
                    self.v[0xF as usize] = self.v[x as usize] & 0xA0;
                    self.v[x as usize] <<= 1;
                }
                _ => (),
            },
            0x9000 => {
                if self.v[x as usize] != self.v[y as usize] {
                    self.pc += 2
                }
            }
            0xA000 => self.i = nnn,
            0xB000 => self.pc = self.v[0] as u16 + nnn,
            0xC000 => self.v[x as usize] = (rand::thread_rng().gen_range(0u16, 256) as u8) & nn,
            0xD000 => {
                let s = self.i as usize;
                let e = s + (n as usize);
                let flipped = self.screen.draw(
                    self.v[x as usize] as usize,
                    self.v[y as usize] as usize,
                    &self.memory[s..e],
                );
                self.v[0xF] = if flipped { 1 } else { 0 };
            }
            0xE000 => match nn {
                0x9E => {
                    if self.key[self.v[x as usize] as usize] {
                        self.pc += 2;
                    }
                }
                0xA1 => {
                    if !self.key[self.v[x as usize] as usize] {
                        self.pc += 2;
                    }
                }
                _ => (),
            },
            0xF000 => match nn {
                0x7 => self.v[x as usize] = self.delay_timer,
                0xA => (), // v[x] = sound timer - unimplemented
                0x15 => self.delay_timer = self.v[x as usize],
                0x18 => (), // sound time = v[x] - unimplemented
                0x1E => self.i += self.v[x as usize] as u16,
                0x29 => unimplemented!(),
                0x33 => unimplemented!(),
                0x55 => unimplemented!(),
                0x65 => unimplemented!(),
                _ => (),
            },
            _ => panic!("Unexpected opcode: {:x}", opcode),
        }
    }

    fn update_display(&self) {
        unimplemented!()
    }

    fn draw(&self, n: u8) {
        unimplemented!()
    }

    fn set_key(&mut self) {
        unimplemented!()
    }
}
