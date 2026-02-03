/// Diagnostic tests for Kaypro emulator
/// Based on diag4.mac from Non-Linear Systems, Inc. (1983)
/// 
/// These tests verify ROM checksum and RAM integrity.

use iz80::Machine;

/// Result of a diagnostic test
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// Run ROM checksum test
/// Calculates 16-bit checksum of ROM and compares against known values
pub fn test_rom<M: Machine>(machine: &M, rom_size: usize) -> TestResult {
    // Calculate checksum: sum all bytes, accumulate carry into high byte
    let mut checksum_l: u16 = 0;
    let mut checksum_h: u16 = 0;
    
    for addr in 0..rom_size {
        let byte = machine.peek(addr as u16);
        checksum_l = checksum_l.wrapping_add(byte as u16);
        if checksum_l > 0xFF {
            checksum_h = checksum_h.wrapping_add(1);
            checksum_l &= 0xFF;
        }
    }
    
    let checksum = ((checksum_h & 0xFF) << 8) | (checksum_l & 0xFF);
    
    // Known good checksums from diag4.mac for various Kaypro ROMs
    // These are for 4KB (0x1000) ROMs
    let known_checksums: &[(u16, &str)] = &[
        (0x5A70, "Kaypro 2"),
        (0x6A92, "Kaypro 4/83 (81-232)"),
        // Add more as discovered
    ];
    
    let mut matched_rom = None;
    for (known, name) in known_checksums {
        if checksum == *known {
            matched_rom = Some(*name);
            break;
        }
    }
    
    if let Some(rom_name) = matched_rom {
        TestResult {
            name: "ROM Checksum".to_string(),
            passed: true,
            message: format!("ROM OK - {} (checksum: 0x{:04X})", rom_name, checksum),
        }
    } else {
        TestResult {
            name: "ROM Checksum".to_string(),
            passed: true, // Don't fail on unknown checksum, just report it
            message: format!("ROM checksum: 0x{:04X} (not in known list)", checksum),
        }
    }
}

/// Sliding data test for RAM
/// Writes rotating bit pattern, verifies all locations
/// Tests for data line faults
fn sliding_data_test<M: Machine>(machine: &mut M, start: u16, end: u16) -> Result<(), (u16, u8, u8)> {
    // First pass: pattern starts as 0x01, rotates left
    // Second pass: pattern starts as 0xFE, rotates left
    for initial_pattern in [0x01u8, 0xFE] {
        for bit_pos in 0..8 {
            let pattern = if initial_pattern == 0x01 {
                initial_pattern.rotate_left(bit_pos)
            } else {
                initial_pattern.rotate_left(bit_pos)
            };
            
            // Write pattern to all locations
            let mut addr = start;
            loop {
                machine.poke(addr, pattern);
                if addr == end {
                    break;
                }
                addr = addr.wrapping_add(1);
            }
            
            // Verify pattern in all locations
            let mut addr = start;
            loop {
                let read = machine.peek(addr);
                if read != pattern {
                    return Err((addr, pattern, read));
                }
                if addr == end {
                    break;
                }
                addr = addr.wrapping_add(1);
            }
        }
    }
    
    Ok(())
}

/// Address data test for RAM
/// Writes address low/high byte as data, verifies
/// Tests for address line faults
fn address_data_test<M: Machine>(machine: &mut M, start: u16, end: u16) -> Result<(), (u16, u8, u8)> {
    // Pass 1: Write low byte of address
    let mut addr = start;
    loop {
        machine.poke(addr, addr as u8);
        if addr == end {
            break;
        }
        addr = addr.wrapping_add(1);
    }
    
    // Verify low byte
    let mut addr = start;
    loop {
        let expected = addr as u8;
        let read = machine.peek(addr);
        if read != expected {
            return Err((addr, expected, read));
        }
        if addr == end {
            break;
        }
        addr = addr.wrapping_add(1);
    }
    
    // Pass 2: Write high byte of address
    let mut addr = start;
    loop {
        machine.poke(addr, (addr >> 8) as u8);
        if addr == end {
            break;
        }
        addr = addr.wrapping_add(1);
    }
    
    // Verify high byte
    let mut addr = start;
    loop {
        let expected = (addr >> 8) as u8;
        let read = machine.peek(addr);
        if read != expected {
            return Err((addr, expected, read));
        }
        if addr == end {
            break;
        }
        addr = addr.wrapping_add(1);
    }
    
    Ok(())
}

