use std::fs::{File};
use std::io::{Write};
use iz80::Machine;
use super::FloppyController;
#[cfg(unix)]
use super::keyboard_unix::Keyboard;
#[cfg(windows)]
use super::keyboard_win::Keyboard;
use super::rtc::Rtc;
use super::sio::Sio;
use super::sy6545::Sy6545;

/* Memory map:

    0x0000-0xffff: 64Kb of RAM
    If bank1 is selected:
        0x0000-0x2fff: 12Kb of ROM
        0x3000-0x3fff: 4Kb of VRAM

*/

/// Video mode selection for backwards compatibility
#[derive(Copy, Clone, PartialEq)]
#[allow(dead_code)]
pub enum VideoMode {
    /// Memory-mapped VRAM at 0x3000-0x3FFF (Kaypro II, 4/83)
    MemoryMapped,
    /// SY6545 CRTC with transparent addressing (Kaypro 2X, 4/84)
    Sy6545Crtc,
}

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum SystemBit {
    DriveA = 0x01,
    DriveB = 0x02,
    Side2 = 0x04,
    CentronicsReady = 0x08,
    CentronicsStrobe = 0x10,
    SingleDensity = 0x20,
    MotorsOff = 0x40,
    Bank = 0x80,
}

const IO_PORT_NAMES: [&str; 40] = [
    /* 0x00 */"Baud rate A, serial",
    /* 0x01 */"-",
    /* 0x02 */"-",
    /* 0x03 */"-",
    /* 0x04 */"SIO A data register.",
    /* 0x05 */"SIO B data register, keyboard.",
    /* 0x06 */"SIO A control register.",
    /* 0x07 */"SIO B control register, keyboard.",
    /* 0x08 */"PIO 1 channel A data register.",
    /* 0x09 */"PIO 1 channel A control register.",
    /* 0x0a */"PIO 1 channel B data register.",
    /* 0x0b */"PIO 1 channel B control register.",
    /* 0x0c */"Baud rate B, keyboard.",
    /* 0x0d */"-",
    /* 0x0e */"-",
    /* 0x0f */"-",
    /* 0x10 */"Floppy controller, Command/status register.",
    /* 0x11 */"Floppy controller, Track register.",
    /* 0x12 */"Floppy controller, Sector register.",
    /* 0x13 */"Floppy controller, Data register.",
    /* 0x14 */"System PIO (Kaypro 4-84)",
    /* 0x15 */"-",
    /* 0x16 */"-",
    /* 0x17 */"-",
    /* 0x18 */"-",
    /* 0x19 */"-",
    /* 0x1a */"-",
    /* 0x1b */"-",
    /* 0x1c */"PIO 2 channel A data register.",
    /* 0x1d */"PIO 2 channel A control register.",
    /* 0x1e */"PIO 2 channel B data register.",
    /* 0x1f */"PIO 2 channel B control register.",
    /* 0x20 */"RTC PIO data (CLKADD).",
    /* 0x21 */"RTC PIO channel B data.",
    /* 0x22 */"RTC PIO control (CLKCTL).",
    /* 0x23 */"RTC PIO channel B control.",
    /* 0x24 */"RTC data (CLKDAT).",
    /* 0x25 */"-",
    /* 0x26 */"-",
    /* 0x27 */"-",
    ];


// Fallback ROM (used when external ROM file can't be loaded)
static FALLBACK_ROM: &[u8] = include_bytes!("../roms/81-292a.rom");

pub struct KayproMachine {
    rom: Vec<u8>,
    ram: [u8; 65536],
    pub vram: [u8; 4096],
    pub vram_dirty: bool,
    pub system_bits: u8,
    port14_raw: u8, // Raw value written to port 0x14 (for 81-292a ROM compatibility)
    
    // Video mode and CRTC for Kaypro 2X/4/84
    pub video_mode: VideoMode,
    pub crtc: Sy6545,

