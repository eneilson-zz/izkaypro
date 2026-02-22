use std::thread;
use std::time::Duration;

use windows_sys::Win32::System::Console::*;
use windows_sys::Win32::Foundation::HANDLE;

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
    stdin_handle: HANDLE,
    original_mode: u32,
    key_buffer: Vec<u8>,
    pub commands: Vec<Command>,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        unsafe {
            let stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
            let mut original_mode: u32 = 0;
            GetConsoleMode(stdin_handle, &mut original_mode);

            // Raw mode: disable line input, echo, and processed input (Ctrl+C).
            // Do NOT set ENABLE_VIRTUAL_TERMINAL_INPUT — that converts keys
            // to ANSI sequences instead of KEY_EVENT records with VK codes.
            SetConsoleMode(stdin_handle, ENABLE_WINDOW_INPUT);

            Keyboard {
                stdin_handle,
                original_mode,
                key_buffer: Vec::new(),
                commands: Vec::<Command>::new(),
            }
        }
    }

    pub fn is_key_pressed(&mut self) -> bool {
        self.consume_input();
        if self.key_buffer.is_empty() {
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

        let mut buffer = String::new();

        loop {
            // Block until at least one input event is available
            let mut events_available: u32 = 0;
            unsafe {
                GetNumberOfConsoleInputEvents(self.stdin_handle, &mut events_available);
            }
            if events_available == 0 {
                thread::sleep(Duration::from_millis(10));
                continue;
            }

            let mut record = [INPUT_RECORD {
                EventType: 0,
                Event: INPUT_RECORD_0 { KeyEvent: KEY_EVENT_RECORD {
                    bKeyDown: 0,
                    wRepeatCount: 0,
                    wVirtualKeyCode: 0,
                    wVirtualScanCode: 0,
                    uChar: KEY_EVENT_RECORD_0 { UnicodeChar: 0 },
                    dwControlKeyState: 0,
                }},
            }];
            let mut events_read: u32 = 0;
            unsafe {
                ReadConsoleInputW(self.stdin_handle, record.as_mut_ptr(), 1, &mut events_read);
            }
            if events_read == 0 {
                continue;
            }

            if record[0].EventType != KEY_EVENT as u16 {
                continue;
            }
            let key_event = unsafe { record[0].Event.KeyEvent };
            if key_event.bKeyDown == 0 {
                continue;
            }

            let ch = unsafe { key_event.uChar.UnicodeChar };
            match ch {
                0x1b => { // ESC - cancel
                    return None;
                }
                0x0d => { // Enter
                    println!();
                    return Some(buffer);
                }
                0x08 => { // Backspace
                    if !buffer.is_empty() {
                        buffer.pop();
                        print!("\x08 \x08");
                        std::io::stdout().flush().unwrap();
                    }
                }
                c if c >= 0x20 && c < 0x7f => {
                    buffer.push(c as u8 as char);
                    print!("{}", c as u8 as char);
                    std::io::stdout().flush().unwrap();
                }
                _ => {}
            }
        }
    }

    pub fn consume_input(&mut self) {
        loop {
            let mut events_available: u32 = 0;
            unsafe {
                GetNumberOfConsoleInputEvents(self.stdin_handle, &mut events_available);
            }
            if events_available == 0 {
                break;
            }

            let mut record = [INPUT_RECORD {
                EventType: 0,
                Event: INPUT_RECORD_0 { KeyEvent: KEY_EVENT_RECORD {
                    bKeyDown: 0,
                    wRepeatCount: 0,
                    wVirtualKeyCode: 0,
                    wVirtualScanCode: 0,
                    uChar: KEY_EVENT_RECORD_0 { UnicodeChar: 0 },
                    dwControlKeyState: 0,
                }},
            }];
            let mut events_read: u32 = 0;
            unsafe {
                ReadConsoleInputW(self.stdin_handle, record.as_mut_ptr(), 1, &mut events_read);
            }
            if events_read == 0 {
                break;
            }

            if record[0].EventType != KEY_EVENT as u16 {
                continue;
            }
            let key_event = unsafe { record[0].Event.KeyEvent };
            if key_event.bKeyDown == 0 {
                continue;
            }

            let vk = key_event.wVirtualKeyCode;
            let ch = unsafe { key_event.uChar.UnicodeChar };

            // Function keys → commands
            match vk as u32 {
                VK_F1 => { self.commands.push(Command::Help); continue; }
                VK_F2 => { self.commands.push(Command::ShowStatus); continue; }
                VK_F4 => { self.commands.push(Command::Quit); continue; }
                VK_F5 => { self.commands.push(Command::SelectDiskA); continue; }
                VK_F6 => { self.commands.push(Command::SelectDiskB); continue; }
                VK_F7 => { self.commands.push(Command::SaveMemory); continue; }
                VK_F8 => { self.commands.push(Command::TraceCPU); continue; }
                VK_F9 => { self.commands.push(Command::SetSpeed); continue; }
                _ => {}
            }

            // Arrow keys → Kaypro cursor codes
            match vk as u32 {
                VK_UP => { self.key_buffer.push(0xf1); continue; }
                VK_DOWN => { self.key_buffer.push(0xf2); continue; }
                VK_LEFT => { self.key_buffer.push(0xf3); continue; }
                VK_RIGHT => { self.key_buffer.push(0xf4); continue; }
                VK_DELETE => { self.key_buffer.push(0x7f); continue; }
                VK_INSERT => { self.key_buffer.push(0x0a); continue; }
                _ => {}
            }

            // Regular character input
            if ch > 0 {
                let key = match ch as u8 {
                    0x7f => 0x08, // DEL → Backspace
                    k => k & 0x7f,
                };
                self.key_buffer.push(key);
            }
        }
    }
}

impl Drop for Keyboard {
    fn drop(&mut self) {
        unsafe {
            SetConsoleMode(self.stdin_handle, self.original_mode);
        }
    }
}

// Windows Virtual Key codes
const VK_DELETE: u32 = 0x2E;
const VK_INSERT: u32 = 0x2D;
const VK_UP: u32 = 0x26;
const VK_DOWN: u32 = 0x28;
const VK_LEFT: u32 = 0x25;
const VK_RIGHT: u32 = 0x27;
const VK_F1: u32 = 0x70;
const VK_F2: u32 = 0x71;
const VK_F4: u32 = 0x73;
const VK_F5: u32 = 0x74;
const VK_F6: u32 = 0x75;
const VK_F7: u32 = 0x76;
const VK_F8: u32 = 0x77;
const VK_F9: u32 = 0x78;
