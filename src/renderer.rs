use std::fs;

use super::kaypro_machine::{KayproMachine, VideoMode};

// Blink cycle: ~1.28s at 60fps ≈ 77 frames per cycle
const BLINK_PERIOD: u32 = 77;

/// Phosphor color scheme for the chargen display.
#[derive(Clone, Copy)]
pub struct PhosphorColors {
    pub fg: u32,
    pub bg: u32,
    pub dim: u32,
}

impl PhosphorColors {
    pub fn from_name(name: &str) -> Option<PhosphorColors> {
        match name.to_lowercase().as_str() {
            "green" => Some(PHOSPHOR_GREEN),
            "amber" => Some(PHOSPHOR_AMBER),
            "white" => Some(PHOSPHOR_WHITE),
            "blue" => Some(PHOSPHOR_BLUE),
            _ => None,
        }
    }

    /// Parse a hex color string like "#33FF33" or "33FF33" into a 0x00RRGGBB u32.
    pub fn parse_hex(s: &str) -> Option<u32> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 {
            return None;
        }
        u32::from_str_radix(hex, 16).ok()
    }
}

// Green P1 phosphor (Kaypro default)
pub const PHOSPHOR_GREEN: PhosphorColors = PhosphorColors {
    fg:  0x0033FF33,
    bg:  0x00002200,
    dim: 0x001A801A,
};

// Amber P3 phosphor
pub const PHOSPHOR_AMBER: PhosphorColors = PhosphorColors {
    fg:  0x00FFB833,
    bg:  0x00221100,
    dim: 0x00805C1A,
};

// White P4 phosphor
pub const PHOSPHOR_WHITE: PhosphorColors = PhosphorColors {
    fg:  0x00E0E0E0,
    bg:  0x00181818,
    dim: 0x00707070,
};

// Cool blue phosphor
pub const PHOSPHOR_BLUE: PhosphorColors = PhosphorColors {
    fg:  0x0066BBFF,
    bg:  0x00001122,
    dim: 0x00335E80,
};

pub struct Renderer {
    chargen: Vec<u8>,
    scanlines_per_char: usize, // 8 or 16 depending on ROM
    /// Base offset in ROM where character data starts.
    /// 2KB ROM (81-146a): data at 0x400 (A10=1), inverted polarity.
    /// 4KB ROM (81-235/81-187): data at 0x000, normal polarity.
    chargen_base: usize,
    /// True if ROM uses inverted pixel polarity (0=lit, 1=dark).
    inverted_polarity: bool,
    framebuffer: Vec<u32>,
    /// Display buffer with scanline doubling applied (for 8-row ROMs).
    display_buffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    /// True when scanlines are doubled for CRT aspect ratio (8-row ROMs).
    scanline_double: bool,
    frame_counter: u32,
    fg_color: u32,
    bg_color: u32,
    dim_color: u32,
}

impl Renderer {
    /// Load character generator ROM and auto-detect 2KB (8-row) vs 4KB (16-row).
    pub fn new(chargen_path: &str, phosphor: PhosphorColors) -> Renderer {
        let chargen = fs::read(chargen_path)
            .unwrap_or_else(|e| {
                eprintln!("Error: Failed to load character ROM '{}': {}", chargen_path, e);
                std::process::exit(1);
            });

        let is_2k = chargen.len() <= 2048;
        let scanlines_per_char = if is_2k { 8 } else { 16 };

        // 2KB ROM (81-146a): character data at offset 0x400, inverted polarity.
        // A10=1 during character display; A9-A3=char code; A2-A0=scan row.
        // 4KB ROM (81-235, 81-187): data at offset 0, normal polarity.
        let chargen_base = if is_2k { 0x400 } else { 0 };
        let inverted_polarity = is_2k;

        // 80 columns × 8 pixels = 640 wide
        // For 16-row ROM: 25 rows × 16 = 400 tall
        // For 8-row ROM: 24 rows × 8 = 192 tall
        let width = 640;
        let height = if scanlines_per_char == 16 { 400 } else { 192 };

        // 8-row ROMs: double each scanline for CRT-like 4:3 aspect ratio
        let scanline_double = scanlines_per_char == 8;
        let display_height = if scanline_double { height * 2 } else { height };

        Renderer {
            chargen,
            scanlines_per_char,
            chargen_base,
            inverted_polarity,
            framebuffer: vec![phosphor.bg; width * height],
            display_buffer: vec![phosphor.bg; width * display_height],
            width,
            height,
            scanline_double,
            frame_counter: 0,
            fg_color: phosphor.fg,
            bg_color: phosphor.bg,
            dim_color: phosphor.dim,
        }
    }