    // SIO state for interrupt-driven keyboard (KayPLUS)
    sio_b_wr_select: u8,   // Next WR register to write (set by WR0 pointer bits)
    pub sio_b_wr1: u8,     // WR1: interrupt enable/mode
    pub sio_b_wr2: u8,     // WR2: interrupt vector (channel B only)
    pub sio_int_pending: bool, // SIO interrupt waiting to be serviced

    pub trace_io: bool,
    trace_system_bits: bool,

    pub kayplus_clock_fixup: bool,

    pub keyboard: Keyboard,
    pub floppy_controller: FloppyController,
    pub sio: Sio,
    pub rtc: Rtc,
}

impl KayproMachine {
    pub fn new(
        rom_path: &str,
        video_mode: VideoMode,
        floppy_controller: FloppyController,
        trace_io: bool,
        trace_system_bits: bool,
        trace_crtc: bool,
        trace_sio: bool,
        trace_rtc: bool,
    ) -> KayproMachine {
        // Load ROM from file, fall back to embedded if not found
        let rom_data = Self::load_rom_or_fallback(rom_path);
        
        // Initialize RAM with ROM content (ROM shadowing)
        // This is needed for ROMs like Turbo ROM that switch to RAM bank
        // and expect to continue executing the same code from RAM.
        let mut ram = [0u8; 65536];
        for (i, &byte) in rom_data.iter().enumerate() {
            ram[i] = byte;
        }
        
        let mut crtc = Sy6545::new();
        crtc.trace = trace_crtc;
        
        KayproMachine {
            rom: rom_data,
            ram,
            vram: [0; 4096],
            vram_dirty: false,
            system_bits: SystemBit::Bank as u8 | SystemBit::MotorsOff as u8,
            port14_raw: 0xDF, // Initial value for 81-292a (ROM mode, drive A, motor on)
            video_mode,
            crtc,
            sio_b_wr_select: 0,
            sio_b_wr1: 0,
            sio_b_wr2: 0,
            sio_int_pending: false,
            trace_io,
            trace_system_bits,
            kayplus_clock_fixup: false,
            keyboard: Keyboard::new(),
            floppy_controller,
            sio: Sio::new(trace_sio),
            rtc: Rtc::new(trace_rtc),
        }
    }
    
    /// Load ROM from file, falling back to embedded ROM if not found
    fn load_rom_or_fallback(path: &str) -> Vec<u8> {
        match std::fs::read(path) {
            Ok(content) => content,
            Err(_) => {
                eprintln!("Warning: Could not load ROM '{}', using fallback", path);
                FALLBACK_ROM.to_vec()
            }
        }
    }
    
    pub fn is_rom_rank(&self) -> bool {
        self.system_bits & SystemBit::Bank as u8 != 0
    }

    /// Check if delivering an NMI right now is safe.
    /// The Z80 NMI always vectors to 0x0066. We check the actual byte(s)
    /// at that address (as currently mapped) to determine whether it is a
    /// proper NMI handler (RET / RETN) or unrelated code (e.g. KayPLUS
    /// checksum loop). Standard Kaypro ROMs all have RET (0xC9) at 0x0066.
    pub fn nmi_vector_is_safe(&self) -> bool {
        let b0 = self.peek(0x0066);
        if b0 == 0xC9 { // RET
            return true;
        }
        let b1 = self.peek(0x0067);
        if b0 == 0xED && b1 == 0x45 { // RETN
            return true;
        }
        if b0 == 0xC3 { // JP nn — safe if target is in RAM
            let target = self.peek(0x0067) as u16 | ((self.peek(0x0068) as u16) << 8);
            if !self.is_rom_rank() || (target as usize) >= self.rom.len() {
                return true;
            }
        }
        false
    }

