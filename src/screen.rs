use std::io::{stdout, Write};
use super::KayproMachine;
use super::kaypro_machine::VideoMode;

pub struct Screen {
    in_place: bool,
    last_system_bits: u8,
    pub show_status: bool,
    pub show_help: bool,
    machine_name: String,
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
    pub fn new(in_place: bool, machine_name: &str) -> Screen {
        Screen {
            in_place,
            last_system_bits: 0,
            show_status: false,
            show_help: false,
            machine_name: machine_name.to_string(),
        }
    }
    
    /// Format the top border line with centered machine name
    fn format_title_line(&self) -> String {
        // Total width is 86 chars: "//" + 82 chars + "\\\\"
        let inner_width = 82;
        let name = &self.machine_name;
        let name_len = name.len();
        
        if name_len >= inner_width - 4 {
            // Name too long, just show equals
            format!("//{}\\\\", "=".repeat(inner_width))
        } else {
            // Center the name with equals on both sides
            let remaining = inner_width - name_len;
            let left_pad = remaining / 2;
            let right_pad = remaining - left_pad;
            format!("//{}{}{}\\\\", 
                "=".repeat(left_pad), 
                name, 
                "=".repeat(right_pad))
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
            let sio_status = machine.sio.status_string();
            println!("//====Last key: 0x{:02x}=={:>40}==============\\\\",
                machine.keyboard.peek_key(), sio_status);
        } else {
            println!("{}", self.format_title_line());
        }
        
        // Get cursor position for CRTC mode
        let (cursor_addr, cursor_visible) = if machine.video_mode == VideoMode::Sy6545Crtc {
            let addr = machine.crtc.cursor_addr() & 0x7FF; // Mask to 2KB VRAM
            let mode = machine.crtc.cursor_mode();
            // Mode 0 = steady, 1 = invisible, 2/3 = blink (we show steady for now)
            (addr, mode != 1)
        } else {
            (0xFFFF, false) // No cursor in memory-mapped mode (handled differently)
        };
        
        // For CRTC mode, display uses linear 80-byte rows from start_addr
        for row in 0..24 {
            print!("|| ");
            for col in 0..80 {
                let (code, attr, is_cursor) = if machine.video_mode == VideoMode::Sy6545Crtc {
                    // CRTC mode: linear 80-byte rows from start_addr (R12:R13)
                    // VRAM wraps at 2KB (0x800) for hardware scrolling
                    let start = machine.crtc.start_addr();
                    let addr = (start + row * 80 + col) & 0x7FF; // 2KB wrap
                    let at_cursor = cursor_visible && addr == cursor_addr;
                    (machine.crtc.get_vram(addr), machine.crtc.get_attr(addr), at_cursor)
                } else {
                    // Memory-mapped mode: 128-byte stride from 0x0
                    (machine.vram[(row * 128 + col) as usize], 0u8, false)
                };
                let ch = translate_char(code);
                
                // Attribute bits (Kaypro 2-84/4-84 Theory of Operation):
                // Bit 0: Reverse video
                // Bit 1: Half intensity (dim)
                // Bit 2: Blink
                // Bit 3: Underline (16th row only - not visible in terminal)
                let reverse = (attr & 0x01) != 0 || is_cursor;
                let dim = (attr & 0x02) != 0;
                // In CRTC mode, blink comes from attribute RAM bit 2
                // In memory-mapped mode, blink comes from character bit 7
                let blink = if machine.video_mode == VideoMode::Sy6545Crtc {
                    (attr & 0x04) != 0
                } else {
                    (code & 0x80) != 0
                };
                let underline = (attr & 0x08) != 0;
                
                // Build ANSI escape sequence for attributes
                let mut seq = String::new();
                if reverse { seq.push_str("\x1b[7m"); }
                if dim { seq.push_str("\x1b[2m"); }
                if blink { seq.push_str("\x1b[5m"); }
                if underline { seq.push_str("\x1b[4m"); }
                
                print!("{}{}\x1b[0m", seq, ch);
            }
            println!(" ||");
        }
        println!("\\\\======{}===================================== F1 for help ==== F4 to exit ====//", disk_status);

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
        println!("||        |  F9: Set CPU speed (MHz)      |                                |        ||");
        println!("||        +----------------------------------------------------------------+        ||");
        println!("||        |  Loaded images:                                                |        ||");
        println!("||        |  A: {:58} |        ||", machine.floppy_controller.media_a().info());
        println!("||        |  B: {:58} |        ||", machine.floppy_controller.media_b().info());
        println!("||        +----------------------------------------------------------------+        ||");

        if self.in_place {
            print!("\x1b[{}B", 21-8);
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
