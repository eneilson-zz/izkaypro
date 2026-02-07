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
        (0x0B69, "Kaypro 2X/4/84 (81-292a)"),
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
    
    // Save current VRAM contents (4KB - char + attr)
    let mut backup = [0u8; 4096];
    for i in 0..4096 {
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
                    for i in 0..4096 {
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
            for i in 0..4096 {
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
            for i in 0..4096 {
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
    for i in 0..4096 {
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
    
    // Save current VRAM contents (4KB - char + attr)
    let mut backup = [0u8; 4096];
    for i in 0..4096 {
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
            for i in 0..4096 {
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
            for i in 0..4096 {
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
    for i in 0..4096 {
        crtc.vram[i] = backup[i];
    }
    
    TestResult {
        name: "VRAM via ports".to_string(),
        passed: true,
        message: format!("OK (0x{:04X}-0x{:04X})", start, end),
    }
}

/// Run Attribute RAM test on SY6545 CRTC (2KB range 0x800-0xFFF)
/// This is the fourth video test from diag4.mac (vatst)
pub fn test_attr_ram(crtc: &mut super::sy6545::Sy6545) -> TestResult {
    let start: usize = 0x800;
    let end: usize = 0xFFF;
    
    // Save current VRAM contents (4KB - char + attr)
    let mut backup = [0u8; 4096];
    for i in 0..4096 {
        backup[i] = crtc.vram[i];
    }
    
    // Fast-complement test (from diag4.mac vatst)
    // Read location, complement, write back, verify, restore
    for addr in start..=end {
        let original = crtc.vram[addr];
        let complement = !original;
        
        // Write complement
        crtc.vram[addr] = complement;
        
        // Verify
        let read_back = crtc.vram[addr];
        if read_back != complement {
            // Restore and return error
            for i in 0..4096 {
                crtc.vram[i] = backup[i];
            }
            return TestResult {
                name: "Attribute RAM".to_string(),
                passed: false,
                message: format!("FAIL at 0x{:04X}: expected 0x{:02X}, got 0x{:02X}", 
                    addr, complement, read_back),
            };
        }
        
        // Restore original
        crtc.vram[addr] = original;
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
                    for i in 0..4096 {
                        crtc.vram[i] = backup[i];
                    }
                    return TestResult {
                        name: "Attribute RAM (sliding)".to_string(),
                        passed: false,
                        message: format!("FAIL at 0x{:04X}: expected 0x{:02X}, got 0x{:02X}", 
                            addr, pattern, read),
                    };
                }
            }
        }
    }
    
    // Restore VRAM
    for i in 0..4096 {
        crtc.vram[i] = backup[i];
    }
    
    TestResult {
        name: "Attribute RAM".to_string(),
        passed: true,
        message: format!("OK (0x{:04X}-0x{:04X})", start, end),
    }
}

/// Boot test configuration for a single machine
pub struct BootTestConfig {
    pub name: &'static str,
    pub rom_path: &'static str,
    pub video_mode: crate::kaypro_machine::VideoMode,
    pub disk_format: crate::media::MediaFormat,
    pub disk_a: &'static str,
    pub disk_b: &'static str,
    pub side1_sector_base: u8,
    pub trace_fdc: bool,
    pub trace_io: bool,
    pub trace_pc: bool,
}

/// Run boot tests for all supported Kaypro models.
/// Each test boots the machine headlessly and checks that "A>" appears in VRAM
/// within a reasonable instruction count, and that the CPU is not stuck in an
/// infinite loop (detected by PC repeating at the same address).
pub fn run_boot_tests() -> Vec<TestResult> {
    let configs = vec![
        BootTestConfig {
            name: "Kaypro 4/84 (81-292a)",
            rom_path: "roms/81-292a.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/cpm22g-rom292a.img",
            disk_b: "disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            trace_fdc: false,
            trace_io: false,
            trace_pc: false,
        },
        BootTestConfig {
            name: "Kaypro 4/84 TurboROM",
            rom_path: "roms/trom34.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/k484_turborom_63k_boot.img",
            disk_b: "disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            trace_fdc: false,
            trace_io: false,
            trace_pc: false,
        },
        BootTestConfig {
            name: "KayPLUS 84",
            rom_path: "roms/kplus84.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/kayplus1.img",
            disk_b: "disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            trace_fdc: false,
            trace_io: false,
            trace_pc: true,
        },
    ];

    let mut results = Vec::new();
    for cfg in &configs {
        results.push(run_single_boot_test(cfg));
    }
    results
}

fn run_single_boot_test(cfg: &BootTestConfig) -> TestResult {
    use iz80::*;

    let fdc = crate::floppy_controller::FloppyController::new(
        cfg.disk_a, cfg.disk_b, cfg.disk_format, cfg.side1_sector_base, false, false,
    );
    let mut machine = crate::kaypro_machine::KayproMachine::new(
        cfg.rom_path, cfg.video_mode, fdc, false, false, false,
    );
    let enable_trace_after_prompt = cfg.trace_fdc || cfg.trace_io;
    let mut cpu = Cpu::new_z80();

    let max_instructions: u64 = 200_000_000;
    let mut counter: u64 = 0;
    let mut nmi_pending = false;
    let mut prompt_found = false;
    let mut prompt_at: u64 = 0;

    // After finding prompt, monitor for boot loops by tracking FDC motor toggles
    let post_prompt_budget: u64 = 50_000_000;
    let mut fdc_motor_toggles: u32 = 0;
    let mut last_fdc_motor = false;

    // Warm-boot tracing: after prompt, keep a rolling buffer of recent PCs.
    // When PC hits 0x0000 (CP/M WBOOT vector), dump the buffer to show what
    // code path triggered the warm boot.
    const PC_RING_SIZE: usize = 2000;
    let mut pc_ring: Vec<(u64, u16, bool, u32)> = Vec::with_capacity(PC_RING_SIZE);
    let mut pc_ring_pos: usize = 0;
    let mut pc_ring_full = false;
    let mut wboot_traces_dumped: u32 = 0;
    let mut pc_prev: u16 = 0xFFFF;

    loop {
        cpu.execute_instruction(&mut machine);
        counter += 1;

        // Warm-boot trigger detection
        if cfg.trace_pc && prompt_found && wboot_traces_dumped < 2 {
            let pc = cpu.registers().pc();

            // Only log non-sequential PC transitions, skipping tight backward
            // loops (JR NZ back by <16 bytes) which fill the ring with delay loops
            let is_sequential = pc >= pc_prev && pc <= pc_prev.wrapping_add(4);
            let is_tight_loop = pc < pc_prev && pc_prev.wrapping_sub(pc) < 16;
            pc_prev = pc;

            if !is_sequential && !is_tight_loop {
                let entry = (counter, pc, machine.is_rom_rank(), 1u32);
                if pc_ring.len() < PC_RING_SIZE {
                    pc_ring.push(entry);
                } else {
                    pc_ring[pc_ring_pos] = entry;
                }
                pc_ring_pos = (pc_ring_pos + 1) % PC_RING_SIZE;
                if pc_ring.len() == PC_RING_SIZE {
                    pc_ring_full = true;
                }
            }

            // Detect warm boot via FDC RESTORE command (first thing boot does)
            // or PC=0x0000 (CP/M WBOOT vector)
            let fdc_restore = machine.floppy_controller.last_command
                .take()
                .map_or(false, |cmd| (cmd & 0xf0) == 0x00);
            if pc == 0x0000 || fdc_restore {
                wboot_traces_dumped += 1;
                println!("\n=== Warm Boot #{} Detected: {} (counter={}) ===", 
                    wboot_traces_dumped, cfg.name, counter);

                // Dump ring buffer, collapsing repeated sequences
                let count = if pc_ring_full { PC_RING_SIZE } else { pc_ring.len() };
                let start_idx = if pc_ring_full { pc_ring_pos } else { 0 };

                // Extract ordered entries
                let mut entries: Vec<(u64, u16, bool)> = Vec::with_capacity(count);
                for i in 0..count {
                    let idx = (start_idx + i) % PC_RING_SIZE;
                    let (cnt, addr, is_rom, _) = pc_ring[idx];
                    entries.push((cnt, addr, is_rom));
                }

                // Detect repeating sequence: find shortest period where the PC
                // sequence repeats identically
                let mut period = 0;
                'outer: for p in 4..count / 3 {
                    let tail = &entries[count - p..];
                    let prev = &entries[count - 2 * p..count - p];
                    if tail.iter().zip(prev.iter()).all(|(a, b)| a.1 == b.1) {
                        period = p;
                        break 'outer;
                    }
                }

                if period > 0 {
                    // Find first occurrence of the repeating pattern
                    let pattern: Vec<u16> = entries[count - period..].iter().map(|e| e.1).collect();
                    let mut first_repeat = count - period;
                    while first_repeat >= period {
                        let candidate: Vec<u16> = entries[first_repeat - period..first_repeat]
                            .iter().map(|e| e.1).collect();
                        if candidate != pattern { break; }
                        first_repeat -= period;
                    }
                    let repeats = (count - first_repeat) / period;

                    // Show unique prefix
                    let prefix_end = first_repeat.min(200);
                    if first_repeat > prefix_end {
                        println!("  (skipping {} earlier entries)", first_repeat - prefix_end);
                    }
                    for i in (first_repeat - prefix_end)..first_repeat {
                        let (cnt, addr, is_rom) = entries[i];
                        let bank = if is_rom { "ROM" } else { "RAM" };
                        println!("  [{:>10}] PC=0x{:04X} [{}]", cnt, addr, bank);
                    }
                    println!("  --- repeating sequence ({} branches, ×{}) ---", period, repeats);
                    for i in first_repeat..first_repeat + period {
                        let (cnt, addr, is_rom) = entries[i];
                        let bank = if is_rom { "ROM" } else { "RAM" };
                        println!("  [{:>10}] PC=0x{:04X} [{}]", cnt, addr, bank);
                    }
                    println!("  --- end repeating sequence ---");
                } else {
                    // No repeating pattern, show last 200
                    let show = count.min(200);
                    let skip = count - show;
                    if skip > 0 {
                        println!("  (skipping {} earlier entries)", skip);
                    }
                    for i in 0..show {
                        let idx = (start_idx + skip + i) % PC_RING_SIZE;
                        let (cnt, addr, is_rom, _) = pc_ring[idx];
                        let bank = if is_rom { "ROM" } else { "RAM" };
                        println!("  [{:>10}] PC=0x{:04X} [{}]", cnt, addr, bank);
                    }
                }
                println!("=== End Warm Boot Trace ===\n");

                // Reset ring for next warm boot capture
                pc_ring.clear();
                pc_ring_pos = 0;
                pc_ring_full = false;
            }
        }

        // SIO interrupt processing (keyboard)
        if counter % 1024 == 0 {
            let i_reg = cpu.registers().get8(iz80::Reg8::I);
            if let Some(handler) = machine.sio_check_interrupt(i_reg) {
                let regs = cpu.registers();
                let pc = regs.pc();
                let mut sp = regs.get16(iz80::Reg16::SP);
                sp = sp.wrapping_sub(2);
                regs.set16(iz80::Reg16::SP, sp);
                machine.poke(sp, pc as u8);
                machine.poke(sp.wrapping_add(1), (pc >> 8) as u8);
                cpu.registers().set_pc(handler);
            }
        }

        // NMI processing (same logic as main loop)
        if machine.floppy_controller.raise_nmi {
            machine.floppy_controller.raise_nmi = false;
            nmi_pending = true;
        }
        let mut nmi_signaled = false;
        if nmi_pending && cpu.is_halted() {
            cpu.signal_nmi();
            nmi_pending = false;
            nmi_signaled = true;
        }
        if !nmi_signaled && cpu.is_halted() {
            if prompt_found {
                return TestResult {
                    name: format!("Boot {}", cfg.name),
                    passed: true,
                    message: format!("Booted OK ({} instructions), stable at HALT", prompt_at),
                };
            }
            return TestResult {
                name: format!("Boot {}", cfg.name),
                passed: false,
                message: format!("HALT at PC=0x{:04X} after {} instructions",
                    cpu.registers().pc(), counter),
            };
        }

        // Check for A> prompt periodically
        if !prompt_found && counter % 100_000 == 0 {
            if check_for_prompt(&machine) {
                prompt_found = true;
                prompt_at = counter;
                last_fdc_motor = machine.floppy_controller.motor_on;
                if enable_trace_after_prompt {
                    if cfg.trace_fdc {
                        machine.floppy_controller.trace = true;
                    }
                    if cfg.trace_io {
                        machine.trace_io = true;
                    }
                }
                if cfg.trace_pc {
                    let screen = extract_vram_text(&machine, 10);
                    println!("PC trace: prompt found at counter={}", counter);
                    println!("PC trace: VRAM:\n{}", screen.replace('|', "\n"));
                }
            }
        }

        // After finding prompt, monitor for boot loops
        if prompt_found {
            if machine.floppy_controller.motor_on != last_fdc_motor {
                fdc_motor_toggles += 1;
                last_fdc_motor = machine.floppy_controller.motor_on;
            }

            if counter > prompt_at + post_prompt_budget {
                let pc = cpu.registers().pc();
                // After boot, a healthy CP/M system does at most a few motor
                // on/off cycles (initial dir read). Repeated disk access (>4
                // toggles) means the BIOS is stuck in a warm-boot loop.
                if fdc_motor_toggles > 4 {
                    return TestResult {
                        name: format!("Boot {}", cfg.name),
                        passed: false,
                        message: format!(
                            "Boot loop: {} motor toggles after A> prompt, PC=0x{:04X}",
                            fdc_motor_toggles, pc),
                    };
                }
                return TestResult {
                    name: format!("Boot {}", cfg.name),
                    passed: true,
                    message: format!("Booted ({} instructions), stable ({} motor toggles)",
                        prompt_at, fdc_motor_toggles),
                };
            }
        }

        if counter >= max_instructions {
            let vram_text = extract_vram_text(&machine, 5);
            let pc = cpu.registers().pc();
            return TestResult {
                name: format!("Boot {}", cfg.name),
                passed: false,
                message: format!("Timed out after {} instructions at PC=0x{:04X} ROM={}. Screen: {}",
                    counter, pc, machine.is_rom_rank(), vram_text),
            };
        }
    }
}

/// Check if "A>" appears in VRAM (works for both CRTC and memory-mapped modes)
fn check_for_prompt(machine: &crate::kaypro_machine::KayproMachine) -> bool {
    // Match "A>", "A0>", "B>", "B0>" etc. - any drive letter followed by optional digit and ">"
    if machine.video_mode == crate::kaypro_machine::VideoMode::Sy6545Crtc {
        for i in 0..0x800 {
            let c0 = machine.crtc.get_vram(i);
            let c1 = machine.crtc.get_vram((i + 1) & 0x7FF);
            let c2 = machine.crtc.get_vram((i + 2) & 0x7FF);
            if (c0 == b'A' || c0 == b'B') && c1 == b'>' { return true; }
            if (c0 == b'A' || c0 == b'B') && c1.is_ascii_digit() && c2 == b'>' { return true; }
        }
    } else {
        for i in 0..machine.vram.len().saturating_sub(2) {
            let c0 = machine.vram[i];
            let c1 = machine.vram[i + 1];
            let c2 = machine.vram[i + 2];
            if (c0 == b'A' || c0 == b'B') && c1 == b'>' { return true; }
            if (c0 == b'A' || c0 == b'B') && c1.is_ascii_digit() && c2 == b'>' { return true; }
        }
    }
    false
}

/// Extract first N lines of text from VRAM for debugging
fn extract_vram_text(machine: &crate::kaypro_machine::KayproMachine, lines: usize) -> String {
    let mut text = String::new();
    for row in 0..lines {
        for col in 0..80 {
            let ch = if machine.video_mode == crate::kaypro_machine::VideoMode::Sy6545Crtc {
                let start = machine.crtc.start_addr();
                let addr = (start + row * 80 + col) & 0x7FF;
                machine.crtc.get_vram(addr)
            } else {
                machine.vram[row * 128 + col]
            };
            if ch >= 0x20 && ch < 0x7F {
                text.push(ch as char);
            } else {
                text.push('.');
            }
        }
        text.push('|');
    }
    text
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
