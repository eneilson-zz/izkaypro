/// SY6545 CRT Controller emulation for Kaypro 2X/4/84
/// 
/// The SY6545 uses "transparent" addressing where video RAM is accessed
/// through registers R18 (address high) and R19 (address low).
/// 
/// I/O Ports (from Kaypro 4/84 theory-of-operation):
/// - Port 0x1C (VIDCTL): Register select (write) / Status (read)
/// - Port 0x1D (VIDDAT): Register data read/write
/// - Port 0x1F (VIDMEM): Video RAM data read/write
///
/// Status Register (read from 0x1C):
/// - Bit 7 (SR7): UR - Update Ready (1 = ready for next update)
/// - Bit 6 (SR6): LRF - Light Pen Register Full (not used on Kaypro)
/// - Bit 5 (SR5): VRT - Vertical Retrace (1 = in vertical retrace)
/// - Bits 4-0: Unused
///
/// R31 (Dummy/Strobe Register):
/// - Does not store data
/// - Accessing via port 0x1D triggers update strobe
/// - Sets UR=0 (busy), then UR=1 (ready) after strobe completes
/// - Increments update address (R18:R19) after access

pub struct Sy6545 {
    // CRTC registers R0-R19 (timing + update address)
    // R31 is handled specially (dummy register)
    regs: [u8; 20],
    reg_index: u8,
    
    // Video RAM - Kaypro uses 2x 6116 SRAMs (2KB each):
    // - Character RAM: 0x000-0x7FF (2KB)
    // - Attribute RAM: 0x800-0xFFF (2KB)
    // We allocate 4KB to hold both planes
    pub vram: [u8; 4096],
    pub vram_dirty: bool,
    
    // Transparent addressing state
    addr_latch: u16,         // Update address for VIDMEM access (R18:R19)
    
    // Status register bits
    update_ready: bool,      // SR7: UR - Update Ready
    vertical_retrace: bool,  // SR5: VRT - Vertical Retrace
    
    // Frame counter for VRT timing simulation
    cycle_counter: u32,
    
    pub trace: bool,
}

impl Sy6545 {
    pub fn new() -> Sy6545 {
        // Initialize VRAM: character RAM (0x000-0x7FF) with spaces,
        // attribute RAM (0x800-0xFFF) with 0x00 (no attributes)
        let mut vram = [0u8; 4096];
        for i in 0..0x800 {
            vram[i] = 0x20; // Space character
        }
        // Attribute RAM (0x800-0xFFF) stays 0x00
        
        Sy6545 {
            regs: [0; 20],
            reg_index: 0,
            vram,
            vram_dirty: true,
            addr_latch: 0,
            update_ready: true,  // Start ready
            vertical_retrace: false,
            cycle_counter: 0,
            trace: false,
        }
    }
    
    /// Advance timing by one CPU cycle (for VRT simulation)
    /// Call this periodically to simulate vertical retrace timing
    #[allow(dead_code)]
    pub fn tick(&mut self) {
        self.cycle_counter = self.cycle_counter.wrapping_add(1);
        
        // Simulate vertical retrace at approximately 60Hz
        // Assuming ~2.5MHz CPU, that's ~41666 cycles per frame
        // VRT is active for roughly 1/10th of the frame (during retrace)
        let frame_cycles = 41666u32;
        let retrace_start = frame_cycles - 4000; // ~4000 cycles of retrace
        
        let frame_pos = self.cycle_counter % frame_cycles;
        self.vertical_retrace = frame_pos >= retrace_start;
    }
    
    /// Get display start address from R12:R13
    pub fn start_addr(&self) -> usize {
        ((self.regs[12] as usize) << 8) | (self.regs[13] as usize)
    }
    
    /// Port 0x1C write - Address Register (register select)
    /// Sets the 5-bit pointer to internal register 0-31
    /// 
    /// Selecting R31 (strobe register) triggers the update cycle:
    /// - Latches addr from R18:R19
    /// - Sets update_ready = false (busy)
    /// - After cycle: increments addr_latch, sets update_ready = true
    pub fn write_port_1c(&mut self, value: u8) {
        self.reg_index = value & 0x1f;
        
        if self.trace && self.reg_index >= 18 {
            println!("CRTC: Select R{}", self.reg_index);
        }
        
        // Selecting R31 triggers the strobe/update cycle
        // This is how the ROM protocol works (OUT 0x1C, 0x1F)
        if self.reg_index == 31 {
            self.update_ready = false;
            // In our simplified model, cycle completes immediately
            // Increment happens AFTER the data access via port 0x1F
            self.update_ready = true;
        }
    }
    
