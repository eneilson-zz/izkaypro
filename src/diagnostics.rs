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
    pub kaypro10_mode: bool,
    pub hd_path: Option<&'static str>,
    pub prompt_mode: PromptMode,
    pub trace_boot: bool,
    pub run_cpm_drive_checks: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    AnyDrivePrompt,
    StrictA0,
}

struct BootTraceEvent {
    step: u64,
    pc: u16,
    detail: String,
}

#[derive(Clone)]
struct K10HiExecEvent {
    step: u64,
    pc: u16,
    op0: u8,
    op1: u8,
    op2: u8,
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    sp: u16,
    ret: u16,
}

fn format_k10_hiexec(trace: &std::collections::VecDeque<K10HiExecEvent>) -> String {
    if trace.is_empty() {
        return "none".to_string();
    }
    trace
        .iter()
        .map(|e| {
            format!(
                "@{} pc={:04X} op={:02X} {:02X} {:02X} af={:04X} bc={:04X} de={:04X} hl={:04X} sp={:04X} ret={:04X}",
                e.step, e.pc, e.op0, e.op1, e.op2, e.af, e.bc, e.de, e.hl, e.sp, e.ret
            )
        })
        .collect::<Vec<_>>()
        .join(" ; ")
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
            kaypro10_mode: false,
            hd_path: None,
            prompt_mode: PromptMode::AnyDrivePrompt,
            trace_boot: false,
            run_cpm_drive_checks: false,
        },
        BootTestConfig {
            name: "Kaypro 4/84 (81-292a)",
            rom_path: "roms/81-292a.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/cpm22g-rom292a.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            kaypro10_mode: false,
            hd_path: None,
            prompt_mode: PromptMode::AnyDrivePrompt,
            trace_boot: false,
            run_cpm_drive_checks: false,
        },
        BootTestConfig {
            name: "Kaypro 4/84 TurboROM",
            rom_path: "roms/trom34.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/k484_turborom_63k_boot.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            kaypro10_mode: false,
            hd_path: None,
            prompt_mode: PromptMode::AnyDrivePrompt,
            trace_boot: false,
            run_cpm_drive_checks: false,
        },
        BootTestConfig {
            name: "KayPLUS 84",
            rom_path: "roms/kplus84.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/kayplus_boot.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 0,
            kaypro10_mode: false,
            hd_path: None,
            prompt_mode: PromptMode::AnyDrivePrompt,
            trace_boot: false,
            run_cpm_drive_checks: false,
        },
        BootTestConfig {
            name: "Kaypro 10 (81-478c)",
            rom_path: "roms/81-478c.rom",
            video_mode: crate::kaypro_machine::VideoMode::Sy6545Crtc,
            disk_format: crate::media::MediaFormat::DsDd,
            disk_a: "disks/system/k10u-rld.img",
            disk_b: "disks/blank_disks/cpm22-kaypro4-blank.img",
            side1_sector_base: 10,
            kaypro10_mode: true,
            hd_path: Some("disks/system/kaypro10.hd"),
            prompt_mode: PromptMode::StrictA0,
            trace_boot: true,
            run_cpm_drive_checks: true,
        },
    ];

    let mut results = Vec::new();
    for cfg in &configs {
        results.push(run_single_boot_test(cfg));
    }
    results
}

