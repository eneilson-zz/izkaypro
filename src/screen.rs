use std::io::{stdout, Write};
use super::KayproMachine;
use super::kaypro_machine::VideoMode;

pub struct Screen {
    in_place: bool,
    last_system_bits: u8,
    pub show_status: bool,
    pub show_help: bool,
}

#[allow(dead_code)]
const CONTROL_CHARS_81_146A: [char; 32] = [
    '`', 'α', 'β', 'γ', 'δ', 'ϵ', 'ϕ', 'ν',
    'θ', 'ι', 'σ', 'κ', 'λ', 'μ', 'υ', 'ω',
    'π', 'η', 'ρ', 'Σ', 'τ', 'χ', 'ψ', '≠',
    'Ξ', 'Ω', 'ζ', '{', '|', '}', '~', '█'];

#[allow(dead_code)]
const CONTROL_CHARS_81_234: [char; 32] = [
    'ñ', 'á', 'é', 'í', 'ó', 'ú', 'â', 'ê',
    'î', 'ô', 'û', '£', 'Ä', 'Ö', 'Ü', '¡',
    'Ñ', 'à', 'è', 'ì', 'ò', 'ù', 'ä', 'ë',
    'ï', 'ö', 'ü', 'º', '§', 'c', 'ß', '¿'];
    

const SHOWN_SYSTEM_BITS: u8 = 0b0110_0011;

impl Screen {
    pub fn new(in_place: bool) -> Screen {
        Screen {
            in_place,
            last_system_bits: 0,
            show_status: false,
            show_help: false,
        }
    }

    pub fn init(&self) {
        if self.in_place {
            for _ in 0..27 {
                println!();
            }
        }
    }

    pub fn set_in_place(&mut self, in_place: bool) {
        self.in_place = in_place;
    }

    pub fn message(&mut self, machine: &mut KayproMachine, message:  &str) {
        if self.in_place {
            print!("\x1b[{}A", 14);
            println!("//==================================================================================\\\\");
            println!("||                                                                                  ||");
            println!("\\\\================================================ Press enter to continue =========//");
            print!("\x1b[{}A", 2);
            print!("|| {} ", message);
            stdout().flush().unwrap();
            machine.keyboard.read_line();
            print!("\x1b[{}B", 13);
            self.update(machine, true);
        } else {
            print!("{}: ", message);
        }
    }

    pub fn prompt(&mut self, machine: &mut KayproMachine, message: &str) -> Option<String> {
        if self.in_place {
            print!("\x1b[{}A", 20);
            println!("//==================================================================================\\\\");
            println!("||                                                                    (ESC cancels) ||");
            println!("\\\\==================================================================================//");
            print!("\x1b[{}A", 2);
            print!("|| {}: ", message);
            stdout().flush().unwrap();
            let line = machine.keyboard.read_line();
            print!("\x1b[{}B", 19);
            self.update(machine, true);
            line
        } else {
            print!("{} (ESC cancels): ", message);
            stdout().flush().unwrap();
            machine.keyboard.read_line()
        }
    }

    pub fn update(&mut self, machine: &mut KayproMachine, force: bool) {
        // Check if we need to update based on video mode
        let vram_dirty = if machine.video_mode == VideoMode::Sy6545Crtc {
            machine.crtc.vram_dirty
        } else {
            machine.vram_dirty
        };
        
        let relevant_system_bits = machine.system_bits & SHOWN_SYSTEM_BITS;
        if !force && !vram_dirty && self.last_system_bits == relevant_system_bits {
            return;
        }
        self.last_system_bits = relevant_system_bits;

        // Move cursor up with ansi escape sequence
        if self.in_place {
            print!("\x1b[{}A", 26);
        }

        let mut disk_status = "======".to_owned();
        if self.show_status && machine.floppy_controller.motor_on {
            if machine.floppy_controller.drive == 0 {
                disk_status = " A".to_owned();
            } else {
                disk_status = " B".to_owned();
            }
            if machine.floppy_controller.single_density {
                disk_status += " SD ";
            } else {
                disk_status += " DD ";
            }
        }

        if self.show_status {
            println!("//====Last key: 0x{:02x}================================================================\\\\", machine.keyboard.peek_key());
        } else {
            println!("//==================================================================================\\\\");
        }
        
        // For CRTC mode, display uses 128-byte stride starting at 0x300
        // (based on original working implementation from thread summary)
        for row in 0..24 {
            print!("|| ");
            for col in 0..80 {
                let code = if machine.video_mode == VideoMode::Sy6545Crtc {
                    // CRTC mode: linear 80-byte rows from start_addr (R12:R13)
                    // VRAM wraps at 2KB (0x800) for hardware scrolling
                    let start = machine.crtc.start_addr();
                    let addr = (start + row * 80 + col) & 0x7FF; // 2KB wrap
                    machine.crtc.get_vram(addr)
                } else {
                    // Memory-mapped mode: 128-byte stride from 0x0
                    machine.vram[(row * 128 + col) as usize]
                };
                let ch = translate_char(code);
                if code & 0x80 == 0 {
                    print!("{}", ch);
                } else {
                    // Blinking
                    print!("\x1b[5m{}\x1b[25m", ch);
                }
            }
            println!(" ||");
        }
        println!("\\\\======{}==================================== F1 for help ==== F4 to exit ====//", disk_status);

        if self.show_help {
            self.update_help(machine)
        }
        
        // Clear dirty flag on appropriate VRAM
        if machine.video_mode == VideoMode::Sy6545Crtc {
            machine.crtc.vram_dirty = false;
        } else {
            machine.vram_dirty = false;
        }
    }

    fn update_help (&mut self, machine: &KayproMachine) {
        if self.in_place {
            print!("\x1b[{}A", 21);
        }
        println!("||        +----------------------------------------------------------------+        ||");
        println!("||        |  izkaypro: Kaypro II emulator for console terminals            |        ||");
        println!("||        |----------------------------------------------------------------|        ||");
        println!("||        |  F1: Show/hide help           | Host keys to Kaypro keys:      |        ||");
        println!("||        |  F2: Show/hide disk status    |  Delete to DEL                 |        ||");
        println!("||        |  F4: Quit the emulator        |  Insert to LINEFEED            |        ||");
        println!("||        |  F5: Select file for drive A: |                                |        ||");
        println!("||        |  F6: Select file for drive B: |                                |        ||");
        println!("||        |  F7: Save BIOS to file        |                                |        ||");
        println!("||        |  F8: Toggle CPU trace         |                                |        ||");
        println!("||        +----------------------------------------------------------------+        ||");
        println!("||        |  Loaded images:                                                |        ||");
        println!("||        |  A: {:58} |        ||", machine.floppy_controller.media_a().info());
        println!("||        |  B: {:58} |        ||", machine.floppy_controller.media_b().info());
        println!("||        +----------------------------------------------------------------+        ||");

        if self.in_place {
            print!("\x1b[{}B", 21-7);
        }
    }


}

fn translate_char(code: u8) -> char {
    let index = code & 0x7f;
    if index < 0x20 {
        CONTROL_CHARS_81_234[index as usize]
    } else if index == 0x7f {
        '▒'
    } else {
        index as char
    }
}
