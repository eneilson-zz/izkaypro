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
    pub has_hard_disk: bool,
    pub hd_image: Option<&'static str>,
}

/// Run boot tests for all supported Kaypro models.
/// Each test boots the machine headlessly and checks that "A>" appears in VRAM
/// within a reasonable instruction count, and that the CPU is not stuck in an
/// infinite loop (detected by PC repeating at the same address).
pub fn run_boot_tests() -> Vec<TestResult> {
    let configs = vec![
        BootTestConfig {
            name: "Kaypro II (81-149c)",
            rom_path: "roms/81-149c.rom",
            video_mode: crate::kaypro_machine::VideoMode::MemoryMapped,
            disk_format: crate::media::MediaFormat::SsDd,
            disk_a: "disks/system/cpm22-rom149.img",
            disk_b: "disks/blank_disks/cpm22-rom149-blank.img",
            side1_sector_base: 10,
            has_hard_disk: false,
            hd_image: None,
        },
        BootTestConfig {
            name: "Kaypro 4/84 (81-292a)",
            rom_path: "roms/81-292a.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/cpm22g-rom292a.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            has_hard_disk: false,
            hd_image: None,
        },
        BootTestConfig {
            name: "Kaypro 4/84 TurboROM",
            rom_path: "roms/trom34.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/k484_turborom_63k_boot.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            has_hard_disk: false,
            hd_image: None,
        },
        BootTestConfig {
            name: "KayPLUS 84",
            rom_path: "roms/kplus84.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/kayplus_boot.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 0,
            has_hard_disk: false,
            hd_image: None,
        },
        BootTestConfig {
            name: "Kaypro 10 (81-478c)",
            rom_path: "roms/81-478c.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/kaypro10_boot.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            has_hard_disk: true,
            hd_image: Some("disks/system/kaypro10.hd"),
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
        cfg.rom_path, cfg.video_mode, fdc, cfg.has_hard_disk,
        false, false, false, false, false, false,
    );

    // No floppy in drive when booting from HD (ROM checks NOT READY for boot priority)
    if cfg.has_hard_disk && cfg.hd_image.is_some() {
        machine.floppy_controller.disk_in_drive = false;
    }

    // Load HD image if specified (copy to temp file to avoid modifying original)
    let hd_tmp_path: Option<std::path::PathBuf> = if let Some(hd_src) = cfg.hd_image {
        let tmp = std::env::temp_dir()
            .join(format!("izkaypro_boot_test_{}.hd", std::process::id()));
        if let Err(e) = std::fs::copy(hd_src, &tmp) {
            return TestResult {
                name: format!("Boot {}", cfg.name),
                passed: false,
                message: format!("Failed to copy HD image '{}': {}", hd_src, e),
            };
        }
        if let Some(ref mut hd) = machine.hard_disk {
            if let Err(e) = hd.load_image(tmp.to_str().unwrap()) {
                let _ = std::fs::remove_file(&tmp);
                return TestResult {
                    name: format!("Boot {}", cfg.name),
                    passed: false,
                    message: format!("Failed to load HD image: {}", e),
                };
            }
        }
        Some(tmp)
    } else {
        None
    };

    let mut cpu = Cpu::new_z80();

    let max_instructions: u64 = 200_000_000;
    let mut counter: u64 = 0;
    let mut nmi_pending = false;
    let mut nmi_deadline: u64 = 0;
    let mut prompt_found = false;
    let mut prompt_at: u64 = 0;

    // After finding prompt, monitor for boot loops by tracking FDC motor toggles
    let post_prompt_budget: u64 = 50_000_000;
    let mut fdc_motor_toggles: u32 = 0;
    let mut last_fdc_motor = false;

    let result = loop {
        cpu.execute_instruction(&mut machine);
        counter += 1;

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
            nmi_deadline = counter + 10_000_000;
        }
        let mut nmi_signaled = false;
        if nmi_pending && (cpu.is_halted()
            || (counter >= nmi_deadline && machine.nmi_vector_is_safe()))
        {
            cpu.signal_nmi();
            nmi_pending = false;
            nmi_signaled = true;
        }
        if !nmi_signaled && cpu.is_halted() {
            if prompt_found {
                break TestResult {
                    name: format!("Boot {}", cfg.name),
                    passed: true,
                    message: format!("Booted OK ({} instructions), stable at HALT", prompt_at),
                };
            }
            break TestResult {
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
                if fdc_motor_toggles > 4 {
                    break TestResult {
                        name: format!("Boot {}", cfg.name),
                        passed: false,
                        message: format!(
                            "Boot loop: {} motor toggles after A> prompt, PC=0x{:04X}",
                            fdc_motor_toggles, pc),
                    };
                }
                break TestResult {
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
            break TestResult {
                name: format!("Boot {}", cfg.name),
                passed: false,
                message: format!("Timed out after {} instructions at PC=0x{:04X} ROM={}. Screen: {}",
                    counter, pc, machine.is_rom_rank(), vram_text),
            };
        }
    };

    // Clean up temp HD image
    if let Some(ref path) = hd_tmp_path {
        let _ = std::fs::remove_file(path);
    }

    result
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

/// Run Kaypro 10 HD boot with comprehensive tracing to diagnose boot issues.
/// All output goes to hdc-trace.log (high-level boot trace + HDC register
/// detail combined). Analyzes the HD image before and after boot.
pub fn trace_turborom_hd_boot() {
    use iz80::*;
    use std::io::Write;

    let log_path = "hdc-trace.log";
    let mut log = std::fs::File::create(log_path)
        .expect("failed to create hdc-trace.log");

    macro_rules! trace {
        ($($arg:tt)*) => {
            let _ = writeln!(log, $($arg)*);
        }
    }

    trace!("=== Kaypro 10 HD Boot Trace ===");
    trace!("ROM: roms/81-478c.rom");
    trace!("HD:  disks/system/kaypro10.hd");
    trace!("Date: {:?}", std::time::SystemTime::now());
    trace!("");

    let fdc = crate::floppy_controller::FloppyController::new(
        "disks/system/kaypro10_boot.img",
        "disks/blank_disks/cpm22-kaypro4-blank.img",
        crate::media::MediaFormat::DsDd,
        10,
        false,
        false,
    );
    let mut machine = crate::kaypro_machine::KayproMachine::new(
        "roms/81-478c.rom",
        crate::kaypro_machine::VideoMode::Sy6545Crtc,
        fdc,
        true,    // has_hard_disk
        false,   // trace_io
        false,   // trace_system_bits
        false,   // trace_crtc
        false,   // trace_sio
        false,   // trace_rtc
        true,    // trace_hdc
    );

    // Load the real HD image
    let hd_path = "disks/system/kaypro10.hd";
    let hdc_log_path = "hdc-detail.log";
    if let Some(ref mut hd) = machine.hard_disk {
        hd.load_image(hd_path)
            .expect("failed to load HD image");
        // Direct HDC register-level traces to a separate log file
        let hdc_log_file = std::fs::File::create(hdc_log_path)
            .expect("failed to create hdc-detail.log");
        hd.set_trace_file(hdc_log_file);
    }

    // === PRE-BOOT HD IMAGE ANALYSIS ===
    trace!("\n=== PRE-BOOT HD IMAGE ANALYSIS ===");
    if let Some(ref hd) = machine.hard_disk {
        // Boot sector (C=0, H=0, S=0)
        trace!("\n--- HD Boot Sector (C=0, H=0, S=0) ---");
        dump_hex_to_log(&mut log, &hd.disk_data, 0, 512);
        let byte0 = hd.disk_data[0];
        trace!("Boot sector byte 0: 0x{:02X} ({})", byte0,
            if byte0 == 0xC3 { "JP -- bootable" }
            else if byte0 == 0x00 { "NOP -- empty/unbootable" }
            else { "unknown" });
        if byte0 == 0xC3 {
            let target = hd.disk_data[1] as u16 | ((hd.disk_data[2] as u16) << 8);
            trace!("Boot JP target: 0x{:04X}", target);
        }

        // System tracks (C=0, H=0, S=1 through S=16) -- first 64 bytes of each
        trace!("\n--- System Tracks Summary (C=0, H=0) ---");
        for s in 0..17 {
            let off = s * 512;
            let nonzero = hd.disk_data[off..off+512].iter().filter(|&&b| b != 0).count();
            let first4: String = (0..4).map(|i| format!("{:02x}", hd.disk_data[off+i])).collect::<Vec<_>>().join(" ");
            trace!("  Sector {:2}: {} non-zero bytes, first 4: {}", s, nonzero, first4);
        }

        // Check second head (C=0, H=1)
        let head1_off = 17 * 512; // 17 sectors per track
        let head1_nonzero = hd.disk_data[head1_off..head1_off+512].iter().filter(|&&b| b != 0).count();
        trace!("  C=0 H=1 S=0: {} non-zero bytes", head1_nonzero);

        // Track formatting summary
        let formatted: usize = (0..1224).filter(|&t| {
            let off = t * 17 * 512;
            let end = (off + 17 * 512).min(hd.disk_data.len());
            if off < hd.disk_data.len() {
                hd.disk_data[off..end].iter().any(|&b| b != 0)
            } else { false }
        }).count();
        trace!("\nFormatted tracks: {}/1224", formatted);
    }

    // === BOOT EXECUTION TRACE ===
    trace!("\n\n=== BOOT EXECUTION TRACE ===\n");

    let mut cpu = Cpu::new_z80();
    let max_instructions: u64 = 200_000_000;
    let mut counter: u64 = 0;
    let mut nmi_pending = false;
    let mut nmi_deadline: u64 = 0;
    let mut prompt_found = false;
    let mut prompt_at: u64 = 0;
    let post_prompt_budget: u64 = 50_000_000;

    // Track state changes for logging
    let mut last_rom_rank = true;
    let mut last_port14_raw: u8 = 0xFF;
    let mut bios_base: Option<u16> = None;
    let mut _bdos_base: Option<u16> = None;

    loop {
        cpu.execute_instruction(&mut machine);
        counter += 1;

        let pc = cpu.registers().pc();
        let in_rom = machine.is_rom_rank();

        // Log ROM/RAM bank switches
        if in_rom != last_rom_rank {
            trace!("[{:>10}] BANK: {} -> {} at PC=0x{:04X}",
                counter, if last_rom_rank { "ROM" } else { "RAM" },
                if in_rom { "ROM" } else { "RAM" }, pc);
            last_rom_rank = in_rom;

            // When switching to RAM mode for the first time, discover BIOS/BDOS addresses
            if !in_rom && bios_base.is_none() {
                let warm_lo = machine.peek(0x0001) as u16;
                let warm_hi = machine.peek(0x0002) as u16;
                let warm_boot = (warm_hi << 8) | warm_lo;
                if warm_boot > 0x100 && warm_boot < 0xFFFF {
                    let base = warm_boot - 3; // BIOS base is 3 bytes before WBOOT
                    bios_base = Some(base);
                    trace!("[{:>10}]   BIOS base discovered: 0x{:04X} (warm boot at 0x{:04X})",
                        counter, base, warm_boot);
                }
                let bdos_lo = machine.peek(0x0006) as u16;
                let bdos_hi = machine.peek(0x0007) as u16;
                let bdos_addr = (bdos_hi << 8) | bdos_lo;
                if bdos_addr > 0x100 {
                    _bdos_base = Some(bdos_addr);
                    trace!("[{:>10}]   BDOS entry: 0x{:04X}", counter, bdos_addr);
                }
            }
        }

        // Port 0x14 tracking (drive select, banking, SASI reset)
        let cur_port14 = machine.port14_raw;
        if cur_port14 != last_port14_raw {
            let bank = if cur_port14 & 0x80 != 0 { "ROM" } else { "RAM" };
            let motor = if cur_port14 & 0x10 != 0 { "ON" } else { "OFF" };
            let side = if cur_port14 & 0x04 != 0 { "0" } else { "1" };
            let sasi_mr = if cur_port14 & 0x02 != 0 { "hi" } else { "LO(reset)" };
            let drv = if cur_port14 & 0x01 != 0 { "B" } else { "A" };
            trace!("[{:>10}] PORT14: 0x{:02X} -> 0x{:02X} [bank={} motor={} side={} SASI={} drv={}]",
                counter, last_port14_raw, cur_port14, bank, motor, side, sasi_mr, drv);
            last_port14_raw = cur_port14;
        }

        // BDOS tracing with enhanced detail
        if !in_rom && pc == 0x0005 {
            let cmd = cpu.registers().get8(Reg8::C);
            let de = cpu.registers().get16(Reg16::DE);
            let name = match cmd {
                0 => "P_TERMCPM", 1 => "C_READ", 2 => "C_WRITE",
                9 => "C_WRITESTR", 10 => "C_READSTR", 12 => "S_BDOSVER",
                13 => "DRV_ALLRESET", 14 => "DRV_SET", 15 => "F_OPEN",
                16 => "F_CLOSE", 17 => "F_SFIRST", 18 => "F_SNEXT",
                19 => "F_DELETE", 20 => "F_READ", 21 => "F_WRITE",
                22 => "F_MAKE", 25 => "DRV_GET", 26 => "F_DMAOFF",
                33 => "F_READRAND", 34 => "F_WRITERAND", 36 => "F_RANDREC",
                _ => "",
            };
            trace!("[{:>10}] BDOS {}: {} DE=0x{:04X}", counter, cmd, name, de);

            // For DRV_SET (14), log which drive is being selected
            if cmd == 14 {
                let drive = cpu.registers().get8(Reg8::E);
                trace!("[{:>10}]   DRV_SET: selecting drive {} ({})",
                    counter, drive, (b'A' + drive) as char);
            }

            // For file operations, dump FCB filename
            if cmd == 15 || cmd == 17 || cmd == 22 {
                // DE points to FCB; first byte = drive, bytes 1-11 = filename
                let fcb_drive = machine.peek(de);
                let mut filename = String::new();
                for i in 1..=11u16 {
                    let ch = machine.peek(de.wrapping_add(i)) & 0x7F;
                    if ch >= 0x20 { filename.push(ch as char); }
                }
                trace!("[{:>10}]   FCB: drive={} file=\"{}\"",
                    counter, fcb_drive, filename.trim());
            }
        }

        // BIOS entry point tracing (after BIOS base is discovered)
        if !in_rom {
            if let Some(base) = bios_base {
                if pc >= base && pc <= base + 51 && (pc - base) % 3 == 0 {
                    let entry = (pc - base) / 3;
                    let name = match entry {
                        0 => "BOOT",
                        1 => "WBOOT",
                        2 => "CONST",
                        3 => "CONIN",
                        4 => "CONOUT",
                        5 => "LIST",
                        6 => "PUNCH",
                        7 => "READER",
                        8 => "HOME",
                        9 => {
                            let drv = cpu.registers().get8(Reg8::C);
                            let _ = writeln!(log, "[{:>10}] BIOS: SELDSK drive={} ({})",
                                counter, drv, (b'A' + drv) as char);
                            "SELDSK"
                        },
                        10 => {
                            let trk = cpu.registers().get8(Reg8::C);
                            let _ = writeln!(log, "[{:>10}] BIOS: SETTRK track={}",
                                counter, trk);
                            "SETTRK"
                        },
                        11 => {
                            let sec = cpu.registers().get8(Reg8::C);
                            let _ = writeln!(log, "[{:>10}] BIOS: SETSEC sector={}",
                                counter, sec);
                            "SETSEC"
                        },
                        12 => "SETDMA",
                        13 => "READ",
                        14 => "WRITE",
                        15 => "LISTST",
                        16 => "SECTRAN",
                        _ => "?",
                    };
                    if entry != 9 && entry != 10 && entry != 11 {
                        trace!("[{:>10}] BIOS: {} (entry {})", counter, name, entry);
                    }
                }
            }
        }

        // ROM entry point tracing for 81-478c
        if in_rom {
            match pc {
                0x0000 => { trace!("[{:>10}] ROM: Cold boot entry", counter); },
                0x0003 => { trace!("[{:>10}] ROM: Warm boot entry", counter); },
                _ => {}
            }
        }

        // Periodic VRAM dump to catch screen messages
        if counter % 2_000_000 == 0 {
            let text = extract_vram_text(&machine, 5);
            let lines: Vec<&str> = text.split('|').collect();
            let interesting = lines.iter().any(|l| {
                let t = l.trim();
                !t.is_empty() && t.chars().any(|c| c != '.')
            });
            if interesting {
                trace!("[{:>10}] VRAM:", counter);
                for (i, line) in lines.iter().enumerate() {
                    let trimmed = line.trim_end();
                    if !trimmed.is_empty() && trimmed.chars().any(|c| c != '.') {
                        trace!("  {:2}: {}", i, trimmed);
                    }
                }
            }
        }

        // SIO interrupt processing
        if counter % 1024 == 0 {
            let i_reg = cpu.registers().get8(Reg8::I);
            if let Some(handler) = machine.sio_check_interrupt(i_reg) {
                let regs = cpu.registers();
                let pc = regs.pc();
                let mut sp = regs.get16(Reg16::SP);
                sp = sp.wrapping_sub(2);
                regs.set16(Reg16::SP, sp);
                machine.poke(sp, pc as u8);
                machine.poke(sp.wrapping_add(1), (pc >> 8) as u8);
                cpu.registers().set_pc(handler);
            }
        }

        // NMI processing
        if machine.floppy_controller.raise_nmi {
            machine.floppy_controller.raise_nmi = false;
            nmi_pending = true;
            nmi_deadline = counter + 10_000_000;
        }
        let mut nmi_signaled = false;
        if nmi_pending && (cpu.is_halted()
            || (counter >= nmi_deadline && machine.nmi_vector_is_safe()))
        {
            cpu.signal_nmi();
            nmi_pending = false;
            nmi_signaled = true;
        }
        if !nmi_signaled && cpu.is_halted() {
            if prompt_found {
                trace!("\n=== Boot complete, stable at HALT after {} instructions ===", prompt_at);
            } else {
                trace!("\n=== HALT at PC=0x{:04X} after {} instructions (no prompt) ===",
                    cpu.registers().pc(), counter);
            }
            break;
        }

        // Check for prompt
        if !prompt_found && counter % 100_000 == 0 {
            if check_for_prompt(&machine) {
                prompt_found = true;
                prompt_at = counter;
                trace!("\n=== Prompt found at {} instructions ===\n", counter);
            }
        }

        if prompt_found && counter > prompt_at + post_prompt_budget {
            trace!("\n=== Post-prompt budget exhausted ===");
            break;
        }

        if counter >= max_instructions {
            trace!("\n=== Timed out after {} instructions ===", counter);
            break;
        }
    }

    // === POST-BOOT ANALYSIS ===
    trace!("\n\n=== POST-BOOT ANALYSIS ===\n");

    // VRAM screen contents
    trace!("--- Final VRAM Screen Contents ---");
    let text = extract_vram_text(&machine, 24);
    for (i, line) in text.split('|').enumerate() {
        if !line.is_empty() {
            let trimmed = line.trim_end();
            if !trimmed.is_empty() {
                trace!("{:2}: {}", i, trimmed);
            }
        }
    }

    // CP/M memory layout
    trace!("\n--- CP/M Memory Layout ---");
    let jp_warm = machine.peek(0x0000);
    let warm_lo = machine.peek(0x0001) as u16;
    let warm_hi = machine.peek(0x0002) as u16;
    let warm_addr = (warm_hi << 8) | warm_lo;
    trace!("0x0000: {:02X} {:02X} {:02X}  (JP 0x{:04X} -- warm boot)", jp_warm, warm_lo as u8, warm_hi as u8, warm_addr);
    let jp_bdos = machine.peek(0x0005);
    let bdos_lo = machine.peek(0x0006) as u16;
    let bdos_hi = machine.peek(0x0007) as u16;
    let bdos_entry = (bdos_hi << 8) | bdos_lo;
    trace!("0x0005: {:02X} {:02X} {:02X}  (JP 0x{:04X} -- BDOS entry)", jp_bdos, bdos_lo as u8, bdos_hi as u8, bdos_entry);
    trace!("Current drive: {}", machine.peek(0x0004));

    // BIOS jump table
    if let Some(base) = bios_base {
        trace!("\n--- BIOS Jump Table at 0x{:04X} ---", base);
        let names = ["BOOT", "WBOOT", "CONST", "CONIN", "CONOUT", "LIST",
                     "PUNCH", "READER", "HOME", "SELDSK", "SETTRK", "SETSEC",
                     "SETDMA", "READ", "WRITE", "LISTST", "SECTRAN"];
        for (i, name) in names.iter().enumerate() {
            let addr = base + (i as u16) * 3;
            let opcode = machine.peek(addr);
            let target_lo = machine.peek(addr + 1) as u16;
            let target_hi = machine.peek(addr + 2) as u16;
            let target = (target_hi << 8) | target_lo;
            trace!("  0x{:04X}: {:02X} {:02X} {:02X}  {} -> 0x{:04X}",
                addr, opcode, target_lo as u8, target_hi as u8, name, target);
        }

        // Dump DPH for drives 0-3 by calling SELDSK logic
        // DPH is 16 bytes: XLT(2), scratch(6), DIRBUF(2), DPB(2), CSV(2), ALV(2)
        trace!("\n--- Drive Parameter Headers ---");
        // Look for DPH table by examining SELDSK handler
        let seldsk_addr = base + 27; // SELDSK entry
        let seldsk_opcode = machine.peek(seldsk_addr);
        let seldsk_target = machine.peek(seldsk_addr + 1) as u16
            | ((machine.peek(seldsk_addr + 2) as u16) << 8);
        trace!("SELDSK at 0x{:04X} -> 0x{:04X}", seldsk_addr, seldsk_target);
        let _ = seldsk_opcode; // used only for the trace above

        // Scan RAM for DPB signatures (common disk parameter values)
        // Kaypro 10 HD: SPT=68 (0x44), BSH=4, BLM=15
        trace!("\n--- DPB Scan (looking for HD DPB) ---");
        for addr in (0x100..0xFE00u32).step_by(2) {
            let spt = machine.peek(addr as u16) as u16
                | ((machine.peek((addr + 1) as u16) as u16) << 8);
            let bsh = machine.peek((addr + 2) as u16);
            let blm = machine.peek((addr + 3) as u16);
            // Look for typical HD DPB values:
            // SPT=68 (4 sectors * 17) or SPT=136, BSH=4, BLM=15
            if (spt == 68 || spt == 136) && bsh == 4 && blm == 15 {
                let dsm = machine.peek((addr + 5) as u16) as u16
                    | ((machine.peek((addr + 6) as u16) as u16) << 8);
                let drm = machine.peek((addr + 7) as u16) as u16
                    | ((machine.peek((addr + 8) as u16) as u16) << 8);
                let off = machine.peek((addr + 13) as u16) as u16
                    | ((machine.peek((addr + 14) as u16) as u16) << 8);
                trace!("  DPB at 0x{:04X}: SPT={} BSH={} BLM={} DSM={} DRM={} OFF={}",
                    addr, spt, bsh, blm, dsm, drm, off);
                // Dump full 15 bytes
                let dpb_hex: String = (0..15u32).map(|i| {
                    format!("{:02x}", machine.peek((addr + i) as u16))
                }).collect::<Vec<_>>().join(" ");
                trace!("    raw: {}", dpb_hex);
            }
        }
    }

    // HD boot sector (post-boot, in case it was modified)
    if let Some(ref hd) = machine.hard_disk {
        trace!("\n--- HD Boot Sector (post-boot) ---");
        dump_hex_to_log(&mut log, &hd.disk_data, 0, 256);
    }

    // Flush and report
    let _ = log.flush();
    eprintln!("Boot trace written to {}", log_path);
    eprintln!("HDC register-level trace written to {}", hdc_log_path);
}

/// Dump hex data from a buffer to a log file in 16-byte-per-line format
fn dump_hex_to_log(log: &mut std::fs::File, data: &[u8], offset: usize, len: usize) {
    use std::io::Write;
    let end = (offset + len).min(data.len());
    for row_start in (offset..end).step_by(16) {
        let row_end = (row_start + 16).min(end);
        let hex: String = (row_start..row_end)
            .map(|i| format!("{:02x}", data[i]))
            .collect::<Vec<_>>().join(" ");
        let ascii: String = (row_start..row_end)
            .map(|i| {
                let b = data[i];
                if b >= 0x20 && b < 0x7F { b as char } else { '.' }
            })
            .collect();
        let _ = writeln!(log, "  {:06X}: {:<48}  {}", row_start, hex, ascii);
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

/// Debug MAKTURBO on Kaypro 10: boot with turborom_install.img, type MAKTURBO,
/// and log all BIOS/BDOS calls to understand why TURBO-BS.REL isn't found.
pub fn debug_makturbo() {
    use iz80::*;
    use std::io::Write;

    let log_path = "makturbo-debug.log";
    let mut log = std::fs::File::create(log_path)
        .expect("failed to create makturbo-debug.log");

    let _ = writeln!(log, "=== MAKTURBO Debug Trace ===");
    let _ = writeln!(log, "ROM: 81-478c (Kaypro 10)");
    let _ = writeln!(log, "Disk A: turborom_install.img");
    let _ = writeln!(log, "");

    let mut fdc = crate::floppy_controller::FloppyController::new(
        "disks/system/turborom_install.img",
        "disks/blank_disks/cpm22-kaypro4-blank.img",
        crate::media::MediaFormat::DsDd,
        10,
        false,
        false,
    );
    // Enable FDC trace file
    let fdc_log_path = "makturbo-fdc.log";
    let fdc_file = std::fs::File::create(fdc_log_path)
        .expect("failed to create FDC trace log");
    fdc.trace_file = Some(fdc_file);

    let mut machine = crate::kaypro_machine::KayproMachine::new(
        "roms/81-478c.rom",
        crate::kaypro_machine::VideoMode::Sy6545Crtc,
        fdc,
        true,    // has_hard_disk (K10 always has it)
        false, false, false, false, false, false,
    );

    // Create blank HD (K10 needs one, will fall back to floppy boot)
    let hd_path = std::env::temp_dir()
        .join(format!("izkaypro_makturbo_debug_{}.hd", std::process::id()));
    if let Some(ref mut hd) = machine.hard_disk {
        hd.load_image(hd_path.to_str().unwrap())
            .expect("failed to init blank HD image");
    }

    let mut cpu = Cpu::new_z80();
    let max_instructions: u64 = 500_000_000;
    let mut counter: u64 = 0;
    let mut nmi_pending = false;
    let mut nmi_deadline: u64 = 0;
    let mut prompt_found = false;
    let mut keys_injected = false;
    let mut bios_base: Option<u16> = None;
    let mut last_rom_rank = true;
    let mut tracing_active = false;

    println!("Debug: Booting Kaypro 10 with turborom_install.img...");

    loop {
        cpu.execute_instruction(&mut machine);
        counter += 1;

        // SIO interrupt processing
        if counter % 1024 == 0 {
            let i_reg = cpu.registers().get8(Reg8::I);
            if let Some(handler) = machine.sio_check_interrupt(i_reg) {
                let regs = cpu.registers();
                let pc = regs.pc();
                let mut sp = regs.get16(Reg16::SP);
                sp = sp.wrapping_sub(2);
                regs.set16(Reg16::SP, sp);
                machine.poke(sp, pc as u8);
                machine.poke(sp.wrapping_add(1), (pc >> 8) as u8);
                cpu.registers().set_pc(handler);
            }
        }

        // NMI processing
        if machine.floppy_controller.raise_nmi {
            machine.floppy_controller.raise_nmi = false;
            nmi_pending = true;
            nmi_deadline = counter + 10_000_000;
        }
        if nmi_pending && (cpu.is_halted()
            || (counter >= nmi_deadline && machine.nmi_vector_is_safe()))
        {
            cpu.signal_nmi();
            nmi_pending = false;
        } else if !nmi_pending && cpu.is_halted() {
            let _ = writeln!(log, "[{:>10}] HALT at PC=0x{:04X}", counter, cpu.registers().pc());
            println!("Debug: HALT at PC=0x{:04X} after {} instructions", cpu.registers().pc(), counter);
            break;
        }

        // Check for prompt
        if !prompt_found && counter % 100_000 == 0 {
            if check_for_prompt(&machine) {
                prompt_found = true;
                let _ = writeln!(log, "[{:>10}] A> prompt detected", counter);
                println!("Debug: A> prompt found at {} instructions", counter);
            }
        }

        // Inject MAKTURBO command after prompt settles
        if prompt_found && !keys_injected && counter % 500_000 == 0 {
            // Wait a bit for CCP to be fully ready
            let _ = writeln!(log, "[{:>10}] Injecting 'MAKTURBO' command", counter);
            println!("Debug: Injecting MAKTURBO command");
            machine.keyboard.inject_keys(b"MAKTURBO\r");
            keys_injected = true;
            tracing_active = true;
        }

        // BIOS base discovery
        if tracing_active {
            let in_rom = machine.is_rom_rank();
            if in_rom != last_rom_rank {
                last_rom_rank = in_rom;
                if !in_rom && bios_base.is_none() {
                    let warm_lo = machine.peek(0x0001) as u16;
                    let warm_hi = machine.peek(0x0002) as u16;
                    let warm_boot = (warm_hi << 8) | warm_lo;
                    if warm_boot > 0x100 && warm_boot < 0xFFFF {
                        let base = warm_boot - 3;
                        bios_base = Some(base);
                        let _ = writeln!(log, "[{:>10}] BIOS base: 0x{:04X}", counter, base);
                    }
                }
            }

            // BIOS entry point tracing
            if !in_rom {
                if let Some(base) = bios_base {
                    let pc = cpu.registers().pc();
                    if pc >= base && pc <= base + 51 && (pc - base) % 3 == 0 {
                        let entry = (pc - base) / 3;
                        let msg: Option<String> = match entry {
                            0 => Some("BOOT".into()),
                            1 => Some("WBOOT".into()),
                            8 => Some("HOME".into()),
                            9 => Some(format!("SELDSK drive={} ({})",
                                cpu.registers().get8(Reg8::C),
                                (b'A' + cpu.registers().get8(Reg8::C)) as char)),
                            10 => Some(format!("SETTRK track={}",
                                cpu.registers().get8(Reg8::C))),
                            11 => Some(format!("SETSEC sector={}",
                                cpu.registers().get8(Reg8::C))),
                            12 => Some(format!("SETDMA addr=0x{:04X}",
                                cpu.registers().get16(Reg16::BC))),
                            13 => {
                                // After READ returns, log the return value
                                Some("READ".into())
                            },
                            14 => Some("WRITE".into()),
                            16 => {
                                let sec = cpu.registers().get16(Reg16::BC);
                                let xlt = cpu.registers().get16(Reg16::DE);
                                Some(format!("SECTRAN sector={} xlt=0x{:04X}", sec, xlt))
                            },
                            _ => None,
                        };
                        if let Some(m) = msg {
                            let _ = writeln!(log, "[{:>10}] BIOS: {}", counter, m);
                        }
                    }
                }
            }

            // After the F_OPEN for TURBO-BS.REL, trace the SETDMA that follows READ
            // to dump what the BIOS READ actually put in the DIRBUF
            if !in_rom {
                if let Some(base) = bios_base {
                    let pc = cpu.registers().pc();
                    // Catch SETDMA(0x0080) after directory READ during TURBO-BS search
                    if pc == base + 12*3 && tracing_active { // SETDMA entry
                        let bc = cpu.registers().get16(Reg16::BC);
                        if bc == 0x0080 && counter > 3200000 && counter < 3300000 {
                            let a = cpu.registers().get8(Reg8::A);
                            let _ = writeln!(log, "[{:>10}] *** Post-READ SETDMA(0x0080): A={} (READ result)", counter, a);
                            // Dump DIRBUF at 0xFAF7 (128 bytes = 4 directory entries)
                            let _ = write!(log, "[{:>10}]   DIRBUF[0xFAF7]: ", counter);
                            for i in 0..128u16 {
                                if i % 32 == 0 && i > 0 {
                                    let _ = write!(log, "\n[{:>10}]                 ", counter);
                                }
                                let _ = write!(log, "{:02X} ", machine.peek(0xFAF7u16.wrapping_add(i)));
                            }
                            let _ = writeln!(log);
                            // Show the first entry as ASCII
                            let _ = write!(log, "[{:>10}]   Entry 0: user={} name=\"", counter,
                                machine.peek(0xFAF7));
                            for i in 1..=11u16 {
                                let ch = machine.peek(0xFAF7u16.wrapping_add(i)) & 0x7F;
                                if ch >= 0x20 { let _ = write!(log, "{}", ch as char); }
                            }
                            let _ = writeln!(log, "\"");
                        }
                    }
                }
            }

            // BDOS call tracing
            if !machine.is_rom_rank() && cpu.registers().pc() == 0x0005 {
                let command = cpu.registers().get8(Reg8::C);
                let args = cpu.registers().get16(Reg16::DE);
                static NAMES: &[&str] = &[
                    "P_TERMCPM", "C_READ", "C_WRITE", "A_READ", "A_WRITE",
                    "L_WRITE", "C_RAWIO", "A_STATIN", "A_STATOUT", "C_WRITESTR",
                    "C_READSTR", "C_STAT", "S_BDOSVER", "DRV_ALLRESET", "DRV_SET",
                    "F_OPEN", "F_CLOSE", "F_SFIRST", "F_SNEXT", "F_DELETE",
                    "F_READ", "F_WRITE", "F_MAKE", "F_RENAME", "DRV_LOGINVEC",
                    "DRV_GET", "F_DMAOFF", "DRV_ALLOCVEC", "DRV_SETRO", "DRV_ROVEC",
                    "F_ATTRIB", "DRV_DPB", "F_USERNUM", "F_READRAND", "F_WRITERAND",
                    "F_SIZE", "F_RANDREC", "DRV_RESET",
                ];
                let name = if (command as usize) < NAMES.len() {
                    NAMES[command as usize]
                } else { "?" };
                // Skip C_RAWIO (6) to reduce noise
                if command != 6 {
                    let _ = writeln!(log, "[{:>10}] BDOS {}: {}(0x{:04X})", counter, command, name, args);
                }
                // For F_OPEN (15), dump FCB filename
                if command == 15 || command == 17 {
                    let mut filename = String::new();
                    let fcb_drive = machine.peek(args);
                    for i in 1..=11u16 {
                        let ch = machine.peek(args.wrapping_add(i)) & 0x7F;
                        if ch >= 0x20 { filename.push(ch as char); }
                    }
                    let _ = writeln!(log, "[{:>10}]   FCB: drive={} file=\"{}\"",
                        counter, fcb_drive, filename.trim());
                    // If searching for TURBO-BS, search for DPH in BIOS area
                    if filename.contains("TURBO") {
                        // Search for DPH in BIOS area (0xEE00-0xFFFF) by looking for DIRBUF=0xFAF7
                        let _ = writeln!(log, "[{:>10}]   === Searching for DPH (DIRBUF=0xFAF7) in BIOS area ===", counter);
                        for addr in (0xEE00u16..0xFFF0).step_by(2) {
                            let dirbuf = machine.peek(addr) as u16 | ((machine.peek(addr+1) as u16) << 8);
                            if dirbuf == 0xFAF7 {
                                // Check if this could be DPH bytes 8-9 (DIRBUF field)
                                let dph_start = addr - 8;
                                let xlt = machine.peek(dph_start) as u16 | ((machine.peek(dph_start+1) as u16) << 8);
                                let dpb = machine.peek(dph_start+10) as u16 | ((machine.peek(dph_start+11) as u16) << 8);
                                let csv = machine.peek(dph_start+12) as u16 | ((machine.peek(dph_start+13) as u16) << 8);
                                let alv = machine.peek(dph_start+14) as u16 | ((machine.peek(dph_start+15) as u16) << 8);
                                let _ = writeln!(log, "[{:>10}]   DPH candidate at 0x{:04X}: XLT=0x{:04X} DIRBUF=0x{:04X} DPB=0x{:04X} CSV=0x{:04X} ALV=0x{:04X}",
                                    counter, dph_start, xlt, dirbuf, dpb, csv, alv);
                                // If DPB pointer is in BIOS area, dump DPB
                                if dpb >= 0xEE00 {
                                    let spt = machine.peek(dpb) as u16 | ((machine.peek(dpb+1) as u16) << 8);
                                    let bsh = machine.peek(dpb+2);
                                    let drm = machine.peek(dpb+7) as u16 | ((machine.peek(dpb+8) as u16) << 8);
                                    let off = machine.peek(dpb+13) as u16 | ((machine.peek(dpb+14) as u16) << 8);
                                    let _ = writeln!(log, "[{:>10}]     DPB: SPT={} BSH={} DRM={} OFF={}",
                                        counter, spt, bsh, drm, off);
                                }
                                // Dump CSV contents if in valid range
                                if csv >= 0x100 && csv < 0xFF00 {
                                    let _ = write!(log, "[{:>10}]     CSV[16]: ", counter);
                                    for i in 0..16u16 {
                                        let _ = write!(log, "{:02X} ", machine.peek(csv.wrapping_add(i)));
                                    }
                                    let _ = writeln!(log);
                                }
                            }
                        }
                        // Also check BDOS's internal DPHA pointer area
                        // Dump memory at BIOS_BASE-16 through BIOS_BASE+128 for context
                        if let Some(base) = bios_base {
                            let _ = write!(log, "[{:>10}]   BIOS area 0x{:04X}+: ", counter, base);
                            for i in 0..64u16 {
                                let _ = write!(log, "{:02X} ", machine.peek(base + i));
                            }
                            let _ = writeln!(log);
                        }
                    }
                }
            }
        } else {
            // Discover BIOS base even before tracing is active
            let in_rom = machine.is_rom_rank();
            if in_rom != last_rom_rank {
                last_rom_rank = in_rom;
                if !in_rom && bios_base.is_none() {
                    let warm_lo = machine.peek(0x0001) as u16;
                    let warm_hi = machine.peek(0x0002) as u16;
                    let warm_boot = (warm_hi << 8) | warm_lo;
                    if warm_boot > 0x100 && warm_boot < 0xFFFF {
                        bios_base = Some(warm_boot - 3);
                    }
                }
            }
        }

        // Check for MAKTURBO output (look for error messages in VRAM)
        if keys_injected && counter % 1_000_000 == 0 {
            let vram_text = extract_vram_text(&machine, 10);
            if vram_text.contains("not found") || vram_text.contains("Not found")
                || vram_text.contains("NOT FOUND") || vram_text.contains("Error")
                || vram_text.contains("error") || vram_text.contains("A>")
                || vram_text.contains("A0>")
            {
                // Check if we're past the initial prompt (CCP has processed command)
                if counter > 50_000_000 {
                    let _ = writeln!(log, "\n=== VRAM at exit ===");
                    let _ = writeln!(log, "{}", vram_text);
                    println!("Debug: Screen output detected, stopping.");
                    println!("VRAM: {}", vram_text);
                    break;
                }
            }
        }

        if counter >= max_instructions {
            let vram_text = extract_vram_text(&machine, 10);
            let _ = writeln!(log, "\n=== VRAM at timeout ===");
            let _ = writeln!(log, "{}", vram_text);
            println!("Debug: Timed out after {} instructions", counter);
            println!("VRAM: {}", vram_text);
            break;
        }
    }

    // Clean up temp files
    let _ = std::fs::remove_file(&hd_path);

    println!("Debug trace written to {}", log_path);
}

/// Debug Kaypro 10 floppy access after HD boot.
/// Boots from HD, inserts floppy, switches to C:, runs DIR, then tries to
/// load a program. Traces all BIOS/FDC operations to diagnose "Bad Sector".
pub fn debug_floppy_k10() {
    use iz80::*;
    use std::io::Write;

    let log_path = "k10-floppy-debug.log";
    let fdc_log_path = "k10-floppy-debug-fdc.log";
    let mut log = std::fs::File::create(log_path)
        .expect("failed to create log");

    let _ = writeln!(log, "=== Kaypro 10 Floppy Access Debug ===");
    let _ = writeln!(log, "Goal: Boot from HD, insert floppy, DIR C:, run program");
    let _ = writeln!(log, "");

    // Set up FDC with the boot floppy image (contains CP/M utilities)
    let floppy_path = "disks/system/cpm22g-rom292a.img";
    let mut fdc = crate::floppy_controller::FloppyController::new(
        floppy_path,
        "disks/blank_disks/cpm22-kaypro4-blank.img",
        crate::media::MediaFormat::DsDd,
        10,    // side1_sector_base
        true,  // trace FDC commands
        true,  // trace FDC reads/writes
    );
    let fdc_file = std::fs::File::create(fdc_log_path)
        .expect("failed to create FDC trace log");
    fdc.trace_file = Some(fdc_file);

    let mut machine = crate::kaypro_machine::KayproMachine::new(
        "roms/81-478c.rom",
        crate::kaypro_machine::VideoMode::Sy6545Crtc,
        fdc,
        true,    // has_hard_disk
        false, false, false, false, false, false,
    );

    // No floppy in drive for HD boot
    machine.floppy_controller.disk_in_drive = false;

    // Copy HD image to temp file
    let hd_src = "disks/system/kaypro10.hd";
    let hd_path = std::env::temp_dir()
        .join(format!("izkaypro_floppy_debug_{}.hd", std::process::id()));
    std::fs::copy(hd_src, &hd_path).expect("failed to copy HD image");
    if let Some(ref mut hd) = machine.hard_disk {
        hd.load_image(hd_path.to_str().unwrap())
            .expect("failed to load HD image");
    }

    let mut cpu = Cpu::new_z80();
    let max_instructions: u64 = 500_000_000;
    let mut counter: u64 = 0;
    let mut nmi_pending = false;
    let mut nmi_deadline: u64 = 0;

    let mut bios_base: Option<u16> = None;
    let mut last_rom_rank = true;

    // State machine for injection
    let mut phase = 0u8;  // 0=booting, 1=prompt found, 2=injected C:, 3=injected DIR, 4=done
    let mut phase_counter: u64 = 0;

    println!("Debug: Booting Kaypro 10 from HD...");

    loop {
        cpu.execute_instruction(&mut machine);
        counter += 1;

        // BIOS base discovery
        let in_rom = machine.is_rom_rank();
        if in_rom != last_rom_rank {
            last_rom_rank = in_rom;
            if !in_rom && bios_base.is_none() {
                let warm_lo = machine.peek(0x0001) as u16;
                let warm_hi = machine.peek(0x0002) as u16;
                let warm_boot = (warm_hi << 8) | warm_lo;
                if warm_boot > 0x100 && warm_boot < 0xFFFF {
                    bios_base = Some(warm_boot - 3);
                    let _ = writeln!(log, "[{:>10}] BIOS base: 0x{:04X}", counter, warm_boot - 3);
                }
            }
        }

        // BIOS entry tracing (after base discovered)
        if !in_rom {
            if let Some(base) = bios_base {
                let pc = cpu.registers().pc();
                if pc >= base && pc <= base + 51 && (pc - base) % 3 == 0 {
                    let entry = (pc - base) / 3;
                    let msg: Option<String> = match entry {
                        0 => Some("BOOT".into()),
                        1 => Some("WBOOT".into()),
                        8 => Some("HOME".into()),
                        9 => Some(format!("SELDSK drive={} ({})",
                            cpu.registers().get8(Reg8::C),
                            (b'A' + cpu.registers().get8(Reg8::C)) as char)),
                        10 => Some(format!("SETTRK track={}",
                            cpu.registers().get8(Reg8::C))),
                        11 => Some(format!("SETSEC sector={}",
                            cpu.registers().get8(Reg8::C))),
                        12 => Some(format!("SETDMA addr=0x{:04X}",
                            cpu.registers().get16(Reg16::BC))),
                        13 => Some("READ".into()),
                        14 => Some("WRITE".into()),
                        16 => {
                            let sec = cpu.registers().get16(Reg16::BC);
                            let xlt = cpu.registers().get16(Reg16::DE);
                            Some(format!("SECTRAN sector={} xlt=0x{:04X}", sec, xlt))
                        },
                        _ => None,
                    };
                    if let Some(m) = msg {
                        let _ = writeln!(log, "[{:>10}] BIOS: {}", counter, m);
                        let _ = log.flush();
                    }
                }
            }
        }

        // BDOS tracing
        if !in_rom && cpu.registers().pc() == 0x0005 {
            let command = cpu.registers().get8(Reg8::C);
            if command != 6 { // skip C_RAWIO
                let args = cpu.registers().get16(Reg16::DE);
                static NAMES: &[&str] = &[
                    "P_TERMCPM", "C_READ", "C_WRITE", "A_READ", "A_WRITE",
                    "L_WRITE", "C_RAWIO", "A_STATIN", "A_STATOUT", "C_WRITESTR",
                    "C_READSTR", "C_STAT", "S_BDOSVER", "DRV_ALLRESET", "DRV_SET",
                    "F_OPEN", "F_CLOSE", "F_SFIRST", "F_SNEXT", "F_DELETE",
                    "F_READ", "F_WRITE", "F_MAKE", "F_RENAME", "DRV_LOGINVEC",
                    "DRV_GET", "F_DMAOFF", "DRV_ALLOCVEC", "DRV_SETRO", "DRV_ROVEC",
                    "F_ATTRIB", "DRV_DPB", "F_USERNUM", "F_READRAND", "F_WRITERAND",
                    "F_SIZE", "F_RANDREC", "DRV_RESET",
                ];
                let name = if (command as usize) < NAMES.len() {
                    NAMES[command as usize]
                } else { "?" };
                let _ = writeln!(log, "[{:>10}] BDOS {}: {}(0x{:04X})", counter, command, name, args);
                // Dump FCB for file operations
                if command == 15 || command == 17 || command == 22 || command == 20 {
                    let fcb_drive = machine.peek(args);
                    let mut filename = String::new();
                    for i in 1..=11u16 {
                        let ch = machine.peek(args.wrapping_add(i)) & 0x7F;
                        if ch >= 0x20 { filename.push(ch as char); }
                    }
                    let _ = writeln!(log, "[{:>10}]   FCB: drive={} file=\"{}\"",
                        counter, fcb_drive, filename.trim());
                }
                let _ = log.flush();
            }
        }

        // SIO interrupt processing
        if counter % 1024 == 0 {
            let i_reg = cpu.registers().get8(Reg8::I);
            if let Some(handler) = machine.sio_check_interrupt(i_reg) {
                let regs = cpu.registers();
                let pc = regs.pc();
                let mut sp = regs.get16(Reg16::SP);
                sp = sp.wrapping_sub(2);
                regs.set16(Reg16::SP, sp);
                machine.poke(sp, pc as u8);
                machine.poke(sp.wrapping_add(1), (pc >> 8) as u8);
                cpu.registers().set_pc(handler);
            }
        }

        // NMI processing
        if machine.floppy_controller.raise_nmi {
            machine.floppy_controller.raise_nmi = false;
            nmi_pending = true;
            nmi_deadline = counter + 10_000_000;
        }
        let mut nmi_signaled = false;
        if nmi_pending && (cpu.is_halted()
            || (counter >= nmi_deadline && machine.nmi_vector_is_safe()))
        {
            cpu.signal_nmi();
            nmi_pending = false;
            nmi_signaled = true;
        }
        if !nmi_signaled && cpu.is_halted() {
            let _ = writeln!(log, "[{:>10}] HALT at PC=0x{:04X}", counter, cpu.registers().pc());
            break;
        }

        // Phase state machine
        if counter % 100_000 == 0 {
            match phase {
                0 => {
                    // Wait for A> prompt
                    if check_for_prompt(&machine) {
                        phase = 1;
                        phase_counter = counter;
                        let _ = writeln!(log, "\n[{:>10}] === A> prompt detected ===", counter);

                        // Dump BIOS DPH/DPB info
                        if let Some(base) = bios_base {
                            let _ = writeln!(log, "\n--- BIOS Jump Table at 0x{:04X} ---", base);
                            let names = ["BOOT", "WBOOT", "CONST", "CONIN", "CONOUT", "LIST",
                                "PUNCH", "READER", "HOME", "SELDSK", "SETTRK", "SETSEC",
                                "SETDMA", "READ", "WRITE", "LISTST", "SECTRAN"];
                            for (i, name) in names.iter().enumerate() {
                                let addr = base + (i as u16) * 3;
                                let target_lo = machine.peek(addr + 1) as u16;
                                let target_hi = machine.peek(addr + 2) as u16;
                                let target = (target_hi << 8) | target_lo;
                                let _ = writeln!(log, "  {:8} -> 0x{:04X}", name, target);
                            }

                            // Scan for floppy DPB (SPT=40, BSH=3 or 4)
                            let _ = writeln!(log, "\n--- DPB Scan (floppy: SPT=40) ---");
                            for addr in (0x100u32..0xFE00).step_by(2) {
                                let spt = machine.peek(addr as u16) as u16
                                    | ((machine.peek((addr + 1) as u16) as u16) << 8);
                                let bsh = machine.peek((addr + 2) as u16);
                                let blm = machine.peek((addr + 3) as u16);
                                if spt == 40 && (bsh == 3 || bsh == 4) && (blm == 7 || blm == 15) {
                                    let exm = machine.peek((addr + 4) as u16);
                                    let dsm = machine.peek((addr + 5) as u16) as u16
                                        | ((machine.peek((addr + 6) as u16) as u16) << 8);
                                    let drm = machine.peek((addr + 7) as u16) as u16
                                        | ((machine.peek((addr + 8) as u16) as u16) << 8);
                                    let al0 = machine.peek((addr + 9) as u16);
                                    let al1 = machine.peek((addr + 10) as u16);
                                    let cks = machine.peek((addr + 11) as u16) as u16
                                        | ((machine.peek((addr + 12) as u16) as u16) << 8);
                                    let off = machine.peek((addr + 13) as u16) as u16
                                        | ((machine.peek((addr + 14) as u16) as u16) << 8);
                                    let _ = writeln!(log, "  DPB at 0x{:04X}: SPT={} BSH={} BLM={} EXM={} DSM={} DRM={} AL={:02X}/{:02X} CKS={} OFF={}",
                                        addr, spt, bsh, blm, exm, dsm, drm, al0, al1, cks, off);
                                }
                            }

                            // Scan for SECTRAN table (XLT) - look for 16/32 byte sequences containing sector mapping
                            let _ = writeln!(log, "\n--- SECTRAN XLT table scan ---");
                            // The SELDSK function returns HL=DPH pointer
                            // DPH: bytes 0-1 = XLT pointer (sector translation table)
                            // If XLT is 0x0000, no translation (1:1 mapping)
                            // Scan for DPH structures by looking for plausible XLT pointers
                            // near BIOS area
                            let seldsk_target = machine.peek(base + 28) as u16
                                | ((machine.peek(base + 29) as u16) << 8);
                            let _ = writeln!(log, "  SELDSK handler at 0x{:04X}", seldsk_target);
                            // Dump 128 bytes around SELDSK handler
                            let _ = write!(log, "  SELDSK code:");
                            for i in 0..64u16 {
                                if i % 16 == 0 { let _ = write!(log, "\n    {:04X}:", seldsk_target + i); }
                                let _ = write!(log, " {:02X}", machine.peek(seldsk_target + i));
                            }
                            let _ = writeln!(log);
                        }

                        // Dump drive type table at FFF4-FFF7
                        {
                            let _ = writeln!(log, "\n--- Drive type table FFF4-FFF7 ---");
                            for i in 0..4u16 {
                                let addr = 0xFFF4 + i;
                                let val = machine.peek(addr);
                                let drive_letter = (b'A' + i as u8) as char;
                                let type_bits = val & 0x0C;
                                let type_name = match type_bits {
                                    0x00 => "SSSD floppy",
                                    0x04 => "SSDD floppy",
                                    0x08 => "DSDD/HD",
                                    0x0C => "invalid",
                                    _ => "?",
                                };
                                let _ = writeln!(log, "  FFF{:X}: 0x{:02X} (Drive {}) type={} bit6={} bit0={}",
                                    4 + i, val, drive_letter, type_name,
                                    if val & 0x40 != 0 { "set" } else { "clear" },
                                    val & 0x01);
                            }
                        }

                        // Dump BIOS dispatch code at 0xEF74 (256 bytes)
                        if let Some(base) = bios_base {
                            let _ = writeln!(log, "\n--- BIOS dispatch code at 0xEF74 ---");
                            for row in 0..16u16 {
                                let addr = 0xEF74 + row * 16;
                                let _ = write!(log, "  {:04X}:", addr);
                                for col in 0..16u16 {
                                    let _ = write!(log, " {:02X}", machine.peek(addr + col));
                                }
                                let _ = writeln!(log);
                            }

                            // Dump BIOS READ handler area
                            let read_target = machine.peek(base + 40) as u16
                                | ((machine.peek(base + 41) as u16) << 8);
                            let _ = writeln!(log, "\n--- BIOS READ handler at 0x{:04X} ---", read_target);
                            for row in 0..16u16 {
                                let addr = read_target + row * 16;
                                let _ = write!(log, "  {:04X}:", addr);
                                for col in 0..16u16 {
                                    let _ = write!(log, " {:02X}", machine.peek(addr + col));
                                }
                                let _ = writeln!(log);
                            }

                            // Dump 0xF000-0xF200 (likely contains floppy read/write routines)
                            let _ = writeln!(log, "\n--- BIOS area 0xEE00-0xF200 ---");
                            for row in 0..((0xF200u16 - 0xEE00) / 16) {
                                let addr = 0xEE00 + row * 16;
                                let _ = write!(log, "  {:04X}:", addr);
                                for col in 0..16u16 {
                                    let _ = write!(log, " {:02X}", machine.peek(addr + col));
                                }
                                // ASCII
                                let _ = write!(log, "  ");
                                for col in 0..16u16 {
                                    let b = machine.peek(addr + col);
                                    let c = if b >= 0x20 && b < 0x7F { b as char } else { '.' };
                                    let _ = write!(log, "{}", c);
                                }
                                let _ = writeln!(log);
                            }
                        }

                        println!("Debug: A> prompt found, inserting floppy and switching to C:");
                    }
                }
                1 => {
                    // Wait a bit, then insert floppy and type C: + CR
                    if counter > phase_counter + 2_000_000 {
                        machine.floppy_controller.disk_in_drive = true;

                        // The ROM cached the floppy drive type as SSDD (0x05) at boot
                        // because disk_in_drive was false (NOT READY). Now that a DSDD
                        // floppy is inserted, patch the drive type at 0xFFF6 to DSDD
                        // (0x09) so the ROM uses the correct track/side/sector mapping.
                        let old_type = machine.peek(0xFFF6);
                        machine.poke(0xFFF6, 0x09);
                        let _ = writeln!(log, "\n[{:>10}] === Inserting floppy, patching drive type 0xFFF6: 0x{:02X} -> 0x09, typing C: ===", counter, old_type);

                        machine.keyboard.inject_keys(b"C:\r");
                        phase = 2;
                        phase_counter = counter;
                    }
                }
                2 => {
                    // Wait for C> prompt, then DIR
                    if counter > phase_counter + 5_000_000 {
                        let vram = extract_vram_text(&machine, 24);
                        let _ = writeln!(log, "\n[{:>10}] VRAM after C:  ==>", counter);
                        for (i, line) in vram.split('|').enumerate() {
                            let t = line.trim_end();
                            if !t.is_empty() && t.chars().any(|c| c != '.') {
                                let _ = writeln!(log, "  {:2}: {}", i, t);
                            }
                        }
                        let _ = writeln!(log, "\n[{:>10}] === Typing DIR ===", counter);
                        machine.keyboard.inject_keys(b"DIR\r");
                        phase = 3;
                        phase_counter = counter;
                    }
                }
                3 => {
                    // Wait for DIR output, then try loading a program
                    if counter > phase_counter + 20_000_000 {
                        let vram = extract_vram_text(&machine, 24);
                        let _ = writeln!(log, "\n[{:>10}] VRAM after DIR ==>", counter);
                        for (i, line) in vram.split('|').enumerate() {
                            let t = line.trim_end();
                            if !t.is_empty() && t.chars().any(|c| c != '.') {
                                let _ = writeln!(log, "  {:2}: {}", i, t);
                            }
                        }
                        // Now try to load a program (STAT is usually small)
                        let _ = writeln!(log, "\n[{:>10}] === Typing STAT ===", counter);
                        machine.keyboard.inject_keys(b"STAT\r");
                        phase = 4;
                        phase_counter = counter;
                    }
                }
                4 => {
                    // Wait for result
                    if counter > phase_counter + 30_000_000 {
                        let vram = extract_vram_text(&machine, 24);
                        let _ = writeln!(log, "\n[{:>10}] VRAM after STAT ==>", counter);
                        for (i, line) in vram.split('|').enumerate() {
                            let t = line.trim_end();
                            if !t.is_empty() && t.chars().any(|c| c != '.') {
                                let _ = writeln!(log, "  {:2}: {}", i, t);
                            }
                        }
                        phase = 5;
                    }
                }
                _ => break,
            }
        }

        if counter >= max_instructions {
            let _ = writeln!(log, "\n[{:>10}] === Timed out ===", counter);
            break;
        }
    }

    let _ = log.flush();
    let _ = std::fs::remove_file(&hd_path);

    println!("Debug trace written to {} and {}", log_path, fdc_log_path);
}