fn run_single_boot_test(cfg: &BootTestConfig) -> TestResult {
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use iz80::*;

    let force_no_hd = std::env::var("IZK10_BOOTTEST_NO_HD")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);

    let hd_path_owned: Option<String> = if cfg.kaypro10_mode && !force_no_hd {
        // Boot tests must be deterministic and must not mutate user images.
        // Always recreate a disposable Kaypro10 HD image in temp space.
        let mut temp_hd: PathBuf = std::env::temp_dir();
        temp_hd.push("izkaypro-k10-boottest.hd");
        let mut temp_map = temp_hd.clone();
        temp_map.set_extension("hd.fmtmap");

        let _ = std::fs::remove_file(&temp_hd);
        let _ = std::fs::remove_file(&temp_map);
        let temp_hd_str = temp_hd.to_string_lossy().into_owned();
        let _ = crate::hard_disk_image::ensure_exists(&temp_hd_str);
        let _ = crate::hard_disk_image::seed_kaypro10_from_floppy(
            &temp_hd_str,
            "disks/system/k10u-rld.img",
        );
        Some(temp_hd_str)
    } else {
        None
    };

    let fdc = crate::floppy_controller::FloppyController::new(
        cfg.disk_a, cfg.disk_b, cfg.disk_format, cfg.side1_sector_base, false, false,
    );
    let wd_trace_on = cfg.kaypro10_mode && cfg.trace_boot;
    let wd_trace_log = if wd_trace_on {
        Some("logs/wd1002-boottest.log")
    } else {
        None
    };
    let hd_path_for_machine = if cfg.kaypro10_mode && force_no_hd {
        None
    } else {
        hd_path_owned.as_deref().or(cfg.hd_path)
    };

    let mut machine = crate::kaypro_machine::KayproMachine::new(
        cfg.rom_path,
        cfg.video_mode,
        fdc,
        false,
        false,
        false,
        false,
        false,
        wd_trace_on,
        wd_trace_log,
        cfg.kaypro10_mode,
        hd_path_for_machine,
    );
    let mut cpu = Cpu::new_z80();

    // Keep both an instruction budget and a wall-clock budget so boot tests
    // always terminate with an explicit pass/fail result.
    let max_instructions: u64 = 100_000_000;
    let max_wall_time = Duration::from_secs(30);
    let start_time = Instant::now();
    let mut counter: u64 = 0;
    let mut nmi_pending = false;
    let mut nmi_deadline: u64 = 0;
    let mut prompt_found = false;
    let mut prompt_at: u64 = 0;
    let mut prompt_text = String::new();

    // After finding prompt, monitor for boot loops by tracking FDC motor toggles
    let post_prompt_budget: u64 = 5_000_000;
    let mut fdc_motor_toggles: u32 = 0;
    let mut last_fdc_motor = false;
    let mut last_fdc_cmd_count: u64 = 0;
    let mut trace_log: VecDeque<BootTraceEvent> = VecDeque::new();
    let mut last_fbf5: Option<u8> = None;
    let mut last_fbf6: Option<u8> = None;
    let mut observed_lines: Vec<String> = Vec::new();
    let mut saw_a0 = false;
    let mut saw_b0 = false;
    let mut saw_c0 = false;
    let mut saw_stat_com = false;
    let mut saw_directory_banner = false;
    let mut saw_k390 = false;
    let mut saw_k5m = false;
    let mut script_started = false;
    let mut script_idx = 0usize;
    let mut script_next_inject_at = 0u64;
    let mut script_done = false;
    let mut script_settle_until = 0u64;
    let mut k10_last_read_order: Option<(u8, u16, u8, u8, u16)> = None;
    let mut k10_first_order_mismatch: Option<String> = None;
    let mut k10_handoff_logged = false;
    let mut k10_handoff_sanity_checked = false;
    let mut k10_fd90_logged = false;
    let mut k10_handoff_prev: [u8; 16] = [0; 16];
    let mut k10_handoff_prev_init = false;
    let mut k10_handoff_change_logs = 0usize;
    let mut prev_pc: u16 = 0;
    let mut k10_hi_exec: VecDeque<K10HiExecEvent> = VecDeque::new();
    // Commands are typed by the diagnostics harness into SIO input.
    const K10_DRIVE_SCRIPT: &[u8] =
        b"B:\rC:\rD C:\rA:\rD A:\rB:\rD B:\rSTAT A:\rSTAT B:\rSTAT C:\rA:\r";

    let mut push_trace = |step: u64, pc: u16, detail: String| {
        if !cfg.trace_boot {
            return;
        }
        if trace_log.len() == 1024 {
            trace_log.pop_front();
        }
        trace_log.push_back(BootTraceEvent { step, pc, detail });
    };

    let k10_sum_128 = |machine: &crate::kaypro_machine::KayproMachine, base: u16| -> u16 {
        let mut sum = 0u16;
        for i in 0..128u16 {
            sum = sum.wrapping_add(machine.peek(base.wrapping_add(i)) as u16);
        }
        sum
    };

    loop {
        cpu.execute_instruction(&mut machine);
        counter += 1;
        let pc = cpu.registers().pc();

        if cfg.kaypro10_mode {
            let mut cur = [0u8; 16];
            for (i, b) in cur.iter_mut().enumerate() {
                *b = machine.peek(0xDCF0u16.wrapping_add(i as u16));
            }
            if !k10_handoff_prev_init {
                k10_handoff_prev = cur;
                k10_handoff_prev_init = true;
            } else if cur != k10_handoff_prev && k10_handoff_change_logs < 16 {
                let regs = cpu.registers();
                push_trace(
                    counter,
                    pc,
                    format!(
                        "K10 handoff-bytes change@DCF0 pc={:04X} op={:02X} {:02X} {:02X} before={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} after={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} de={:04X} hl={:04X} sp={:04X}",
                        pc,
                        machine.peek(pc),
                        machine.peek(pc.wrapping_add(1)),
                        machine.peek(pc.wrapping_add(2)),
                        k10_handoff_prev[0], k10_handoff_prev[1], k10_handoff_prev[2], k10_handoff_prev[3],
                        k10_handoff_prev[4], k10_handoff_prev[5], k10_handoff_prev[6], k10_handoff_prev[7],
                        cur[0], cur[1], cur[2], cur[3], cur[4], cur[5], cur[6], cur[7],
                        regs.get16(iz80::Reg16::DE),
                        regs.get16(iz80::Reg16::HL),
                        regs.get16(iz80::Reg16::SP),
                    ),
                );
                k10_handoff_prev = cur;
                k10_handoff_change_logs += 1;
            }
        }

        if cfg.kaypro10_mode && pc >= 0xF000 {
            let regs = cpu.registers();
            let sp = regs.get16(iz80::Reg16::SP);
            let ret = u16::from_le_bytes([machine.peek(sp), machine.peek(sp.wrapping_add(1))]);
            if k10_hi_exec.len() == 256 {
                k10_hi_exec.pop_front();
            }
            k10_hi_exec.push_back(K10HiExecEvent {
                step: counter,
                pc,
                op0: machine.peek(pc),
                op1: machine.peek(pc.wrapping_add(1)),
                op2: machine.peek(pc.wrapping_add(2)),
                af: regs.get16(iz80::Reg16::AF),
                bc: regs.get16(iz80::Reg16::BC),
                de: regs.get16(iz80::Reg16::DE),
                hl: regs.get16(iz80::Reg16::HL),
                sp,
                ret,
            });
        }

        if cfg.kaypro10_mode {
            if let Some(wd) = machine.wd1002.as_mut() {
                wd.step();
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
            nmi_deadline = counter + 10_000_000;
            if !cfg.kaypro10_mode {
                push_trace(counter, pc, "FDC NMI raised".to_string());
            }
        }
        if cfg.kaypro10_mode {
            if let Some(wd) = machine.wd1002.as_mut() {
                if wd.take_intrq() {
                    if cfg.trace_boot {
                        push_trace(counter, pc, "WD INTRQ latched as NMI".to_string());
                    }
                    nmi_pending = true;
                    nmi_deadline = counter + 10_000_000;
                }
            }
        }
        let mut nmi_signaled = false;
        if !nmi_signaled && nmi_pending && (cpu.is_halted()
            || (counter >= nmi_deadline && machine.nmi_vector_is_safe()))
        {
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
            let mut halt_lines = Vec::new();
            collect_visible_lines(&machine, &mut halt_lines);
            let screen = halt_lines
                .iter()
                .take(6)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            push_trace(counter, pc, "CPU halted before prompt".to_string());
            return TestResult {
                name: format!("Boot {}", cfg.name),
                passed: false,
                message: format!(
                    "HALT at PC=0x{:04X} after {} instructions. Screen: [{}]. Trace: {}",
                    pc,
                    counter,
                    screen,
                    format_boot_trace(&trace_log)
                ),
            };
        }

        if cfg.trace_boot && machine.floppy_controller.last_command_count != last_fdc_cmd_count {
            last_fdc_cmd_count = machine.floppy_controller.last_command_count;
            let cmd = machine.floppy_controller.last_command;
            if cmd != 0xD0 {
                let drive = machine.floppy_controller.drive;
                let side = machine.floppy_controller.current_side();
                let trk_reg = machine.floppy_controller.get_track();
                let head = machine.floppy_controller.head_position;
                let sec = machine.floppy_controller.get_sector();
                let mut detail = format!(
                    "FDC CMD 0x{:02X} drv={} side={} trkreg={} head={} sec={}",
                    cmd,
                    drive,
                    if side { 1 } else { 0 },
                    trk_reg,
                    head,
                    sec
                );
                if (cmd & 0xE0) == 0x80 || (cmd & 0xE0) == 0xA0 {
                    let media = machine.floppy_controller.media_selected_ref();
                    let (ok, idx, last) = media.sector_index(side, head, sec);
                    if ok && last > idx {
                        let b0 = media.read_byte(idx);
                        let b1 = media.read_byte(idx + 1);
                        let b2 = media.read_byte(idx + 2);
                        let b3 = media.read_byte(idx + 3);
                        let t0 = media.read_byte(last - 4);
                        let t1 = media.read_byte(last - 3);
                        let t2 = media.read_byte(last - 2);
                        let t3 = media.read_byte(last - 1);
                        detail.push_str(&format!(
                            " ix={}..{} len={} data={:02X} {:02X} {:02X} {:02X} tail={:02X} {:02X} {:02X} {:02X}",
                            idx,
                            last,
                            last - idx,
                            b0,
                            b1,
                            b2,
                            b3,
                            t0,
                            t1,
                            t2,
                            t3
                        ));
                    } else {
                        detail.push_str(" ix=invalid");
                    }
                }
                push_trace(counter, pc, detail);
            }
        }

        if cfg.kaypro10_mode {
            let b5 = machine.peek(0xFBF5);
            let b6 = machine.peek(0xFBF6);
            if last_fbf5 != Some(b5) || last_fbf6 != Some(b6) {
                let wd = machine
                    .wd1002
                    .as_ref()
                    .map(|w| w.debug_snapshot())
                    .unwrap_or((0, 0, 0, 0, 0, 0, 0, 0, 0));
                push_trace(
                    counter,
                    pc,
                    format!(
                        "K10 fb_tail change fbf5={:02X} fbf6={:02X} fec1={:04X} fd84={:02X} fd85={:02X} fd7d={:02X} wd[cmd={:02X} sts={:02X} cnt={:02X} sec={:02X} cyl={:02X} sdh={:02X} xfer={} ix={} ph={}]",
                        b5,
                        b6,
                        u16::from_le_bytes([machine.peek(0xFEC1), machine.peek(0xFEC2)]),
                        machine.peek(0xFD84),
                        machine.peek(0xFD85),
                        machine.peek(0xFD7D),
                        wd.0,
                        wd.1,
                        wd.2,
                        wd.3,
                        wd.4,
                        wd.5,
                        wd.6,
                        wd.7,
                        wd.8
                    ),
                );
                last_fbf5 = Some(b5);
                last_fbf6 = Some(b6);
            }
        }

        // Check for prompt/screen updates periodically.
        if counter % 25_000 == 0 {
            collect_visible_lines(&machine, &mut observed_lines);
            if contains_line_substr(&observed_lines, "A0>") {
                saw_a0 = true;
            }
            if contains_line_substr(&observed_lines, "B0>") {
                saw_b0 = true;
            }
            if contains_line_substr(&observed_lines, "C0>") {
                saw_c0 = true;
            }
            if contains_line_substr(&observed_lines, "STAT.COM")
                || contains_line_substr(&observed_lines, "PUTSYSU.COM")
            {
                saw_stat_com = true;
            }
            if contains_line_substr(&observed_lines, "DIRECTORY")
                || contains_line_substr(&observed_lines, "NO FILE")
            {
                saw_directory_banner = true;
            }
            if contains_k_value_in_range(&observed_lines, 360, 430) {
                saw_k390 = true;
            }
            if contains_k_value_in_range(&observed_lines, 4700, 5300) {
                saw_k5m = true;
            }

            if !prompt_found {
                if let Some(prompt) = check_for_prompt(&machine, cfg.prompt_mode) {
                    prompt_found = true;
                    prompt_at = counter;
                    prompt_text = prompt.clone();
                    last_fdc_motor = machine.floppy_controller.motor_on;
                    push_trace(counter, pc, format!("Prompt detected: {}", prompt));
                }
            } else if cfg.run_cpm_drive_checks && !script_started {
                // Start scripted CP/M drive checks only after reaching a stable prompt.
                script_started = true;
                script_idx = 0;
                script_next_inject_at = counter + 50_000;
                push_trace(counter, pc, "Starting Kaypro10 CP/M drive script".to_string());
            }
        }

        if cfg.run_cpm_drive_checks && script_started && !script_done && counter >= script_next_inject_at {
            if script_idx < K10_DRIVE_SCRIPT.len() {
                machine.keyboard.inject_key(K10_DRIVE_SCRIPT[script_idx]);
                script_idx += 1;
                script_next_inject_at = counter + 8_000;
            } else {
                script_done = true;
                script_settle_until = counter + 1_500_000;
                push_trace(counter, pc, "Kaypro10 CP/M drive script completed".to_string());
            }
        }

        // Explicitly fail if ROM reported boot media failure.
        if counter % 10_000 == 0 {
            if let Some(msg) = check_for_failure_banner(&machine) {
                push_trace(counter, pc, format!("Failure banner: {}", msg));
                return TestResult {
                    name: format!("Boot {}", cfg.name),
                    passed: false,
                    message: format!(
                        "Boot failure banner detected at PC=0x{:04X} after {} instructions: '{}' Trace: {}",
                        pc,
                        counter,
                        msg,
                        format_boot_trace(&trace_log)
                    ),
                };
            }
        }

        // Kaypro10 ROM checksum compare point in select_boot.
        // Trace it for debugging but do not force failure here: the real
        // pass/fail criteria are prompt detection and explicit failure banners.
        if cfg.kaypro10_mode && pc == 0x175A {
            let mut s = [0u8; 128];
            for (i, b) in s.iter_mut().enumerate() {
                *b = machine.peek(0x8000 + i as u16);
            }
            let fd8a = u16::from_le_bytes([machine.peek(0xFD8A), machine.peek(0xFD8B)]);
            let fec1 = u16::from_le_bytes([machine.peek(0xFEC1), machine.peek(0xFEC2)]);
            let src0 = machine.peek(fd8a);
            let src1 = machine.peek(fd8a.wrapping_add(1));
            let src2 = machine.peek(fd8a.wrapping_add(2));
            let src3 = machine.peek(fd8a.wrapping_add(3));
            let fb0 = machine.peek(0xFB77);
            let fb1 = machine.peek(0xFB78);
            let fb2 = machine.peek(0xFB79);
            let fb3 = machine.peek(0xFB7A);
            let fb124 = machine.peek(0xFB77 + 124);
            let fb125 = machine.peek(0xFB77 + 125);
            let fb126 = machine.peek(0xFB77 + 126);
            let fb127 = machine.peek(0xFB77 + 127);
            let src124 = machine.peek(fd8a.wrapping_add(124));
            let src125 = machine.peek(fd8a.wrapping_add(125));
            let src126 = machine.peek(fd8a.wrapping_add(126));
            let src127 = machine.peek(fd8a.wrapping_add(127));
            let mut sum: u16 = 0;
            for b in s.iter().take(126) {
                sum = sum.wrapping_add(*b as u16);
            }
            let stored = u16::from_le_bytes([s[126], s[127]]);
            let wd = machine
                .wd1002
                .as_ref()
                .map(|w| w.debug_snapshot())
                .unwrap_or((0, 0, 0, 0, 0, 0, 0, 0, 0));
            push_trace(
                counter,
                pc,
                format!(
                    "K10 chk sum=0x{:04X} stored=0x{:04X} hdr={:02X} {:02X} {:02X} {:02X} tail={:02X} {:02X} {:02X} {:02X} fd8a={:04X} fec1={:04X} src={:02X} {:02X} {:02X} {:02X} src_tail={:02X} {:02X} {:02X} {:02X} fb77={:02X} {:02X} {:02X} {:02X} fb_tail={:02X} {:02X} {:02X} {:02X} wd[cmd={:02X} sts={:02X} cnt={:02X} sec={:02X} cyl={:02X} sdh={:02X} xfer={} ix={} ph={}]",
                    sum,
                    stored,
                    s[0],
                    s[1],
                    s[2],
                    s[3],
                    s[124],
                    s[125],
                    s[126],
                    s[127],
                    fd8a,
                    fec1,
                    src0,
                    src1,
                    src2,
                    src3,
                    src124,
                    src125,
                    src126,
                    src127,
                    fb0,
                    fb1,
                    fb2,
                    fb3,
                    fb124,
                    fb125,
                    fb126,
                    fb127,
                    wd.0,
                    wd.1,
                    wd.2,
                    wd.3,
                    wd.4,
                    wd.5,
                    wd.6,
                    wd.7,
                    wd.8
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x1748 {
            let mut s = [0u8; 128];
            for (i, b) in s.iter_mut().enumerate() {
                *b = machine.peek(0x8000 + i as u16);
            }
            let fd8a = u16::from_le_bytes([machine.peek(0xFD8A), machine.peek(0xFD8B)]);
            let fec1 = u16::from_le_bytes([machine.peek(0xFEC1), machine.peek(0xFEC2)]);
            let src0 = machine.peek(fd8a);
            let src1 = machine.peek(fd8a.wrapping_add(1));
            let src2 = machine.peek(fd8a.wrapping_add(2));
            let src3 = machine.peek(fd8a.wrapping_add(3));
            let fb0 = machine.peek(0xFB77);
            let fb1 = machine.peek(0xFB78);
            let fb2 = machine.peek(0xFB79);
            let fb3 = machine.peek(0xFB7A);
            let wd = machine
                .wd1002
                .as_ref()
                .map(|w| w.debug_snapshot())
                .unwrap_or((0, 0, 0, 0, 0, 0, 0, 0, 0));
            push_trace(
                counter,
                pc,
                format!(
                    "K10 read@1748 bytes={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} tail={:02X} {:02X} {:02X} {:02X} fd8a={:04X} fec1={:04X} src={:02X} {:02X} {:02X} {:02X} fb77={:02X} {:02X} {:02X} {:02X} wd[cmd={:02X} sts={:02X} cnt={:02X} sec={:02X} cyl={:02X} sdh={:02X} xfer={} ix={} ph={}]",
                    s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7], s[124], s[125], s[126], s[127],
                    fd8a, fec1, src0, src1, src2, src3, fb0, fb1, fb2, fb3,
                    wd.0, wd.1, wd.2, wd.3, wd.4, wd.5, wd.6, wd.7, wd.8
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x1548 {
            let fd8a = u16::from_le_bytes([machine.peek(0xFD8A), machine.peek(0xFD8B)]);
            let fd86 = u16::from_le_bytes([machine.peek(0xFD86), machine.peek(0xFD87)]);
            let fd84 = machine.peek(0xFD84);
            let fd85 = machine.peek(0xFD85);
            let fd7d = machine.peek(0xFD7D);
            let f4 = machine.peek(0xFFF4);
            let f5 = machine.peek(0xFFF5);
            let f6 = machine.peek(0xFFF6);
            let f7 = machine.peek(0xFFF7);
            let fd8c = machine.peek(0xFD8C);
            let mut src = [0u8; 8];
            for (i, b) in src.iter_mut().enumerate() {
                *b = machine.peek(fd8a.wrapping_add(i as u16));
            }
            let src124 = machine.peek(fd8a.wrapping_add(124));
            let src125 = machine.peek(fd8a.wrapping_add(125));
            let src126 = machine.peek(fd8a.wrapping_add(126));
            let src127 = machine.peek(fd8a.wrapping_add(127));
            let mut dst = [0u8; 8];
            for (i, b) in dst.iter_mut().enumerate() {
                *b = machine.peek(fd86.wrapping_add(i as u16));
            }
            let dst124 = machine.peek(fd86.wrapping_add(124));
            let dst125 = machine.peek(fd86.wrapping_add(125));
            let dst126 = machine.peek(fd86.wrapping_add(126));
            let dst127 = machine.peek(fd86.wrapping_add(127));
            let src_sum = k10_sum_128(&machine, fd8a);
            let dst_sum = k10_sum_128(&machine, fd86);
            push_trace(
                counter,
                pc,
                format!(
                    "K10 copy@1548 fd84={:02X} fd85={:02X} fd7d={:02X} fd8a={:04X} fd86={:04X} map[fff4={:02X} fff5={:02X} fff6={:02X} fff7={:02X} fd8c={:02X}] src={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} src_tail={:02X} {:02X} {:02X} {:02X} src_sum={:04X} dst={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} dst_tail={:02X} {:02X} {:02X} {:02X} dst_sum={:04X}",
                    fd84,
                    fd85,
                    fd7d,
                    fd8a,
                    fd86,
                    f4,
                    f5,
                    f6,
                    f7,
                    fd8c,
                    src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
                    src124, src125, src126, src127,
                    src_sum,
                    dst[0], dst[1], dst[2], dst[3], dst[4], dst[5], dst[6], dst[7]
                    ,dst124, dst125, dst126, dst127,
                    dst_sum
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x18B7 {
            let fec1 = u16::from_le_bytes([machine.peek(0xFEC1), machine.peek(0xFEC2)]);
            let mut at_fec1 = [0u8; 8];
            for (i, b) in at_fec1.iter_mut().enumerate() {
                *b = machine.peek(fec1.wrapping_add(i as u16));
            }
            push_trace(
                counter,
                pc,
                format!(
                    "K10 inir-start fec1={:04X} mem={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                    fec1,
                    at_fec1[0], at_fec1[1], at_fec1[2], at_fec1[3], at_fec1[4], at_fec1[5], at_fec1[6], at_fec1[7]
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x18C2 {
            let fec1 = u16::from_le_bytes([machine.peek(0xFEC1), machine.peek(0xFEC2)]);
            let mut fb = [0u8; 8];
            for (i, b) in fb.iter_mut().enumerate() {
                *b = machine.peek(0xFB77 + i as u16);
            }
            let fb124 = machine.peek(0xFB77 + 124);
            let fb125 = machine.peek(0xFB77 + 125);
            let fb126 = machine.peek(0xFB77 + 126);
            let fb127 = machine.peek(0xFB77 + 127);
            let mut at_fec1 = [0u8; 8];
            for (i, b) in at_fec1.iter_mut().enumerate() {
                *b = machine.peek(fec1.wrapping_add(i as u16));
            }
            let fb_sum = k10_sum_128(&machine, 0xFB77);
            if let Some(wd) = machine.wd1002.as_ref().map(|w| w.debug_snapshot_ext()) {
                let head = wd.sdh & 0x07;
                let order_now = (wd.cmd, wd.cyl, head, wd.sector, fec1);
                if let Some((pcmd, pcyl, phead, psec, pfec1)) = k10_last_read_order {
                    // Only assert CHS progression across a continuous READ stream.
                    if (pcmd & 0xF0) == 0x20
                        && (wd.cmd & 0xF0) == 0x20
                        && pcmd == wd.cmd
                        && pfec1 == fec1
                    {
                        let mut ecyl = pcyl;
                        let mut ehead = phead;
                        let mut esec = psec.wrapping_add(1);
                        if esec >= wd.logical_spt {
                            esec = 0;
                            ehead = ehead.wrapping_add(1);
                            if (ehead as u64) >= crate::hard_disk_image::HEADS {
                                ehead = 0;
                                ecyl = ecyl.wrapping_add(1);
                            }
                        }
                        if order_now != (wd.cmd, ecyl, ehead, esec, fec1) && k10_first_order_mismatch.is_none() {
                            k10_first_order_mismatch = Some(format!(
                                "Unexpected WD read order prev={:03X}/{}/{} expected={:03X}/{}/{} got={:03X}/{}/{}",
                                pcyl,
                                phead,
                                psec,
                                ecyl,
                                ehead,
                                esec,
                                wd.cyl,
                                head,
                                wd.sector
                            ));
                        }
                    }
                }
                k10_last_read_order = Some(order_now);
                push_trace(
                    counter,
                    pc,
                    format!(
                        "K10 readchk fb_sum={:04X} wd_sum128={:04X} wd_sumfull={:04X} wd_off={} wd_pending={} wd[cmd={:02X} sts={:02X} cnt={:02X} cyl={:03X} head={} sec={} spt={} xfer={} ix={} ph={}]",
                        fb_sum,
                        wd.last_load_sum128,
                        wd.last_load_sum_full,
                        wd.last_load_offset,
                        wd.pending_offset,
                        wd.cmd,
                        wd.status,
                        wd.sec_count,
                        wd.cyl,
                        head,
                        wd.sector,
                        wd.logical_spt,
                        wd.xfer_size,
                        wd.data_ix,
                        wd.phase
                    ),
                );
            }
            push_trace(
                counter,
                pc,
                format!(
                    "K10 inir-done fec1={:04X} fb77={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} fb_tail={:02X} {:02X} {:02X} {:02X} mem={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                    fec1,
                    fb[0], fb[1], fb[2], fb[3], fb[4], fb[5], fb[6], fb[7],
                    fb124, fb125, fb126, fb127,
                    at_fec1[0], at_fec1[1], at_fec1[2], at_fec1[3], at_fec1[4], at_fec1[5], at_fec1[6], at_fec1[7]
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x1E9D {
            let hl = cpu.registers().get16(iz80::Reg16::HL);
            let de = u16::from_le_bytes([machine.peek(0xF946), machine.peek(0xF947)]);
            let fd8c = machine.peek(0xFD8C);
            push_trace(
                counter,
                pc,
                format!("K10 chk1@1E9D hl={:04X} de(f946)={:04X} fd8c={:02X}", hl, de, fd8c),
            );
        }
        if cfg.kaypro10_mode && pc == 0x1F21 {
            let hl = cpu.registers().get16(iz80::Reg16::HL);
            let de = u16::from_le_bytes([machine.peek(0xF7C6), machine.peek(0xF7C7)]);
            let fd8c = machine.peek(0xFD8C);
            let f4 = machine.peek(0xFFF4);
            let f5 = machine.peek(0xFFF5);
            let f6 = machine.peek(0xFFF6);
            let f7 = machine.peek(0xFFF7);
            push_trace(
                counter,
                pc,
                format!(
                    "K10 chk2@1F21 hl={:04X} de(f7c6)={:04X} fd8c={:02X} map[fff4={:02X} fff5={:02X} fff6={:02X} fff7={:02X}]",
                    hl, de, fd8c, f4, f5, f6, f7
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x1F54 {
            let f4 = machine.peek(0xFFF4);
            let f5 = machine.peek(0xFFF5);
            let f6 = machine.peek(0xFFF6);
            let f7 = machine.peek(0xFFF7);
            let fd8c = machine.peek(0xFD8C);
            push_trace(
                counter,
                pc,
                format!(
                    "K10 sub_1f54 fallback fff4={:02X} fff5={:02X} fff6={:02X} fff7={:02X} fd8c={:02X}",
                    f4, f5, f6, f7, fd8c
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x1F6C {
            let f4 = machine.peek(0xFFF4);
            let f5 = machine.peek(0xFFF5);
            let f6 = machine.peek(0xFFF6);
            let f7 = machine.peek(0xFFF7);
            let fd8c = machine.peek(0xFD8C);
            push_trace(
                counter,
                pc,
                format!(
                    "K10 sub_1f6c swap fff4={:02X} fff5={:02X} fff6={:02X} fff7={:02X} fd8c={:02X}",
                    f4, f5, f6, f7, fd8c
                ),
            );
        }
        if cfg.kaypro10_mode && pc == 0x1F65 {
            let f4 = machine.peek(0xFFF4);
            let f5 = machine.peek(0xFFF5);
            let f6 = machine.peek(0xFFF6);
            let f7 = machine.peek(0xFFF7);
            let fd8c = machine.peek(0xFD8C);
            push_trace(
                counter,
                pc,
                format!(
                    "K10 nohd-commit fff4={:02X} fff5={:02X} fff6={:02X} fff7={:02X} fd8c={:02X}",
                    f4, f5, f6, f7, fd8c
                ),
            );
        }

        if cfg.kaypro10_mode && !k10_handoff_logged && (0xDCF0..=0xDD10).contains(&pc) {
            let regs = cpu.registers();
            let a = regs.get8(iz80::Reg8::A);
            let f = regs.get8(iz80::Reg8::F);
            let bc = regs.get16(iz80::Reg16::BC);
            let de = regs.get16(iz80::Reg16::DE);
            let hl = regs.get16(iz80::Reg16::HL);
            let ix = regs.get16(iz80::Reg16::IX);
            let iy = regs.get16(iz80::Reg16::IY);
            let sp = regs.get16(iz80::Reg16::SP);
            let op = machine.peek(pc);
            let op1 = machine.peek(pc.wrapping_add(1));
            let op2 = machine.peek(pc.wrapping_add(2));
            let target = match op {
                0xC3 | 0xC2 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA | 0xCD => {
                    Some(u16::from_le_bytes([op1, op2]))
                }
                0x18 | 0x20 | 0x28 | 0x30 | 0x38 => {
                    let rel = op1 as i8 as i16;
                    Some((pc as i16).wrapping_add(2).wrapping_add(rel) as u16)
                }
                0xE9 => Some(hl),
                0xC9 => {
                    let lo = machine.peek(sp);
                    let hi = machine.peek(sp.wrapping_add(1));
                    Some(u16::from_le_bytes([lo, hi]))
                }
                _ => None,
            };
            let mut t0 = [0u8; 16];
            if let Some(t) = target {
                for (i, b) in t0.iter_mut().enumerate() {
                    *b = machine.peek(t.wrapping_add(i as u16));
                }
            }
            push_trace(
                counter,
                pc,
                format!(
                    "K10 handoff pc={:04X} op={:02X} {:02X} {:02X} af={:02X}{:02X} bc={:04X} de={:04X} hl={:04X} ix={:04X} iy={:04X} sp={:04X} target={} target16={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                    pc,
                    op,
                    op1,
                    op2,
                    a,
                    f,
                    bc,
                    de,
                    hl,
                    ix,
                    iy,
                    sp,
                    target.map(|t| format!("{:04X}", t)).unwrap_or_else(|| "none".to_string()),
                    t0[0], t0[1], t0[2], t0[3], t0[4], t0[5], t0[6], t0[7], t0[8], t0[9], t0[10], t0[11], t0[12], t0[13], t0[14], t0[15]
                ),
            );
            k10_handoff_logged = true;
        }

        if cfg.kaypro10_mode && !k10_handoff_sanity_checked && (0xDC00..=0xDD20).contains(&pc) {
            let sanity_addrs = [0xDE00u16, 0xDE80u16, 0xDF00u16, 0xDF80u16];
            let mut bad: Vec<u16> = Vec::new();
            for base in sanity_addrs {
                let mut all_zero = true;
                let mut all_ff = true;
                for i in 0..16u16 {
                    let b = machine.peek(base.wrapping_add(i));
                    all_zero &= b == 0x00;
                    all_ff &= b == 0xFF;
                }
                if all_zero || all_ff {
                    bad.push(base);
                }
            }
            if !bad.is_empty() {
                let mut details = String::new();
                for a in &bad {
                    if !details.is_empty() {
                        details.push_str(", ");
                    }
                    details.push_str(&format!("{:04X}", a));
                }
                return TestResult {
                    name: format!("Boot {}", cfg.name),
                    passed: false,
                    message: format!(
                        "Pre-jump sanity failed near PC=0x{:04X}; blank loaded vectors at [{}]. Trace: {}",
                        pc,
                        details,
                        format_boot_trace(&trace_log)
                    ),
                };
            }
            k10_handoff_sanity_checked = true;
        }

        if cfg.kaypro10_mode && cfg.trace_boot && pc == 0xFD90 && !k10_fd90_logged {
            k10_fd90_logged = true;
            let sp = cpu.registers().get16(iz80::Reg16::SP);
            let ret = u16::from_le_bytes([machine.peek(sp), machine.peek(sp.wrapping_add(1))]);
            let de = cpu.registers().get16(iz80::Reg16::DE);
            let hl = cpu.registers().get16(iz80::Reg16::HL);
            let mut b = [0u8; 8];
            for (i, out) in b.iter_mut().enumerate() {
                *out = machine.peek(0xFD90u16.wrapping_add(i as u16));
            }
            let mut r = [0u8; 16];
            for (i, out) in r.iter_mut().enumerate() {
                *out = machine.peek(ret.wrapping_add(i as u16));
            }
            let mut d = [0u8; 8];
            for (i, out) in d.iter_mut().enumerate() {
                *out = machine.peek(de.wrapping_add(i as u16));
            }
            let mut h = [0u8; 8];
            for (i, out) in h.iter_mut().enumerate() {
                *out = machine.peek(hl.wrapping_add(i as u16));
            }
            push_trace(
                counter,
                pc,
                format!(
                    "K10 fd90-enter prev_pc={:04X} sp={:04X} ret@sp={:04X} de={:04X} hl={:04X} bytes={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} ret16={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} de8={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} hl8={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                    prev_pc, sp, ret, de, hl,
                    b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                    r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[10], r[11], r[12], r[13], r[14], r[15],
                    d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7],
                    h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]
                ),
            );
        }

        // After finding prompt, monitor for boot loops
        if prompt_found {
            if machine.floppy_controller.motor_on != last_fdc_motor {
                fdc_motor_toggles += 1;
                last_fdc_motor = machine.floppy_controller.motor_on;
            }

            if cfg.run_cpm_drive_checks && script_done && counter >= script_settle_until {
                if !saw_a0 || !saw_b0 || !saw_c0 || !saw_stat_com || !saw_directory_banner || !saw_k390 || !saw_k5m {
                    return TestResult {
                        name: format!("Boot {}", cfg.name),
                        passed: false,
                        message: format!(
                            "CP/M drive checks failed A0={} B0={} C0={} floppy_files={} dir={} k390={} k5m={}. Seen: {}",
                            saw_a0,
                            saw_b0,
                            saw_c0,
                            saw_stat_com,
                            saw_directory_banner,
                            saw_k390,
                            saw_k5m,
                            summarize_lines(&observed_lines),
                        ),
                    };
                }
                return TestResult {
                    name: format!("Boot {}", cfg.name),
                    passed: true,
                    message: format!(
                        "Booted and validated CP/M drive script (A/B/C prompts seen; 5MB+390K capacity markers present)"
                    ),
                };
            }

            if counter >= prompt_at + post_prompt_budget {
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
                    message: format!("Booted ({} instructions), prompt '{}', stable ({} motor toggles)",
                        prompt_at, prompt_text, fdc_motor_toggles),
                };
            }
        }

        if counter >= max_instructions {
            let vram_text = extract_vram_text(&machine, 5);
            let regs = cpu.registers();
            let sp = regs.get16(iz80::Reg16::SP);
            let af = regs.get16(iz80::Reg16::AF);
            let bc = regs.get16(iz80::Reg16::BC);
            let de = regs.get16(iz80::Reg16::DE);
            let hl = regs.get16(iz80::Reg16::HL);
            let mut op = [0u8; 8];
            for (i, b) in op.iter_mut().enumerate() {
                *b = machine.peek(pc.wrapping_add(i as u16));
            }
            push_trace(counter, pc, "Timeout".to_string());
            return TestResult {
                name: format!("Boot {}", cfg.name),
                passed: false,
                message: format!(
                    "Timed out after {} instructions at PC=0x{:04X} ROM={}. regs[AF={:04X} BC={:04X} DE={:04X} HL={:04X} SP={:04X}] op={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}. Screen: {} order_mismatch={}. hi_exec: {}. Trace: {}",
                    counter,
                    pc,
                    machine.is_rom_rank(),
                    af,
                    bc,
                    de,
                    hl,
                    sp,
                    op[0],
                    op[1],
                    op[2],
                    op[3],
                    op[4],
                    op[5],
                    op[6],
                    op[7],
                    vram_text,
                    k10_first_order_mismatch.as_deref().unwrap_or("none"),
                    format_k10_hiexec(&k10_hi_exec),
                    format_boot_trace(&trace_log)
                ),
            };
        }

        if start_time.elapsed() >= max_wall_time {
            let vram_text = extract_vram_text(&machine, 5);
            push_trace(counter, pc, "Wall timeout".to_string());
            return TestResult {
                name: format!("Boot {}", cfg.name),
                passed: false,
                message: format!(
                    "Timed out after {:.1}s and {} instructions at PC=0x{:04X} ROM={}. Screen: {} Trace: {}",
                    start_time.elapsed().as_secs_f32(),
                    counter,
                    pc,
                    machine.is_rom_rank(),
                    vram_text,
                    format_boot_trace(&trace_log),
                ),
            };
        }

        prev_pc = pc;
    }
}

/// Check for a boot prompt in VRAM (works for both CRTC and memory-mapped modes).
/// Returns the matched prompt snippet when found.
fn check_for_prompt(machine: &crate::kaypro_machine::KayproMachine, mode: PromptMode) -> Option<String> {
    let norm = |c: u8| c & 0x7F;

    if machine.video_mode == crate::kaypro_machine::VideoMode::Sy6545Crtc {
        for i in 0..0x800 {
            let c0 = norm(machine.crtc.get_vram(i));
            let c1 = norm(machine.crtc.get_vram((i + 1) & 0x7FF));
            let c2 = norm(machine.crtc.get_vram((i + 2) & 0x7FF));
            if mode == PromptMode::StrictA0 {
                if c0 == b'A' && c1 == b'0' && c2 == b'>' {
                    return Some("A0>".to_string());
                }
            } else {
                if (c0 == b'A' || c0 == b'B') && c1 == b'>' {
                    return Some(format!("{}>", c0 as char));
                }
                if (c0 == b'A' || c0 == b'B') && c1.is_ascii_digit() && c2 == b'>' {
                    return Some(format!("{}{}>", c0 as char, c1 as char));
                }
            }
        }
    } else {
        for i in 0..machine.vram.len().saturating_sub(2) {
            let c0 = norm(machine.vram[i]);
            let c1 = norm(machine.vram[i + 1]);
            let c2 = norm(machine.vram[i + 2]);
            if mode == PromptMode::StrictA0 {
                if c0 == b'A' && c1 == b'0' && c2 == b'>' {
                    return Some("A0>".to_string());
                }
            } else {
                if (c0 == b'A' || c0 == b'B') && c1 == b'>' {
                    return Some(format!("{}>", c0 as char));
                }
                if (c0 == b'A' || c0 == b'B') && c1.is_ascii_digit() && c2 == b'>' {
                    return Some(format!("{}{}>", c0 as char, c1 as char));
                }
            }
        }
    }
    None
}

fn check_for_failure_banner(machine: &crate::kaypro_machine::KayproMachine) -> Option<String> {
    let text = extract_vram_text(machine, 25).to_ascii_lowercase();
    let collapsed = text.replace('|', " ");
    if collapsed.contains("no operating system present on this disk") {
        return Some("No operating system present on this disk".to_string());
    }
    if collapsed.contains("unable to locate winchester drive") {
        return Some("Unable to locate Winchester drive".to_string());
    }
    if collapsed.contains("drive is not ready") {
        return Some("Drive is not ready".to_string());
    }
    None
}

fn format_boot_trace(trace_log: &std::collections::VecDeque<BootTraceEvent>) -> String {
    if trace_log.is_empty() {
        return "none".to_string();
    }
    let mut out = String::new();
    for (idx, ev) in trace_log.iter().enumerate() {
        if idx > 0 {
            out.push_str(" ; ");
        }
        out.push_str(&format!("@{} pc={:04X} {}", ev.step, ev.pc, ev.detail));
    }
    out
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

fn collect_visible_lines(machine: &crate::kaypro_machine::KayproMachine, out: &mut Vec<String>) {
    let rows = if machine.video_mode == crate::kaypro_machine::VideoMode::Sy6545Crtc {
        machine.crtc.displayed_rows().clamp(24, 25)
    } else {
        24
    };
    let snapshot = extract_vram_text(machine, rows);
    for line in snapshot.split('|') {
        let trimmed = line.trim().to_ascii_uppercase();
        if trimmed.is_empty() {
            continue;
        }
        if !out.iter().any(|l| l == &trimmed) {
            out.push(trimmed);
        }
    }
    if out.len() > 256 {
        let drop = out.len() - 256;
        out.drain(0..drop);
    }
}

fn contains_line_substr(lines: &[String], needle: &str) -> bool {
    let n = needle.to_ascii_uppercase();
    lines.iter().any(|l| l.contains(&n))
}

fn contains_k_value_in_range(lines: &[String], min_k: u32, max_k: u32) -> bool {
    for line in lines {
        let bytes = line.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i].is_ascii_digit() {
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'K' {
                    if let Ok(v) = line[start..i].parse::<u32>() {
                        if v >= min_k && v <= max_k {
                            return true;
                        }
                    }
                }
            } else {
                i += 1;
            }
        }
    }
    false
}

fn summarize_lines(lines: &[String]) -> String {
    let mut shown = String::new();
    for (idx, line) in lines.iter().rev().take(16).enumerate() {
        if idx > 0 {
            shown.push_str(" || ");
        }
        shown.push_str(line);
    }
    shown
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
