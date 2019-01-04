extern crate lazy_static;
#[macro_use]
extern crate maplit;
extern crate rand;
extern crate sdl2;

use rand::Rng;
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::WindowCanvas, EventPump,
};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

mod font;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;
const SCALE: u32 = 10;

const SIXTY_HERTZ: Duration = Duration::from_millis(16u64);

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
    sys_time: SystemTime,
}

impl Chip8 {
    pub fn new() -> Self {
        let mut memory = [0; 4096];
        memory[0..font::FONT.len()].copy_from_slice(&font::FONT);
        Chip8 {
            memory,
            v: [0; 16],
            i: 0,
            pc: 0x200,
            screen: Screen::new(),
            delay_timer: 60,
            stack: [0; 16],
            sp: 0,
            key: [false; 16],
            sys_time: SystemTime::now(),
        }
    }

    pub fn load_game(&mut self, game: &[u8]) {
        self.memory[0x200..(0x200 + game.len())].copy_from_slice(game);
    }

    pub fn run(&mut self) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window(
                "chip80 gaming",
                (WIDTH as u32) * SCALE,
                (HEIGHT as u32) * SCALE,
            )
            .position_centered()
            .build()
            .unwrap();
        let mut canvas = window.into_canvas().build().unwrap();
        canvas.present();
        let mut event_pump = sdl_context.event_pump().unwrap();

        'running: loop {
            self.emulate_cycle();
            if self.do_tick() {
                self.delay_timer = if self.delay_timer > 0 { self.delay_timer - 1 } else { 0 };
                self.update_display(&mut canvas);
            }
            if self.set_key(&mut event_pump) {
                break 'running;
            }
        }
    }

    fn do_tick(&mut self) -> bool {
        let elapsed = self
            .sys_time
            .elapsed()
            .expect("System time went backwards!");

        let mut tick = false;
        if elapsed >= SIXTY_HERTZ {
            tick = true;
            self.sys_time = SystemTime::now();
        }
        tick
    }

    fn emulate_cycle(&mut self) {
        let opcode = (u16::from(self.memory[self.pc as usize]) << 8)
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
            0x7000 => self.v[x as usize] = u8::wrapping_add(self.v[x as usize], nn),
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
                    self.v[0xF as usize] = self.v[x as usize] & 0x80;
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
                0x1E => {
                    self.i += self.v[x as usize] as u16;
                    self.v[0xF] = if self.i > 0xFFF { 1 } else { 0 };
                }
                0x29 => self.i = self.v[x as usize] as u16 * 5,
                0x33 => {
                    self.memory[self.i as usize] = self.v[x as usize] / 100;
                    self.memory[self.i as usize + 1] = (self.v[x as usize] / 10) % 10;
                    self.memory[self.i as usize + 2] = (self.v[x as usize] % 100) % 10;
                }
                0x55 => self.memory[(self.i as usize)..(self.i + x as u16 + 1) as usize]
                    .copy_from_slice(&self.v[0..(x as usize + 1)]),
                0x65 => self.v[0..(x as usize + 1)].copy_from_slice(
                    &self.memory[(self.i as usize)..(self.i + x as u16 + 1) as usize],
                ),
                0xFF => println!("{:?}", self.v[x as usize]), // println value of register X for debugging
                _ => (),
            },
            _ => panic!("Unexpected opcode: {:x}", opcode),
        }
    }

    fn update_display(&self, canvas: &mut WindowCanvas) {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        for j in 0..(HEIGHT as i32) {
            for i in 0..(WIDTH as i32) {
                if self.screen.get(i as usize, j as usize) {
                    canvas
                        .fill_rect(Rect::new(
                            i * (SCALE as i32),
                            j * (SCALE as i32),
                            SCALE,
                            SCALE,
                        ))
                        .unwrap();
                }
            }
        }
        canvas.present();
    }

    fn set_key(&mut self, event_pump: &mut EventPump) -> bool {
        let mut quit = false;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => quit = true,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(mapped) = KEY_BINDINGS.get(&key) {
                        self.key[*mapped] = true;
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(mapped) = KEY_BINDINGS.get(&key) {
                        self.key[*mapped] = false;
                    }
                }
                _ => {}
            }
        }
        quit
    }
}

lazy_static::lazy_static! {
    static ref KEY_BINDINGS: HashMap<Keycode, usize> = maplit::hashmap! {
        Keycode::Num1 => 0,
        Keycode::Num2 => 1,
        Keycode::Num3 => 2,
        Keycode::Num4 => 3,
        Keycode::Q => 4,
        Keycode::W => 5,
        Keycode::E => 6,
        Keycode::R => 7,
        Keycode::A => 8,
        Keycode::S => 9,
        Keycode::D => 10,
        Keycode::F => 11,
        Keycode::Z => 12,
        Keycode::X => 13,
        Keycode::C => 14,
        Keycode::V => 15,
    };
}