    fn update_system_bits(&mut self, bits: u8) {
        self.system_bits = bits;
        if bits & SystemBit::DriveA as u8 != 0 {
            self.floppy_controller.set_drive(0);
        } else if bits & SystemBit::DriveB as u8 != 0 {
            self.floppy_controller.set_drive(1);
        }

        let motor_off = bits & SystemBit::MotorsOff as u8 != 0;
        self.floppy_controller.set_motor(!motor_off);

        let single_density = bits & SystemBit::SingleDensity as u8 != 0;
        self.floppy_controller.set_single_density(single_density);

        let side_2 = bits & SystemBit::Side2 as u8 != 0;
        self.floppy_controller.set_side(side_2);

        if self.trace_system_bits {
            print_system_bits(self.system_bits);
        }
    }

    // Kaypro 4-84 uses port 0x14 with different bit layout:
    // Bit 7: BANK (0=RAM, 1=ROM/Video)
    // Bit 6: CHSET (character set)
    // Bit 5: DENSITY (0=double, 1=single)
    // Bit 4: MOTOR (0=off, 1=on)
    // Bit 3: PSTROB (Centronics strobe)
    // Bit 2: SIDE (1=side 0, 0=side 1) - ACTIVE LOW
    // Bit 1-0: Drive select - DIRECTLY ACTIVE (active bits select drive)
    //   81-232 ROM: A=01, B=10
    //   81-292a ROM: A=11 (both bits active), B=00 or different encoding
    //   We detect which ROM based on whether it sets 0x03 for drive A
    fn update_system_bits_k484(&mut self, bits: u8) {
        // Convert port 0x14 format to internal system_bits format
        let mut sys_bits: u8 = 0;

        // Bank bit (same position, same polarity)
        if bits & 0x80 != 0 {
            sys_bits |= SystemBit::Bank as u8;
        }

        // Motor (port 0x14: bit 4, 1=on; system_bits: bit 6, 1=off)
        if bits & 0x10 == 0 {
            sys_bits |= SystemBit::MotorsOff as u8;
        }

        // Density (port 0x14: bit 5, 1=single; system_bits: bit 5, 1=single)
        if bits & 0x20 != 0 {
            sys_bits |= SystemBit::SingleDensity as u8;
        }

        // Side (port 0x14: bit 2, 1=side0, 0=side1; system_bits: bit 2, 1=side2)
        // INVERTED polarity!
        if bits & 0x04 == 0 {
            sys_bits |= SystemBit::Side2 as u8;
        }

        // Drive select (port 0x14: bits 1-0)
        // 81-232 uses: A=01, B=10
        // 81-292a uses: A=10 (bit 1), B=01 (bit 0), both=11, neither=00
        // The ROM writes 0xDF (11) for init, then 0xDE (10) for drive A
        let drive_sel = bits & 0x03;
        let drive: Option<u8> = match drive_sel {
            0x02 => Some(0), // 81-292a: A=10 (bit 1 set, bit 0 clear)
            0x01 => Some(1), // 81-292a: B=01 (bit 0 set, bit 1 clear)
            0x03 => Some(0), // Both bits = default to A (initialization)
            0x00 => None,    // No drive selected
            _ => None,
        };

        if let Some(d) = drive {
            if d == 0 {
                sys_bits |= SystemBit::DriveA as u8;
            } else {
                sys_bits |= SystemBit::DriveB as u8;
            }
        }

        // Centronics strobe (bit 3)
        if bits & 0x08 != 0 {
            sys_bits |= SystemBit::CentronicsStrobe as u8;
        }

        self.system_bits = sys_bits;
        self.port14_raw = bits; // Save raw value for reads

        // Apply settings to floppy controller
        if let Some(d) = drive {
            self.floppy_controller.set_drive(d);
        }

        let motor_on = bits & 0x10 != 0;
        self.floppy_controller.set_motor(motor_on);

        let single_density = bits & 0x20 != 0;
        self.floppy_controller.set_single_density(single_density);

        // Side select inverted: bit 2 = 1 means side 0, bit 2 = 0 means side 1
        let side_2 = bits & 0x04 == 0;
        self.floppy_controller.set_side(side_2);

        if self.trace_system_bits {
            print_system_bits(self.system_bits);
        }
    }

