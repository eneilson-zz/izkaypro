/// SY6545 CRT Controller emulation for Kaypro 2X/4/84
/// 
/// The SY6545 uses "transparent" addressing where video RAM is accessed
/// through registers R18 (address high) and R19 (address low).
/// 
/// On Kaypro 2X/4/84:
/// - Port 0x1C: CRTC register select (0-17 for timing, 18-19 for VRAM address)
/// - Port 0x1D: CRTC register data (R18/R19 set address latch, auto-increments between them)
/// - Port 0x1F: Control register (0x20 = strobe on, 0x00 = strobe off) AND character data
///
/// VRAM Access Protocol (from 81-292a ROM trace):
/// 1. Select R18 via port 0x1C (value 0x12)
/// 2. Write addr_hi, addr_lo to port 0x1D (auto-increment R18→R19→R18)
/// 3. Write character to port 0x1F (writes to VRAM at addr_latch)

pub struct Sy6545 {
    // CRTC registers R0-R17 (timing) + R18-R19 (update address)
    regs: [u8; 20],
    reg_index: u8,
    
    // Video RAM (16KB - SY6545 has 14-bit MA address bus)
    pub vram: [u8; 16384],
    pub vram_dirty: bool,
    
    // Transparent addressing state
    strobe: bool,            // true = address mode (hardware), false = data mode (CPU)
    addr_latch: u16,         // VRAM address for port 0x1F writes (set when strobe OFF)
    addr_hardware: u16,      // Hardware address (set when strobe ON)
    
    pub trace: bool,
}

impl Sy6545 {
    pub fn new() -> Sy6545 {
        Sy6545 {
            regs: [0; 20],
            reg_index: 0,
            vram: [0x20; 16384], // Initialize with spaces
            vram_dirty: true,
            strobe: false,
            addr_latch: 0,
            addr_hardware: 0,
            trace: false,
        }
    }
    
    /// Get display start address from R12:R13
    pub fn start_addr(&self) -> usize {
        ((self.regs[12] as usize) << 8) | (self.regs[13] as usize)
    }
    
    /// Port 0x1C write - Register select
    pub fn write_port_1c(&mut self, value: u8) {
        self.reg_index = value & 0x1f;
    }
    
    /// Port 0x1C read - Status
    pub fn read_port_1c(&self) -> u8 {
        // Return ready status with bit 7 set to prevent hangs
        0x80
    }
    
    /// Port 0x1D write - Data register
    /// Behavior depends on which register is selected and strobe state
    pub fn write_port_1d(&mut self, value: u8) {
        match self.reg_index {
            0..=17 => {
                // Standard CRTC timing registers (R0-R17)
                self.regs[self.reg_index as usize] = value;
                if self.trace {
                    // Log all timing registers for debugging
                    match self.reg_index {
                        0 => println!("CRTC: R0 (H Total) = 0x{:02x} ({})", value, value),
                        1 => println!("CRTC: R1 (H Displayed) = 0x{:02x} ({})", value, value),
                        6 => println!("CRTC: R6 (V Displayed) = 0x{:02x} ({})", value, value),
                        12 | 13 => println!("CRTC: R{} = 0x{:02x} (start_addr = 0x{:04x})", 
                            self.reg_index, value,
                            ((self.regs[12] as u16) << 8) | (self.regs[13] as u16)),
                        _ => {}
                    }
                }
            }
            18 => {
                // R18 - Update Address High / Data
                if self.strobe {
                    // Strobe ON (hardware mode): set hardware address high byte
                    self.addr_hardware = (self.addr_hardware & 0x00FF) | ((value as u16) << 8);
                    if self.trace {
                        println!("CRTC: R18 hw_addr_hi = 0x{:02x} (addr_hardware = 0x{:04x})", 
                            value, self.addr_hardware);
                    }
                } else {
                    // Strobe OFF (CPU mode): set CPU address high byte for port 0x1F writes
                    self.addr_latch = (self.addr_latch & 0x00FF) | ((value as u16) << 8);
                    if self.trace {
                        println!("CRTC: R18 addr_hi = 0x{:02x} (addr_latch = 0x{:04x})", 
                            value, self.addr_latch);
                    }
                }
                // SY6545 auto-increments reg_index from R18 to R19 for transparent addressing
                self.reg_index = 19;
            }
            19 => {
                // R19 - Update Address Low / Commit
                if self.strobe {
                    // Strobe ON (hardware mode): set hardware address low byte
                    self.addr_hardware = (self.addr_hardware & 0xFF00) | (value as u16);
                    if self.trace {
                        println!("CRTC: R19 hw_addr_lo = 0x{:02x} (addr_hardware = 0x{:04x})", 
                            value, self.addr_hardware);
                    }
                } else {
                    // Strobe OFF (CPU mode): set CPU address low byte for port 0x1F writes
                    self.addr_latch = (self.addr_latch & 0xFF00) | (value as u16);
                    if self.trace {
                        println!("CRTC: R19 addr_lo = 0x{:02x} (addr_latch = 0x{:04x})", 
                            value, self.addr_latch);
                    }
                }
                // SY6545 auto-increments reg_index back from R19 to R18 for transparent addressing
                self.reg_index = 18;
            }
            _ => {
                // Unknown register - ignore
                if self.trace {
                    println!("CRTC: Unknown R{} = 0x{:02x}", self.reg_index, value);
                }
            }
        }
    }
    