    /// Advance frame counter for blink timing.
    pub fn tick_frame(&mut self) {
        self.frame_counter = self.frame_counter.wrapping_add(1);
    }

    /// Render full screen from machine state, return pixel buffer.
    pub fn render(&mut self, machine: &KayproMachine) -> &[u32] {
        let blink_on = (self.frame_counter % BLINK_PERIOD) < (BLINK_PERIOD / 2);

        // Fixed display size matching window dimensions
        let display_rows = self.height / self.scanlines_per_char;

        // Get cursor info for CRTC mode
        let (cursor_addr, cursor_visible) = if machine.video_mode == VideoMode::Sy6545Crtc {
            let addr = machine.crtc.cursor_addr() & 0x7FF;
            let mode = machine.crtc.cursor_mode();
            let visible = match mode {
                0 => true,                 // steady
                1 => false,                // invisible
                2 | 3 => blink_on,         // blink
                _ => false,
            };
            (addr, visible)
        } else {
            (0xFFFF, false)
        };

        for row in 0..display_rows {
            for col in 0..80 {
                let (code, attr, is_cursor) = if machine.video_mode == VideoMode::Sy6545Crtc {
                    let start = machine.crtc.start_addr();
                    let addr = (start + row * 80 + col) & 0x7FF;
                    let at_cursor = cursor_visible && addr == cursor_addr;
                    (machine.crtc.get_vram(addr), machine.crtc.get_attr(addr), at_cursor)
                } else {
                    // Memory-mapped mode: 128-byte stride
                    (machine.vram[row * 128 + col], 0u8, false)
                };

                // Attribute bits
                let reverse = (attr & 0x01) != 0 || is_cursor;
                let dim = (attr & 0x02) != 0;
                let blink = if machine.video_mode == VideoMode::Sy6545Crtc {
                    (attr & 0x04) != 0
                } else {
                    (code & 0x80) != 0
                };
                let underline = (attr & 0x08) != 0;

                // If blinking and currently in "off" phase, render as blank
                let blank_cell = blink && !blink_on;

                // Look up character in ROM
                let char_index = if self.scanlines_per_char == 16 {
                    // 4KB ROM: codes 0-127 in lower 2KB, 128-255 in upper 2KB
                    code as usize
                } else {
                    // 2KB ROM: mask to 7 bits (bit 7 is blink in memory-mapped mode)
                    (code & 0x7F) as usize
                };

                let rom_offset = self.chargen_base + char_index * self.scanlines_per_char;

                // Colors for this cell
                let (on_color, off_color) = if reverse {
                    if dim { (self.bg_color, self.dim_color) } else { (self.bg_color, self.fg_color) }
                } else if dim {
                    (self.dim_color, self.bg_color)
                } else {
                    (self.fg_color, self.bg_color)
                };

                for scanline in 0..self.scanlines_per_char {
                    let mut rom_byte = if blank_cell {
                        0x00
                    } else if rom_offset + scanline < self.chargen.len() {
                        self.chargen[rom_offset + scanline]
                    } else {
                        0x00
                    };

                    // 2KB ROM uses inverted polarity (0=lit, 1=dark)
                    if self.inverted_polarity && !blank_cell {
                        rom_byte ^= 0xFF;
                    }

                    // Underline: force last scanline all-on
                    let pixels = if underline && scanline == self.scanlines_per_char - 1 && !blank_cell {
                        0xFF
                    } else {
                        rom_byte
                    };

                    let fb_y = row * self.scanlines_per_char + scanline;
                    let fb_x = col * 8;
                    let fb_offset = fb_y * self.width + fb_x;

                    // Bit 7 = leftmost pixel, bit 0 = rightmost
                    for pixel_col in 0..8 {
                        let bit = (pixels >> (7 - pixel_col)) & 1;
                        self.framebuffer[fb_offset + pixel_col] = if bit != 0 {
                            on_color
                        } else {
                            off_color
                        };
                    }
                }
            }
        }

        &self.framebuffer
    }