    fn get_system_bits_k484(&self) -> u8 {
        // Return the raw value that was written to port 0x14
        // This preserves bits like CharSet (bit 6) that we don't track internally
        self.port14_raw
    }

    fn sio_b_write_control(&mut self, value: u8) {
        let reg = self.sio_b_wr_select;
        match reg {
            0 => {
                // WR0: register pointer and command bits
                // Bits 2-0: register pointer for next write
                self.sio_b_wr_select = value & 0x07;
                // Bits 5-3: command (channel reset, etc.)
                let cmd = (value >> 3) & 0x07;
                if cmd == 3 {
                    // Channel reset
                    self.sio_b_wr1 = 0;
                    self.sio_b_wr_select = 0;
                }
            }
            1 => {
                // WR1: interrupt enable/mode
                // Bits 4-3: Rx interrupt mode
                //   00 = Rx INT disabled
                //   01 = Rx INT on first char
                //   10 = Rx INT on all chars (parity affects vector)
                //   11 = Rx INT on all chars (parity does NOT affect vector)
                self.sio_b_wr1 = value;
                self.sio_b_wr_select = 0;
                if self.trace_io {
                    let rx_mode = (value >> 3) & 0x03;
                    println!("SIO B: WR1=0x{:02X} (Rx INT mode={})", value, rx_mode);
                }
            }
            2 => {
                // WR2: interrupt vector (channel B only)
                self.sio_b_wr2 = value;
                self.sio_b_wr_select = 0;
                if self.trace_io {
                    println!("SIO B: WR2=0x{:02X} (interrupt vector)", value);
                }
            }
            _ => {
                // WR3-WR7: other config, not needed for interrupt emulation
                self.sio_b_wr_select = 0;
            }
        }
    }

    /// Check if the SIO should generate an interrupt for keyboard input.
    /// Returns the IM2 vector address if an interrupt should fire, or None.
    pub fn sio_check_interrupt(&mut self, i_reg: u8) -> Option<u16> {
        // Only fire if Rx interrupts are enabled (WR1 bits 4-3 != 00)
        let rx_int_mode = (self.sio_b_wr1 >> 3) & 0x03;
        if rx_int_mode == 0 {
            return None;
        }

        if self.sio_int_pending {
            return None;
        }

        if !self.keyboard.is_key_pressed() {
            return None;
        }

        self.sio_int_pending = true;

        // IM2 vector: I register << 8 | modified WR2
        // Channel B Rx Available: vector bits 3,2,1 = 010
        let vector_byte = (self.sio_b_wr2 & 0xF1) | 0x04;
        let vector_addr = (i_reg as u16) << 8 | vector_byte as u16;

        // Read the handler address from the vector table
        let handler_lo = self.ram[vector_addr as usize] as u16;
        let handler_hi = self.ram[vector_addr.wrapping_add(1) as usize] as u16;
        let handler = handler_hi << 8 | handler_lo;

        Some(handler)
    }

    /// Patch the KayPLUS software clock counters (0xFF5C-0xFF5E) with
    /// wall-clock time from the RTC. Called when PC reaches 0x069E
    /// (the start of the clock increment loop in the KayPLUS BIOS tick
    /// routine). The caller must then advance PC to 0x06CE to skip
    /// the increment loop, so the display code reads the patched values.
    pub fn patch_software_clock(&mut self) {
        let (sec, min, hour) = self.rtc.current_time_hms();
        self.ram[0xFF5E] = sec;
        self.ram[0xFF5D] = min;
        self.ram[0xFF5C] = hour;
    }

    pub fn save_bios(&self) -> Result<String, String> {
        let start = self.ram[1] as usize +
            ((self.ram[2] as usize) << 8) - 3;
        let end = 0xfc00;
        
        if start >= end {
            return Err(format!("Invalid BIOS range: 0x{:04X}-0x{:04X}", start, end));
        }

        let filename = format!("bios_{:x}.bin", start);
        let mut file = File::create(&filename)
            .map_err(|e| format!("Failed to create {}: {}", filename, e))?;
        file.write_all(&self.ram[start..end])
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
        
        Ok(filename)
    }
}