    /// Port 0x1D read
    pub fn read_port_1d(&self) -> u8 {
        if (self.reg_index as usize) < self.regs.len() {
            self.regs[self.reg_index as usize]
        } else {
            0
        }
    }
    
    /// Port 0x1E write - Direct VRAM access (if supported)
    pub fn write_port_1e(&mut self, value: u8) {
        let addr = (self.addr_latch as usize) & 0x3fff;
        self.vram[addr] = value;
        self.vram_dirty = true;
        self.addr_latch = self.addr_latch.wrapping_add(1);
    }
    
    /// Port 0x1E read
    pub fn read_port_1e(&self) -> u8 {
        let addr = (self.addr_latch as usize) & 0x3fff;
        self.vram[addr]
    }
    
    /// Port 0x1F write - Control / Character Data
    /// The 81-292a ROM uses port 0x1F for both control AND character output:
    /// - Value 0x00: Clear strobe (ready for next char)
    /// - Value 0x20: Set strobe AND write space to VRAM (dual purpose)
    /// - Any other value: Character to write to VRAM at addr_latch
    pub fn write_port_1f(&mut self, value: u8) {
        // Value 0x00: just clear strobe (no write)
        if value == 0x00 {
            self.strobe = false;
            if self.trace {
                println!("CRTC: Control = 0x00 (strobe=false)");
            }
            return;
        }
        
        // Value 0x20: set strobe AND write space to VRAM
        // This allows the ROM to clear screen by repeatedly writing 0x20
        if value == 0x20 {
            // Write space to current addr_latch (before setting strobe)
            let masked_addr = (self.addr_latch as usize) & 0x3fff;
            self.vram[masked_addr] = 0x20; // space
            self.vram_dirty = true;
            
            if self.trace {
                println!("CRTC: VRAM[0x{:04x}] = 0x20 ' ' + strobe=true", masked_addr);
            }
            
            // Auto-increment addr_latch after write (same as other characters)
            self.addr_latch = self.addr_latch.wrapping_add(1);
            
            self.strobe = true;
            return;
        }
        
        // Any other value is a character write to VRAM at current addr_latch
        let masked_addr = (self.addr_latch as usize) & 0x3fff;
        self.vram[masked_addr] = value;
        self.vram_dirty = true;
        
        if self.trace {
            println!("CRTC: VRAM[0x{:04x}] = 0x{:02x} '{}'", 
                masked_addr, value, 
                if value >= 0x20 && value < 0x7f { value as char } else { '.' });
        }
        
        // Auto-increment addr_latch after character write (SY6545 transparent addressing)
        self.addr_latch = self.addr_latch.wrapping_add(1);
    }
    
    /// Port 0x1F read
    pub fn read_port_1f(&self) -> u8 {
        if self.strobe { 0x20 } else { 0x00 }
    }
    
    /// Get VRAM byte at offset (for rendering)
    pub fn get_vram(&self, offset: usize) -> u8 {
        self.vram[offset & 0x3fff]
    }
    
}