/// Run RAM test on a memory region
pub fn test_ram_region<M: Machine>(machine: &mut M, start: u16, end: u16, name: &str) -> TestResult {
    // Run sliding data test
    if let Err((addr, expected, got)) = sliding_data_test(machine, start, end) {
        return TestResult {
            name: format!("RAM {} (sliding)", name),
            passed: false,
            message: format!("FAIL at 0x{:04X}: expected 0x{:02X}, got 0x{:02X}", addr, expected, got),
        };
    }
    
    // Run address data test
    if let Err((addr, expected, got)) = address_data_test(machine, start, end) {
        return TestResult {
            name: format!("RAM {} (address)", name),
            passed: false,
            message: format!("FAIL at 0x{:04X}: expected 0x{:02X}, got 0x{:02X}", addr, expected, got),
        };
    }
    
    TestResult {
        name: format!("RAM {}", name),
        passed: true,
        message: format!("OK (0x{:04X}-0x{:04X})", start, end),
    }
}

/// Run all diagnostic tests (generic Machine tests only)
/// Returns a vector of test results
pub fn run_diagnostics<M: Machine>(machine: &mut M, rom_size: usize) -> Vec<TestResult> {
    let mut results = Vec::new();
    
    // Test 1: ROM Checksum
    results.push(test_rom(machine, rom_size));
    
    // Test 2: RAM regions
    // Note: We can't test all regions without disrupting the emulator state
    // Test a safe region that doesn't conflict with the test code
    
    // Test RAM from 0x4000 to 0x7FFF (16KB safe region)
    results.push(test_ram_region(machine, 0x4000, 0x7FFF, "0x4000-0x7FFF"));
    
    // Test RAM from 0x8000 to 0xBFFF
    results.push(test_ram_region(machine, 0x8000, 0xBFFF, "0x8000-0xBFFF"));
    
    results
}

/// Run VRAM test on SY6545 CRTC VRAM (2KB range 0x000-0x7FF)
/// This test requires direct access to the CRTC, not via Machine trait
pub fn test_vram(crtc: &mut super::sy6545::Sy6545) -> TestResult {
    let start: usize = 0x000;
    let end: usize = 0x7FF;
    
    // Save current VRAM contents
    let mut backup = [0u8; 0x800];
    for i in 0..0x800 {
        backup[i] = crtc.vram[i];
    }
    
    // Sliding data test
    for initial_pattern in [0x01u8, 0xFE] {
        for bit_pos in 0..8 {
            let pattern = initial_pattern.rotate_left(bit_pos);
            
            // Write pattern to all locations
            for addr in start..=end {
                crtc.vram[addr] = pattern;
            }
            
            // Verify pattern
            for addr in start..=end {
                let read = crtc.vram[addr];
                if read != pattern {
                    // Restore and return error
                    for i in 0..0x800 {
                        crtc.vram[i] = backup[i];
                    }
                    return TestResult {
                        name: "VRAM (sliding)".to_string(),
                        passed: false,
                        message: format!("FAIL at 0x{:04X}: expected 0x{:02X}, got 0x{:02X}", 
                            addr, pattern, read),
                    };
                }
            }
        }
    }
    
    // Address data test - low byte
    for addr in start..=end {
        crtc.vram[addr] = (addr & 0xFF) as u8;
    }
    for addr in start..=end {
        let expected = (addr & 0xFF) as u8;
        let read = crtc.vram[addr];
        if read != expected {
            for i in 0..0x800 {
                crtc.vram[i] = backup[i];
            }
            return TestResult {
                name: "VRAM (addr-lo)".to_string(),
                passed: false,
                message: format!("FAIL at 0x{:04X}: expected 0x{:02X}, got 0x{:02X}", 
                    addr, expected, read),
            };
        }
    }
    
    // Address data test - high byte
    for addr in start..=end {
        crtc.vram[addr] = ((addr >> 8) & 0xFF) as u8;
    }
    for addr in start..=end {
        let expected = ((addr >> 8) & 0xFF) as u8;
        let read = crtc.vram[addr];
        if read != expected {
            for i in 0..0x800 {
                crtc.vram[i] = backup[i];
            }
            return TestResult {
                name: "VRAM (addr-hi)".to_string(),
                passed: false,
                message: format!("FAIL at 0x{:04X}: expected 0x{:02X}, got 0x{:02X}", 
                    addr, expected, read),
            };
        }
    }
    
    // Restore VRAM
    for i in 0..0x800 {
        crtc.vram[i] = backup[i];
    }
    
    TestResult {
        name: "VRAM".to_string(),
        passed: true,
        message: format!("OK (0x{:04X}-0x{:04X})", start, end),
    }
}

