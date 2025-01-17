use crate::bus::Bus;
use rand::distributions::Distribution;
use rand::rngs::ThreadRng;
use std::fmt;

pub const PROGRAM_START: u16 = 0x200;

pub struct Cpu {
    vx: [u8; 16],
    pc: u16,
    i: u16,
    prev_pc: u16,
    ret_stack: Vec<u16>,
    rng: ThreadRng,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            vx: [0; 16],
            pc: PROGRAM_START,
            i: 0,
            prev_pc: 0,
            ret_stack: Vec::<u16>::new(),
            rng: rand::thread_rng(),
        }
    }

    pub fn run_instruction(&mut self, bus: &mut Bus) {
        let hi = bus.ram_read_byte(self.pc) as u16;
        let lo = bus.ram_read_byte(self.pc + 1) as u16;
        let instruction: u16 = (hi << 8) | lo;
        println!(
            "Instruction read {:#X}:{:#X}: hi{:#X} lo:{:#X} ",
            self.pc, instruction, hi, lo
        );

        let nnn = instruction & 0x0FFF;
        let nn = (instruction & 0x0FF) as u8;
        let n = (instruction & 0x00F) as u8;
        let x = ((instruction & 0x0F00) >> 8) as u8;
        let y = ((instruction & 0x00F0) >> 4) as u8;
        println!("nnn={:?}, nn={:?}, n={:?} x={}, y={}", nnn, nn, n, x, y);

        if self.prev_pc == self.pc {
            panic!("Please increment PC!");
        }
        self.prev_pc = self.pc;

        match (instruction & 0xF000) >> 12 {
            0x0 => {
                match nn {
                    0xE0 => {
                        bus.clear_screen();
                        self.pc += 2;
                    }
                    0xEE => {
                        //return from subroutine
                        let addr = self.ret_stack.pop().unwrap();
                        self.pc = addr;
                    }
                    _ => panic!(
                        "Unrecognized 0x00** instruction {:#X}:{:#X}",
                        self.pc, instruction
                    ),
                }
            }
            0x1 => {
                //goto nnn;
                self.pc = nnn;
            }
            0x2 => {
                //Call subroutine at address NNN
                self.ret_stack.push(self.pc + 2);
                self.pc = nnn;
            }
            0x3 => {
                //if(Vx==NN)
                let vx = self.read_vx(x);
                if vx == nn {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x4 => {
                //if(Vx!=NN)
                let vx = self.read_vx(x);
                if vx != nn {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x5 => {
                //Skip next instruction if(Vx==Vy)
                let vx = self.read_vx(x);
                let vy = self.read_vx(y);
                if vx == vy {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0x6 => {
                //vx = nn
                self.write_vx(x, nn);
                self.pc += 2;
            }
            0x7 => {
                let vx = self.read_vx(x);
                self.write_vx(x, vx.wrapping_add(nn));
                self.pc += 2;
            }
            0x8 => {
                let vy = self.read_vx(y);
                let vx = self.read_vx(x);

                match n {
                    0 => {
                        // Vx=Vy
                        self.write_vx(x, vy);
                    }
                    2 => {
                        // Vx=Vx&Vy
                        self.write_vx(x, vx & vy);
                    }
                    3 => {
                        // Vx=Vx^Vy
                        self.write_vx(x, vx ^ vy);
                    }
                    4 => {
                        //	Vx += Vy
                        let sum: u16 = vx as u16 + vy as u16;
                        self.write_vx(x, sum as u8);
                        if sum > 0xFF {
                            self.write_vx(0xF, 1);
                        }
                    }
                    5 => {
                        let diff: i8 = vx as i8 - vy as i8;
                        self.write_vx(x, diff as u8);
                        if diff < 0 {
                            self.write_vx(0xF, 1);
                        }
                    }
                    6 => {
                        // Vx=Vy=Vy>>1
                        self.write_vx(0xF, vy & 0x1);
                        self.write_vx(y, vy >> 1);
                        self.write_vx(x, vy >> 1);
                    }
                    0x7 => {
                        let diff: i8 = vy as i8 - vx as i8;
                        self.write_vx(x, diff as u8);
                        if diff < 0 {
                            self.write_vx(0xF, 1);
                        } else {
                            self.write_vx(0xF, 0);
                        }
                    }
                    0xE => {
                        // VF is the most significant bit value.
                        // SHR Vx
                        self.write_vx(0xF, (vx & 0x80) >> 7);
                        self.write_vx(x, vx << 1);
                    }
                    _ => panic!(
                        "Unrecognized 0x8XY* instruction {:#X}:{:#X}",
                        self.pc, instruction
                    ),
                };

                self.pc += 2;
            }
            0x9 => {
                //skips the next instruction if(Vx!=Vy)
                let vx = self.read_vx(x);
                let vy = self.read_vx(y);
                if vx != vy {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            }
            0xA => {
                self.i = nnn;
                self.pc += 2;
            }
            0xB => {
                self.pc = self.read_vx(0) as u16 + nnn;
            }
            0xC => {
                // Vx=rand() & NN
                let interval = rand::distributions::Uniform::new(0, 255);
                let number = interval.sample(&mut self.rng);
                self.write_vx(x, number & nn);
                self.pc += 2;
            }
            0xD => {
                // Draw sprite
                let vx = self.read_vx(x);
                let vy = self.read_vx(y);
                self.debug_draw_sprite(bus, vx, vy, n);
                self.pc += 2;
            }
            0xE => {
                match nn {
                    0xA1 => {
                        // Skip next instruction if key in VX is not pressed
                        let key = self.read_vx(x);
                        if !bus.key_pressed(key) {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    0x9E => {
                        // Skip next instruction if key in VX is pressed
                        let key = self.read_vx(x);
                        if bus.key_pressed(key) {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    }
                    _ => panic!(
                        "Unrecognized 0xEX** instruction {:#X}:{:#X}",
                        self.pc, instruction
                    ),
                };
            }
            0xF => {
                match nn {
                    0x07 => {
                        self.write_vx(x, bus.get_delay_timer());
                        self.pc += 2;
                    }
                    0x0A => {
                        if let Some(val) = bus.get_key_pressed() {
                            self.write_vx(x, val);
                            self.pc += 2;
                        }
                    }
                    0x15 => {
                        bus.set_delay_timer(self.read_vx(x));
                        self.pc += 2;
                    }
                    0x18 => {
                        // TODO Sound timer
                        self.pc += 2;
                    }
                    0x1E => {
                        //I +=Vx
                        let vx = self.read_vx(x);
                        self.i += vx as u16;
                        self.pc += 2;
                    }
                    0x29 => {
                        //i == sprite address for character in Vx
                        //Multiply by 5 because each sprite has 5 lines, each line
                        //is 1 byte.
                        self.i = self.read_vx(x) as u16 * 5;
                        self.pc += 2;
                    }
                    0x33 => {
                        let vx = self.read_vx(x);
                        bus.ram_write_byte(self.i, vx / 100);
                        bus.ram_write_byte(self.i + 1, (vx % 100) / 10);
                        bus.ram_write_byte(self.i + 2, vx % 10);
                        self.pc += 2;
                    }
                    0x55 => {
                        for index in 0..x + 1 {
                            let value = self.read_vx(index);
                            bus.ram_write_byte(self.i + index as u16, value);
                        }
                        self.i += x as u16 + 1;
                        self.pc += 2;
                    }
                    0x65 => {
                        for index in 0..x + 1 {
                            let value = bus.ram_read_byte(self.i + index as u16);
                            self.write_vx(index, value);
                        }
                        self.i += x as u16 + 1;
                        self.pc += 2;
                    }
                    _ => panic!(
                        "Unrecognized 0xF instruction {:#X}:{:#X}",
                        self.pc, instruction
                    ),
                }
            }

            _ => panic!("Unrecognized instruction {:#X}:{:#X}", self.pc, instruction),
        }
    }

    fn debug_draw_sprite(&mut self, bus: &mut Bus, x: u8, y: u8, height: u8) {
        let mut should_set_vf = false;
        for sprite_y in 0..height {
            let b = bus.ram_read_byte(self.i + sprite_y as u16);
            if bus.debug_draw_byte(b, x, y + sprite_y) {
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