    /// Port 0x1C read - Status Register
    /// Returns status bits: SR7=UR, SR6=LRF, SR5=VRT
    pub fn read_port_1c(&self) -> u8 {
        let mut status = 0u8;
        
        // SR7: Update Ready (bit 7)
        if self.update_ready {
            status |= 0x80;
        }
        
        // SR6: Light Pen Register Full (bit 6) - not used, always 0
        
        // SR5: Vertical Retrace (bit 5)
        if self.vertical_retrace {
            status |= 0x20;
        }
        
        status
    }
    
    /// Port 0x1D write - Data Register
    /// Writes to the currently selected internal register
    pub fn write_port_1d(&mut self, value: u8) {
        match self.reg_index {
            0..=17 => {
                // Standard CRTC timing registers (R0-R17)
                self.regs[self.reg_index as usize] = value;
                // Cursor position/mode changes require screen refresh
                if self.reg_index >= 10 && self.reg_index <= 15 {
                    self.vram_dirty = true;
                }
                if self.trace {
                    match self.reg_index {
                        0 => println!("CRTC: R0 (H Total) = {} chars", value),
                        1 => println!("CRTC: R1 (H Displayed) = {} chars", value),
                        2 => println!("CRTC: R2 (H Sync Pos) = {}", value),
                        3 => println!("CRTC: R3 (Sync Widths) = 0x{:02x}", value),
                        4 => println!("CRTC: R4 (V Total) = {} rows", value),
                        5 => println!("CRTC: R5 (V Adjust) = {} lines", value),
                        6 => println!("CRTC: R6 (V Displayed) = {} rows", value),
                        7 => println!("CRTC: R7 (V Sync Pos) = {}", value),
                        8 => println!("CRTC: R8 (Mode Control) = 0x{:02x}", value),
                        9 => println!("CRTC: R9 (Scan Lines) = {} lines/row", value + 1),
                        10 => println!("CRTC: R10 (Cursor Start) = 0x{:02x}", value),
                        11 => println!("CRTC: R11 (Cursor End) = {}", value),
                        12 | 13 => println!("CRTC: R{} = 0x{:02x} (start_addr = 0x{:04x})", 
                            self.reg_index, value,
                            ((self.regs[12] as u16) << 8) | (self.regs[13] as u16)),
                        14 | 15 => println!("CRTC: R{} = 0x{:02x} (cursor_addr = 0x{:04x})", 
                            self.reg_index, value,
                            ((self.regs[14] as u16) << 8) | (self.regs[15] as u16)),
                        _ => {}
                    }
                }
            }
            18 => {
                // R18 - Update Address High
                self.regs[18] = value;
                self.addr_latch = (self.addr_latch & 0x00FF) | ((value as u16) << 8);
                if self.trace {
                    println!("CRTC: R18 = 0x{:02x} (addr_latch = 0x{:04x})", 
                        value, self.addr_latch);
                }
                // Auto-increment reg_index from R18 to R19
                self.reg_index = 19;
            }
            19 => {
                // R19 - Update Address Low
                self.regs[19] = value;
                self.addr_latch = (self.addr_latch & 0xFF00) | (value as u16);
                if self.trace {
                    println!("CRTC: R19 = 0x{:02x} (addr_latch = 0x{:04x})", 
                        value, self.addr_latch);
                }
                // Auto-increment reg_index from R19 to R18
                self.reg_index = 18;
            }
            31 => {
                // R31 - Dummy/Strobe Register
                // Writing to R31 triggers the update strobe:
                // 1. Sets UR=0 (busy)
                // 2. Increments update address
                // 3. Sets UR=1 (ready) - happens immediately in our simplified model
                self.update_ready = false;
                self.addr_latch = self.addr_latch.wrapping_add(1);
                self.update_ready = true; // Immediately ready (simplified)
                
                if self.trace {
                    println!("CRTC: R31 write strobe (addr_latch -> 0x{:04x})", self.addr_latch);
                }
            }
            _ => {
                // Registers 20-30 are not used
                if self.trace {
                    println!("CRTC: R{} = 0x{:02x} (unused)", self.reg_index, value);
                }
            }
        }
    }
    