impl Machine for KayproMachine {
    fn peek(&self, address: u16) -> u8 {
        if (address as usize) < self.rom.len() && self.is_rom_rank() {
            // ROM at 0x0000-ROM_SIZE when in ROM bank mode
            self.rom[address as usize]
        } else if address >= 0x3000 && address < 0x4000 && self.is_rom_rank() 
                  && self.video_mode == VideoMode::MemoryMapped {
            // Memory-mapped VRAM at 0x3000-0x3FFF only in ROM bank mode
            // and only for MemoryMapped video mode (Kaypro II, 4/83)
            // For SY6545 CRTC mode, VRAM is accessed via ports, not memory
            self.vram[address as usize - 0x3000]
        } else {
            // RAM (which contains ROM data shadowed at startup for addresses < ROM size)
            self.ram[address as usize]
        }
    }

    fn poke(&mut self, address: u16, value: u8) {
        if address < 0x3000 && self.is_rom_rank() {
            // Writes to ROM area go to RAM (for ROM shadowing)
            self.ram[address as usize] = value;
        } else if address >= 0x3000 && address < 0x4000 && self.is_rom_rank()
                  && self.video_mode == VideoMode::MemoryMapped {
            // Memory-mapped VRAM at 0x3000-0x3FFF only in ROM bank mode
            // and only for MemoryMapped video mode (Kaypro II, 4/83)
            // For SY6545 CRTC mode, VRAM is accessed via ports, not memory
            let offset = address as usize - 0x3000;
            self.vram[offset] = value;
            self.vram_dirty = true;
        } else {
            self.ram[address as usize] = value;
        }
    }

