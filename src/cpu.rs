use crate::bus::Bus;
use std::fmt;

pub const PROGRAM_START: u16 = 0x200;

pub struct Cpu {
    vx: [u8; 16],
    pc: u16,
    i: u16,
    prev_pc: u16,
    ret_stack: Vec<u16>,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            vx: [0; 16],
            pc: PROGRAM_START,
            i: 0,
            prev_pc: 0,
            ret_stack: Vec::<u16>::new(),
        }
    }

    pub fn run_instruction(&mut self, bus: &mut Bus) {
        let hi = bus.ram_read_byte(self.pc) as u16;
        let lo = bus.ram_read_byte(self.pc + 1) as u16;
        let instruction: u16 = (hi << 8) | lo;
        println!(
            "instruction read:{:#X} hi:{:#X} lo:{:#X}",
            instruction, hi, lo
        );

        let nnn = instruction & 0x0FFF;
        let nn = (instruction & 0x00FF) as u8;
        let n = (instruction & 0x000F) as u8;
        let x = ((instruction & 0x0F00) >> 8) as u8;
        let y = ((instruction & 0x00F0) >> 4) as u8;

        println!("nnn:{:?} nn:{:?} n:{:?} x:{} y:{}", nnn, nn, n, x, y);

        if self.prev_pc == self.pc {
            panic!("Infinite loop detected at {:#X}. INCREMENT PC!", self.pc);
        }
        self.prev_pc = self.pc;

        match (instruction & 0xF000) >> 12 {
            0x1 => {
                // Goto to nnn
                self.pc = nnn;
            }
            0x2 => {
                // Call subroutine at nnn
                self.ret_stack.push(self.pc + 2);
                self.pc = nnn;
            }
            0x3 => {
                // Skip next instruction if VX == nn
                let vx = self.read_vx(x);
                if vx == nn {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x6 => {
                // Set VX to nn
                self.write_vx(x, nn as u8);
                self.pc += 2;
            }
            0x7 => {
                // Add nn to VX
                let vx = self.read_vx(x);
                self.write_vx(x, vx.wrapping_add(nn));
                self.pc += 2;
            }
            0x8 => {
                match n {
                    0x0 => {
                        // Set VX to VY
                        let vy = self.read_vx(y);
                        self.write_vx(x, vy);
                        self.pc += 2;
                    }
                    _ => panic!("Unknown instruction: {:#X}:{:#X}", self.pc, instruction),
                }
            }
            0xD => {
                // Draw sprite
                self.debug_draw_sprite(bus, x, y, n);
                self.pc += 2;
            }
            0xE => {
                match nn {
                    0xA1 => {
                        // Skip next instruction if key in VX is not pressed
                        let key = self.read_vx(x);
                        if bus.key_pressed(key) {
                            self.pc += 2; // i think this is wrong. change to 4 later
                        } else {
                            self.pc += 4;
                        }
                    }
                    _ => panic!("Unknown instruction: {:#X}:{:#X}", self.pc, instruction),
                }
            }
            0xA => {
                // Set I to nnn
                self.i = nnn;
                self.pc += 2;
            }
            0xF => {
                // I += VX
                let vx = self.read_vx(x);
                self.i += vx as u16;
                self.pc += 2;
            }

            _ => panic!("Unknown instruction: {:#X}:{:#X}", self.pc, instruction),
        }
    }

    fn debug_draw_sprite(&mut self, bus: &mut Bus, x: u8, y: u8, height: u8) {
        println!("Drawing sprite at ({}, {})", x, y);
        let mut y_coord = y;
        let mut should_set_vf = false;
        for y in 0..height {
            let byte = bus.ram_read_byte(self.i + y as u16);
            if bus.debug_draw_byte(byte, x, y) {
                should_set_vf = true;
            }
        }
        if should_set_vf {
            self.write_vx(0xF, 1);
        } else {
            self.write_vx(0xF, 0);
        }
    }

    fn write_vx(&mut self, x: u8, value: u8) {
        self.vx[x as usize] = value;
    }
    fn read_vx(&mut self, x: u8) -> u8 {
        self.vx[x as usize]
    }
}

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PC: {:#X}\n", self.pc)?;
        write!(f, "VX: ")?;
        for item in self.vx.iter() {
            write!(f, "{:#X} ", *item)?;
        }
        write!(f, "\n");
        write!(f, "i: {:#X}\n", self.i)
    }
}