    /// Port 0x1D read - Data Register
    /// Reads from the currently selected internal register
    pub fn read_port_1d(&mut self) -> u8 {
        match self.reg_index {
            0..=19 => {
                self.regs[self.reg_index as usize]
            }
            31 => {
                // R31 - Dummy/Strobe Register
                // Reading R31 also triggers update strobe and increments address
                self.update_ready = false;
                self.addr_latch = self.addr_latch.wrapping_add(1);
                self.update_ready = true; // Immediately ready (simplified)
                
                if self.trace {
                    println!("CRTC: R31 read strobe (addr_latch -> 0x{:04x})", self.addr_latch);
                }
                
                // Returns undefined data (dummy register)
                0x00
            }
            _ => {
                // Registers 20-30 return 0
                0x00
            }
        }
    }
    
    /// Port 0x1E write - Alternate VRAM access (if supported)
    pub fn write_port_1e(&mut self, value: u8) {
        let addr = (self.addr_latch as usize) & 0xFFF; // 4KB wrap (char + attr)
        self.vram[addr] = value;
        self.vram_dirty = true;
        self.addr_latch = self.addr_latch.wrapping_add(1);
    }
    
    /// Port 0x1E read
    pub fn read_port_1e(&self) -> u8 {
        let addr = (self.addr_latch as usize) & 0xFFF; // 4KB wrap
        self.vram[addr]
    }
    
    /// Port 0x1F write - VIDMEM (Video Memory Data)
    /// 
    /// Two protocols use this port:
    /// 
    /// 1. ROM protocol (81-292a): Uses 0x00 and 0x20 as control codes
    ///    - Value 0x00: Clear strobe (no VRAM write)
    ///    - Value 0x20: Write space to VRAM AND set strobe
    ///    - Other values: Write character to VRAM at addr_latch
    /// 
    /// 2. diag4 protocol: Sends strobe command (0x1F) to port 0x1C first
    ///    - All values are written as data (no control codes)
    ///    - Detected by checking if reg_index == 0x1F (R31 selected)
    pub fn write_port_1f(&mut self, value: u8) {
        // VRAM writes via port 0x1F should only occur after R31 (strobe) was selected.
        // If another register was selected, ignore the write to prevent stray VRAM corruption.
        if self.reg_index != 0x1F {
            if self.trace {
                println!("CRTC: VIDMEM write ignored (reg_index={}, not R31)", self.reg_index);
            }
            return;
        }
        
        // Write value to VRAM at current addr_latch
        let addr = (self.addr_latch as usize) & 0xFFF; // 4KB wrap (char + attr)
        self.vram[addr] = value;
        self.vram_dirty = true;
        
        if self.trace {
            println!("CRTC: VIDMEM[0x{:03x}] = 0x{:02x} '{}'", 
                addr, value, 
                if value >= 0x20 && value < 0x7f { value as char } else { '.' });
        }
        
        // Note: We do NOT auto-increment addr_latch here.
        // The ROM explicitly sets R18:R19 before each access.
        // Auto-increment only happens when accessing R31 via port 0x1D.
    }
    
    /// Port 0x1F read - VIDMEM (Video Memory Data)
    /// Returns byte at current addr_latch
    pub fn read_port_1f(&mut self) -> u8 {
        let addr = (self.addr_latch as usize) & 0xFFF; // 4KB wrap
        let value = self.vram[addr];
        
        if self.trace {
            println!("CRTC: VIDMEM[0x{:03x}] -> 0x{:02x} '{}'", 
                addr, value,
                if value >= 0x20 && value < 0x7f { value as char } else { '.' });
        }
        
        value
    }
    
    /// Get VRAM byte at offset (for rendering - character RAM only, 0x000-0x7FF)
    pub fn get_vram(&self, offset: usize) -> u8 {
        self.vram[offset & 0x7FF] // 2KB wrap for character display
    }
    
    /// Get attribute byte at offset (for rendering - attribute RAM, 0x800-0xFFF)
    pub fn get_attr(&self, offset: usize) -> u8 {
        self.vram[(offset & 0x7FF) + 0x800] // Attribute plane
    }
    
    /// Get cursor address from R14:R15
    pub fn cursor_addr(&self) -> usize {
        ((self.regs[14] as usize) << 8) | (self.regs[15] as usize)
    }
    
    /// Get cursor start line from R10 (bits 4-0)
    #[allow(dead_code)]
    pub fn cursor_start(&self) -> u8 {
        self.regs[10] & 0x1F
    }
    
    /// Get cursor end line from R11 (bits 4-0)
    #[allow(dead_code)]
    pub fn cursor_end(&self) -> u8 {
        self.regs[11] & 0x1F
    }
    
    /// Get cursor mode from R10 (bits 6-5)
    /// 0 = steady, 1 = invisible, 2 = blink 1/16, 3 = blink 1/32
    pub fn cursor_mode(&self) -> u8 {
        (self.regs[10] >> 5) & 0x03
    }
}