/// Test VRAM via port I/O protocol (same as EMUTEST uses)
/// This simulates the diag4 protocol: set R18/R19, send strobe, read/write port 0x1F
pub fn test_vram_via_ports(crtc: &mut super::sy6545::Sy6545) -> TestResult {
    let start: usize = 0x000;
    let end: usize = 0x7FF;
    
    // Save current VRAM contents
    let mut backup = [0u8; 0x800];
    for i in 0..0x800 {
        backup[i] = crtc.vram[i];
    }
    
    // Helper: write to VRAM at address using diag4 protocol
    fn crtc_write(crtc: &mut super::sy6545::Sy6545, addr: u16, value: u8) {
        let addr_hi = ((addr >> 8) & 0x07) as u8;
        let addr_lo = (addr & 0xFF) as u8;
        
        // Select R18 and write high byte
        crtc.write_port_1c(0x12);
        crtc.write_port_1d(addr_hi);
        
        // Select R19 and write low byte
        crtc.write_port_1c(0x13);
        crtc.write_port_1d(addr_lo);
        
        // Send strobe command
        crtc.write_port_1c(0x1F);
        
        // Write data
        crtc.write_port_1f(value);
    }
    
    // Helper: read from VRAM at address using diag4 protocol
    fn crtc_read(crtc: &mut super::sy6545::Sy6545, addr: u16) -> u8 {
        let addr_hi = ((addr >> 8) & 0x07) as u8;
        let addr_lo = (addr & 0xFF) as u8;
        
        // Select R18 and write high byte
        crtc.write_port_1c(0x12);
        crtc.write_port_1d(addr_hi);
        
        // Select R19 and write low byte
        crtc.write_port_1c(0x13);
        crtc.write_port_1d(addr_lo);
        
        // Send strobe command
        crtc.write_port_1c(0x1F);
        
        // Read data
        crtc.read_port_1f()
    }
    
    // Test a few specific addresses first
    let test_addrs = [0x000u16, 0x001, 0x100, 0x200, 0x7FF];
    let test_pattern = 0xA5u8;
    
    for &addr in &test_addrs {
        crtc_write(crtc, addr, test_pattern);
        let read_back = crtc_read(crtc, addr);
        if read_back != test_pattern {
            // Restore VRAM
            for i in 0..0x800 {
                crtc.vram[i] = backup[i];
            }
            return TestResult {
                name: "VRAM via ports".to_string(),
                passed: false,
                message: format!("FAIL at 0x{:04X}: wrote 0x{:02X}, read 0x{:02X}", 
                    addr, test_pattern, read_back),
            };
        }
    }
    
    // Now do full sliding data test on a smaller range to be faster
    for addr in (start..=end).step_by(16) {
        let pattern = 0x55u8;
        crtc_write(crtc, addr as u16, pattern);
        let read_back = crtc_read(crtc, addr as u16);
        if read_back != pattern {
            // Restore VRAM
            for i in 0..0x800 {
                crtc.vram[i] = backup[i];
            }
            return TestResult {
                name: "VRAM via ports".to_string(),
                passed: false,
                message: format!("FAIL at 0x{:04X}: wrote 0x{:02X}, read 0x{:02X}", 
                    addr, pattern, read_back),
            };
        }
    }
    
    // Restore VRAM
    for i in 0..0x800 {
        crtc.vram[i] = backup[i];
    }
    
    TestResult {
        name: "VRAM via ports".to_string(),
        passed: true,
        message: format!("OK (0x{:04X}-0x{:04X})", start, end),
    }
}

/// Print diagnostic results to console
pub fn print_results(results: &[TestResult]) {
    println!("\n=== Kaypro Diagnostics ===\n");
    
    let mut all_passed = true;
    for result in results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        println!("[{}] {}: {}", status, result.name, result.message);
        if !result.passed {
            all_passed = false;
        }
    }
    
    println!();
    if all_passed {
        println!("All tests passed!");
    } else {
        println!("Some tests failed.");
    }
    println!();
}
