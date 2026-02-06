use std::io::{Read, stdin};
use std::thread;
use std::time::Duration;

use termios::*;

const STDIN_FD: i32 = 0;

#[derive(Copy, Clone)]
pub enum Command {
    Help,
    Quit,
    SelectDiskA,
    SelectDiskB,
    ShowStatus,
    TraceCPU,
    SaveMemory,
    SetSpeed,
}

pub struct Keyboard {
    initial_termios: Option<Termios>,
    key_buffer: Vec<u8>,  // Buffer for queued keys
    pub commands: Vec<Command>,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        // Prepare terminal
        let initial_termios = Termios::from_fd(STDIN_FD).ok();

        let c = Keyboard {
            initial_termios,
            key_buffer: Vec::new(),
            commands: Vec::<Command>::new(),
        };

        c.setup_host_terminal(false);
        c
    }

    fn setup_host_terminal(&self, blocking: bool) {
        if let Some(mut initial) = self.initial_termios {
            initial.c_iflag &= !(IXON | ICRNL);
            initial.c_lflag &= !(ISIG | ECHO | ICANON | IEXTEN);
            initial.c_cc[VMIN] = if blocking {1} else {0};
            initial.c_cc[VTIME] = 0;
            tcsetattr(STDIN_FD, TCSANOW, &initial).unwrap();
        }
    }

    pub fn is_key_pressed(&mut self) -> bool {
        self.consume_input();
        if self.key_buffer.is_empty() {
            // Avoid 100% CPU usage waiting for input.
            thread::sleep(Duration::from_nanos(100));
        }
        !self.key_buffer.is_empty()
    }

    pub fn get_key(&mut self) -> u8 {
        self.consume_input();
        if self.key_buffer.is_empty() {
            0
        } else {
            self.key_buffer.remove(0)
        }
    }

    pub fn peek_key(&mut self) -> u8 {
        *self.key_buffer.first().unwrap_or(&0)
    }

    pub fn read_line(&mut self) -> Option<String> {
        use std::io::Write;
        
        // Use raw mode to detect ESC
        self.setup_host_terminal(true); // blocking mode
        
        let mut buffer = String::new();
        let mut buf = [0u8; 1];
        
        loop {
            if stdin().read(&mut buf).unwrap_or(0) == 1 {
                match buf[0] {
                    0x1b => { // ESC - cancel
                        self.setup_host_terminal(false);
                        return None;
                    }
                    0x0d | 0x0a => { // Enter
                        println!(); // newline
                        self.setup_host_terminal(false);
                        return Some(buffer);
                    }
                    0x7f | 0x08 => { // Backspace/Delete
                        if !buffer.is_empty() {
                            buffer.pop();
                            print!("\x08 \x08"); // erase character
                            std::io::stdout().flush().unwrap();
                        }
                    }
                    c if c >= 0x20 && c < 0x7f => { // Printable
                        buffer.push(c as char);
                        print!("{}", c as char);
                        std::io::stdout().flush().unwrap();
                    }
                    _ => {} // Ignore other control chars
                }
            }
        }
    }

    pub fn consume_input(&mut self) {
        let mut buf = [0;100];
        let size = stdin().read(&mut buf).unwrap_or(0);
        if size > 0 {
            self.parse_input(size, &buf);
        }
    }

    fn parse_input(&mut self, size: usize, input: &[u8]) {
        if size == 0 {
            // No new keys
        } else if size > 2 && input[0] == 0x1b {
            // Escape sequences
            // See 5.4 in the ECMA-48 spec
            let mut seq = "".to_owned();
            // Second byte of the CSI
            seq.push(input[1] as char);
            let mut i = 2;
            // Parameter and Intermediate bytes
            while i < size && (
                    input[i] & 0xf0 == 0x20 ||
                    input[i] & 0xf0 == 0x30 ) {
                seq.push(input[i] as char);
                i += 1;
            }
            // Final byte
            if i < size {
                seq.push(input[i] as char);
                i += 1;
            }
            // Debug: uncomment to see escape sequences
            // println!("Escape sequence: {:?}", seq);

            // Execute "showkey -a" to find the key codes
            match seq.as_str() {
                "OP" | "Op" => { // F1 (Linux, macOS application mode)
                    self.commands.push(Command::Help);
                }
                "OQ" | "Oq" => { // F2 (Linux, macOS application mode)
                    self.commands.push(Command::ShowStatus);
                }
                "OS" | "Os" => { // F4 (Linux, macOS application mode)
                    self.commands.push(Command::Quit);
                }
                "[15~" | "Ot" => { // F5 (Linux, macOS application mode)
                    self.commands.push(Command::SelectDiskA);
                }
                "[17~" | "Ou" => { // F6 (Linux, macOS application mode)
                    self.commands.push(Command::SelectDiskB);
                }
                "[18~" | "Ov" => { // F7 (Linux, macOS application mode)
                    self.commands.push(Command::SaveMemory);
                }
                "[19~" | "Ol" => { // F8 (Linux, macOS application mode)
                    self.commands.push(Command::TraceCPU);
                }
                "[20~" | "Ow" => { // F9 (Linux, macOS application mode)
                    self.commands.push(Command::SetSpeed);
                }
                "[3~" => {
                    // "Delete" key mapped to "DEL"
                    self.key_buffer.push(0x7f);
                }
                "[2~" => {
                    // "Insert" key mapped to "LINEFEED"
                    self.key_buffer.push(0x0a);
                }
                "[A" => {
                    // Up arrow mapped to ^K on the BIOS
                    self.key_buffer.push(0xf1); //0x0b
                }
                "[B" => {
                    // Down arrow mapped to ^J on the BIOS
                    self.key_buffer.push(0xf2); //0x0a
                }
                "[C" => {
                    // Right arrow mapped to ^L on the BIOS
                    self.key_buffer.push(0xf4); //0x0c
                }
                "[D" => {
                    // Left arrow mapped to ^H on the BIOS
                    self.key_buffer.push(0xf3); //0x08
                }
                _ => {}
            }
            // Parse the rest
            self.parse_input(size-i, &input[i..]);
        } else if size >= 2 && input[0] == 0xc3 && input[1] == 0xb1 {
            self.key_buffer.push(b':'); // ñ is on the : position
            self.parse_input(size-2, &input[2..]);
        } else if size >= 2 && input[0] == 0xc3 && input[1] == 0x91 {
            self.key_buffer.push(b';'); // Ñ is on the ; position
            self.parse_input(size-2, &input[2..]);
        } else {
            let key = match input[0] {
                0x7f => 0x08, // Backspace to ^H
                k => k & 0x7f,
            };
            self.key_buffer.push(key);
            // Parse the rest
            self.parse_input(size-1, &input[1..]);
        }
    }
}


impl Drop for Keyboard {
    fn drop(&mut self) {
        if let Some(initial) = self.initial_termios {
            tcsetattr(STDIN_FD, TCSANOW, &initial).unwrap();
        }
    }
}