    /// Apply scanline doubling to the already-rendered framebuffer and return
    /// the display buffer. Call after `render()` and any overlay rendering.
    pub fn render_to_display_buffer_only(&mut self) -> &[u32] {
        if self.scanline_double {
            for y in 0..self.height {
                let src_start = y * self.width;
                let dst_row0 = y * 2 * self.width;
                let dst_row1 = dst_row0 + self.width;
                self.display_buffer[dst_row0..dst_row0 + self.width]
                    .copy_from_slice(&self.framebuffer[src_start..src_start + self.width]);
                self.display_buffer[dst_row1..dst_row1 + self.width]
                    .copy_from_slice(&self.framebuffer[src_start..src_start + self.width]);
            }
            &self.display_buffer
        } else {
            &self.framebuffer
        }
    }

    /// Return the display dimensions (after scanline doubling).
    pub fn display_size(&self) -> (usize, usize) {
        if self.scanline_double {
            (self.width, self.height * 2)
        } else {
            (self.width, self.height)
        }
    }

    /// Render a text overlay box onto the framebuffer.
    /// `lines` is a slice of strings to display. The box is centered horizontally,
    /// positioned starting at `start_row` (in character rows).
    pub fn render_overlay(&mut self, lines: &[&str], start_row: usize) {
        let box_width = lines.iter().map(|l| l.len()).max().unwrap_or(0) + 4; // 2 border + 2 padding
        let box_left = if box_width < 80 { (80 - box_width) / 2 } else { 0 };

        let border_fg = 0x0066FF66u32; // bright green
        let border_bg = 0x00001100u32; // very dark green
        let text_fg = self.fg_color;
        let text_bg = border_bg;

        // Top border
        let mut top = String::with_capacity(box_width);
        top.push('+');
        for _ in 0..box_width - 2 { top.push('-'); }
        top.push('+');
        self.render_text_line(&top, start_row, box_left, border_fg, border_bg);

        // Content lines
        for (i, line) in lines.iter().enumerate() {
            let mut row_text = String::with_capacity(box_width);
            row_text.push('|');
            row_text.push(' ');
            row_text.push_str(line);
            while row_text.len() < box_width - 1 { row_text.push(' '); }
            row_text.push('|');
            self.render_text_line(&row_text, start_row + 1 + i, box_left, text_fg, text_bg);
            // Redraw border chars with border colors
            self.render_char_at('|', start_row + 1 + i, box_left, border_fg, border_bg);
            self.render_char_at('|', start_row + 1 + i, box_left + box_width - 1, border_fg, border_bg);
        }

        // Bottom border
        let mut bottom = String::with_capacity(box_width);
        bottom.push('+');
        for _ in 0..box_width - 2 { bottom.push('-'); }
        bottom.push('+');
        self.render_text_line(&bottom, start_row + 1 + lines.len(), box_left, border_fg, border_bg);
    }

    /// Render a single line of text at a given character row and column.
    fn render_text_line(&mut self, text: &str, row: usize, col: usize, fg: u32, bg: u32) {
        for (i, ch) in text.chars().enumerate() {
            self.render_char_at(ch, row, col + i, fg, bg);
        }
    }

    /// Render a single character at the given character row/col position in the framebuffer.
    fn render_char_at(&mut self, ch: char, row: usize, col: usize, fg: u32, bg: u32) {
        if col >= 80 { return; }
        let display_rows = self.height / self.scanlines_per_char;
        if row >= display_rows { return; }

        let code = if (ch as u32) < 128 { ch as u8 } else { b'?' };
        let char_index = if self.scanlines_per_char == 16 {
            code as usize
        } else {
            (code & 0x7F) as usize
        };
        let rom_offset = self.chargen_base + char_index * self.scanlines_per_char;

        for scanline in 0..self.scanlines_per_char {
            let mut rom_byte = if rom_offset + scanline < self.chargen.len() {
                self.chargen[rom_offset + scanline]
            } else {
                0x00
            };
            if self.inverted_polarity {
                rom_byte ^= 0xFF;
            }

            let fb_y = row * self.scanlines_per_char + scanline;
            let fb_x = col * 8;
            let fb_offset = fb_y * self.width + fb_x;
            if fb_offset + 8 > self.framebuffer.len() { return; }

            for pixel_col in 0..8 {
                let bit = (rom_byte >> (7 - pixel_col)) & 1;
                self.framebuffer[fb_offset + pixel_col] = if bit != 0 { fg } else { bg };
            }
        }
    }
}