    fn port_out(&mut self, address: u16, value: u8) {

        let port = address as u8 & 0b_1011_1111; // A7 enables decoder, A6 unused, A5 selects U26/U27
        if port >= 0x80 {
            // Pin 7 is tied to enable of the 3-8 decoder
            if self.trace_io {
                println!("OUT(0x{:02x} 'Ignored', 0x{:02x})", port, value);
            }
            return
        }

        if self.trace_io && port != 0x1c && port != 0x14
            && (port as usize) < IO_PORT_NAMES.len() {
            println!("OUT(0x{:02x} '{}', 0x{:02x}): ", port, IO_PORT_NAMES[port as usize], value);
        }
        match port {
            // 8116 Baud Rate Generator — only accept from user programs (RAM mode).
            // TurboROM's BIOS writes to port 0x00 for internal purposes,
            // which would corrupt the serial baud rate set by QTerm.
            0x00 => {
                if !self.is_rom_rank() {
                    self.sio.set_baud_rate_code(value);
                }
            },
            // SIO-1 Channel A
            0x04 => self.sio.write_data(value),
            0x06 => self.sio.write_control(value),
            // SIO-1 Channel B (keyboard)
            0x07 => self.sio_b_write_control(value),
            // Floppy controller
            0x10 => self.floppy_controller.put_command(value),
            0x11 => self.floppy_controller.put_track(value),
            0x12 => self.floppy_controller.put_sector(value),
            0x13 => self.floppy_controller.put_data(value),
            // System bits - Kaypro 4-84 BIOS uses port 0x14
            // The 81-232 ROM writes 0x17 to port 0x14 during video init (this is the
            // only problematic write - it would unmap ROM mid-execution)
            0x14 => {
                // Special case: 0x17 during ROM init is a video setup, not banking
                // This value happens to have bit 7=0 but shouldn't unmap ROM
                if value == 0x17 && self.is_rom_rank() {
                    // Ignore this specific write - it's from ROM video init
                } else {
                    self.update_system_bits_k484(value);
                }
            },
            // Port 0x1C-0x1F: Different behavior based on video mode
            0x1c => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    // CRTC register select
                    self.crtc.write_port_1c(value);
                } else {
                    // Memory-mapped mode (Kaypro II, 4/83): system bits control
                    self.update_system_bits(value);
                }
            },
            0x1d => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    self.crtc.write_port_1d(value);
                }
            },
            0x1e => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    self.crtc.write_port_1e(value);
                }
            },
            0x1f => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    self.crtc.write_port_1f(value);
                }
            },
            // RTC PIO and clock (Kaypro 4-84 only)
            0x20 => self.rtc.write_addr(value),
            0x22 => self.rtc.write_control(value),
            0x24 => self.rtc.write_data(value),
            _ => {}
        } 
    }

    fn port_in(&mut self, address: u16) -> u8 {
        let port = address as u8 & 0b_1011_1111; // A7 enables decoder, A6 unused, A5 selects U26/U27
        if port >= 0x80 { // Pin 7 is tied to enable of the 3-8 decoder
            if self.trace_io {
                println!("IN(0x{:02x} 'Ignored')", port);
            }
            return 0x00
        }

        let value = match port {
            // SIO-1 Channel A data — only drain Rx FIFO from user programs.
            // TurboROM's BIOS continuously reads port 0x04, which would
            // steal bytes from QTerm's Rx stream.
            0x04 => {
                if self.is_rom_rank() { 0 } else { self.sio.read_data() }
            },
            0x06 => self.sio.read_control(),

            0x05 => {
                self.sio_int_pending = false;
                self.keyboard.get_key()
            },
            0x07 => {
                // SIO B RR0: bit 0 = Rx char available, bit 2 = Tx buffer empty
                let rx_ready = if self.keyboard.is_key_pressed() { 1 } else { 0 };
                rx_ready | 0x04
            },

            // Floppy controller
            0x10 => self.floppy_controller.get_status(),
            0x11 => self.floppy_controller.get_track(),
            0x12 => self.floppy_controller.get_sector(),
            0x13 => self.floppy_controller.get_data(),
            // System bits (Kaypro 4-84 uses port 0x14, Kaypro II uses port 0x1C)
            0x14 => self.get_system_bits_k484(),
            // Port 0x1C-0x1F: Different behavior based on video mode
            0x1c => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    self.crtc.read_port_1c()
                } else {
                    self.system_bits
                }
            },
            0x1d => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    self.crtc.read_port_1d()
                } else {
                    0xca
                }
            },
            0x1e => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    self.crtc.read_port_1e()
                } else {
                    0xca
                }
            },
            0x1f => {
                if self.video_mode == VideoMode::Sy6545Crtc {
                    self.crtc.read_port_1f()
                } else {
                    0xca
                }
            },
            // RTC PIO and clock (Kaypro 4-84 only)
            0x20 => self.rtc.read_addr(),
            0x24 => self.rtc.read_data(),
            _ => 0xca,
        }; 

        if self.trace_io && port != 0x13 && port != 0x07 && port != 0x1c && port != 0x14
            && (port as usize) < IO_PORT_NAMES.len() {
            println!("IN(0x{:02x} '{}') = 0x{:02x}", port, IO_PORT_NAMES[port as usize], value);
        }
        value
    }
}

fn print_system_bits(system_bits: u8) {
    print!("System bits: ");
    if system_bits & SystemBit::DriveA as u8 != 0           {print!("DriveA ");}
    if system_bits & SystemBit::DriveB as u8 != 0           {print!("DriveB ");}
    if system_bits & SystemBit::Side2 as u8 != 0            {print!("Side2 ");}
    if system_bits & SystemBit::CentronicsReady  as u8 != 0 {print!("CentronicsReady ");}
    if system_bits & SystemBit::CentronicsStrobe as u8 != 0 {print!("CentronicsStrobe ");}
    if system_bits & SystemBit::SingleDensity as u8 != 0    {print!("SingleDensity ");}
    if system_bits & SystemBit::MotorsOff as u8 != 0        {print!("MotorsOff ");}
    if system_bits & SystemBit::Bank as u8 != 0             {print!("ROM ");}
    println!();
}
