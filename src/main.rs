use clap::Parser;
use iz80::*;
use std::time::{Duration, Instant};

mod config;
mod kaypro_machine;
mod floppy_controller;
mod hard_disk;
#[cfg(unix)]
mod keyboard_unix;
#[cfg(windows)]
mod keyboard_win;
mod media;
mod screen;
mod rtc;
mod sio;
mod sy6545;
mod diagnostics;
#[cfg(test)]
mod format_test;

use self::config::{Config, KayproModel};
use self::kaypro_machine::KayproMachine;
use self::floppy_controller::FloppyController;
use self::screen::Screen;
#[cfg(unix)]
use self::keyboard_unix::Command;
#[cfg(windows)]
use self::keyboard_win::Command;

#[derive(Parser)]
#[command(
    name = "izkaypro",
    about = "Kaypro computer emulator for the terminal",
    long_about = "izkaypro - Kaypro Emulator\n\
        https://github.com/ivanizag/izkaypro\n\n\
        Emulates Kaypro II, 4/83, 2X/4/84, TurboROM, and KayPLUS computers.\n\
        Configuration is loaded from izkaypro.toml; command-line arguments override config file settings.",
    version,
)]
struct Cli {
    /// Kaypro model preset [models: kaypro_ii, kaypro4_83, kaypro4_84, turbo_rom, kayplus_84, kaypro10, custom]
    #[arg(short = 'm', long, value_name = "MODEL")]
    model: Option<String>,

    /// Disk image file for drive A
    #[arg(short = 'a', long, value_name = "FILE")]
    drivea: Option<String>,

    /// Disk image file for drive B
    #[arg(short = 'b', long, value_name = "FILE")]
    driveb: Option<String>,

    /// Hard disk image file for Kaypro 10 (creates blank image if file doesn't exist)
    #[arg(long, value_name = "FILE")]
    hd: Option<String>,

    /// Custom ROM file (implies --model=custom)
    #[arg(long, value_name = "FILE")]
    rom: Option<String>,

    /// CPU clock speed in MHz (1-100, default: unlimited)
    #[arg(long, value_name = "MHZ")]
    speed: Option<f64>,

    /// Trace CPU instruction execution
    #[arg(short = 'c', long)]
    cpu_trace: bool,

    /// Trace I/O port access
    #[arg(short = 'i', long)]
    io_trace: bool,

    /// Trace floppy disk controller commands
    #[arg(short = 'f', long)]
    fdc_trace: bool,

    /// Trace floppy disk controller read/write data
    #[arg(short = 'w', long)]
    fdc_trace_rw: bool,

    /// Trace system bit changes
    #[arg(short = 's', long)]
    system_bits: bool,

    /// Trace ROM entry point calls
    #[arg(short = 'r', long)]
    rom_trace: bool,

    /// Trace CP/M BDOS calls
    #[arg(long)]
    bdos_trace: bool,

    /// Trace SY6545 CRTC VRAM writes
    #[arg(short = 'v', long)]
    crtc_trace: bool,

    /// Trace SIO-1 Channel A serial port
    #[arg(long)]
    sio_trace: bool,

    /// Trace MM58167A real-time clock register access
    #[arg(long)]
    rtc_trace: bool,

    /// Trace WD1002-05 hard disk controller (Kaypro 10)
    #[arg(long)]
    hdc_trace: bool,

    /// Connect SIO-1 Port A to a serial device (e.g., /dev/ttyUSB0, /tmp/kayproA)
    #[arg(long, value_name = "DEVICE")]
    serial: Option<String>,

    /// Enable all trace options
    #[arg(long)]
    trace_all: bool,

    /// Run ROM and RAM diagnostics then exit
    #[arg(short = 'd', long)]
    diagnostics: bool,

    /// Run headless boot tests for all Kaypro models then exit
    #[arg(long)]
    boot_test: bool,

    /// Trace TurboROM+HD boot sequence (debug diagnostic)
    #[arg(long, hide = true)]
    trace_turborom_hd: bool,

    /// Write HDC/ROM/BDOS traces to a log file (screen keeps working)
    #[arg(long, value_name = "FILE")]
    trace_log: Option<String>,

    /// Debug MAKTURBO on Kaypro 10 (headless, traces to makturbo-debug.log)
    #[arg(long, hide = true)]
    debug_makturbo: bool,

    /// Debug Kaypro 10 floppy access after HD boot (traces to k10-floppy-debug.log)
    #[arg(long, hide = true)]
    debug_floppy_k10: bool,

    /// Run without screen border (fits in 80x26 terminal)
    #[arg(long)]
    no_border: bool,
}

fn main() {
    let cli = Cli::parse();

    // Load configuration from file, then apply CLI overrides
    let mut config = Config::load();
    config.apply_cli_overrides(
        cli.model.as_deref(),
        cli.rom.as_deref(),
        cli.drivea.as_deref(),
        cli.driveb.as_deref(),
    );

    let welcome = format!(
        "izkaypro - Kaypro Emulator\nhttps://github.com/ivanizag/izkaypro\nConfiguration: {}",
        config.get_description()
    );

    // Kaypro 10 has no floppies by default (HD boot); only load if user specified
    let disk_a_path = if config.model == KayproModel::Kaypro10 {
        config.disk_a.clone().unwrap_or_default()
    } else {
        config.disk_a.clone()
            .unwrap_or_else(|| config.get_default_disk_a().to_string())
    };
    let disk_b_path = if config.model == KayproModel::Kaypro10 {
        config.disk_b.clone().unwrap_or_default()
    } else {
        config.disk_b.clone()
            .unwrap_or_else(|| config.get_default_disk_b().to_string())
    };

    let has_trace_log = cli.trace_log.is_some();
    let mut trace_cpu = cli.cpu_trace || cli.trace_all;
    let trace_io = cli.io_trace || cli.trace_all;
    let trace_fdc = cli.fdc_trace || cli.trace_all || has_trace_log;
    let trace_fdc_rw = cli.fdc_trace_rw || cli.trace_all || has_trace_log;
    let trace_system_bits = cli.system_bits || cli.trace_all;
    let trace_rom = cli.rom_trace || cli.trace_all || has_trace_log;
    let trace_bdos = cli.bdos_trace || cli.trace_all || has_trace_log;
    let trace_crtc = cli.crtc_trace || cli.trace_all;
    let trace_sio = cli.sio_trace || cli.trace_all;
    let trace_rtc = cli.rtc_trace || cli.trace_all;
    let trace_hdc = cli.hdc_trace || cli.trace_all || has_trace_log;
    let run_diag = cli.diagnostics;
    let run_boot_test = cli.boot_test;
    // Kaypro 10: controller always present (soldered to motherboard).
    // TurboROM: controller only present when --hd is specified (add-on card).
    // Without the controller, TurboROM loads the disk-based TURBO-BIOS;
    // with it, TurboROM activates its ROM-resident BIOS which needs a
    // formatted HD parameter sector to operate correctly.
    let has_hard_disk = config.model == KayproModel::Kaypro10
        || (config.model == KayproModel::TurboRom && cli.hd.is_some());

    // When --trace-log is used, traces go to a file and don't affect screen rendering.
    // Only count traces that go to stdout/stderr as "any_trace".
    let any_trace = trace_io
        || trace_cpu
        || (trace_fdc && !has_trace_log)
        || (trace_fdc_rw && !has_trace_log)
        || (trace_rom && !has_trace_log)
        || (trace_bdos && !has_trace_log)
        || trace_crtc
        || trace_sio
        || trace_rtc
        || (trace_hdc && !has_trace_log)
        || trace_system_bits;

    // Init device with configuration
    let floppy_controller = FloppyController::new(
        &disk_a_path,
        &disk_b_path,
        config.get_disk_format(),
        config.get_side1_sector_base(),
        trace_fdc,
        trace_fdc_rw,
    );
    let mut screen = Screen::new(!any_trace, config.get_display_name(), cli.no_border);
    let mut machine = KayproMachine::new(
        config.get_rom_path(),
        config.get_video_mode(),
        floppy_controller,
        has_hard_disk,
        trace_io,
        trace_system_bits,
        trace_crtc,
        trace_sio,
        trace_rtc,
        trace_hdc,
    );
    machine.kayplus_clock_fixup = config.model == KayproModel::KayPlus84;

    // Kaypro 10 boot priority: the ROM checks FDC NOT READY at power-on.
    // NOT READY → HD boot, READY → floppy boot. Set disk_in_drive=false
    // when HD is present and no user floppy was specified, so the ROM
    // boots from the hard disk.
    if has_hard_disk && cli.drivea.is_none() {
        machine.floppy_controller.disk_in_drive = false;
    }

    // Load hard disk image: use --hd path if specified, otherwise use default for Kaypro 10
    let hd_path = cli.hd.clone().or_else(|| {
        if config.model == KayproModel::Kaypro10 {
            Some("disks/system/kaypro10.hd".to_string())
        } else {
            None
        }
    });
    if let Some(ref hd_path) = hd_path {
        if let Some(ref mut hd) = machine.hard_disk {
            match hd.load_image(hd_path) {
                Ok(()) => {},
                Err(e) => eprintln!("Warning: Failed to load hard disk image '{}': {}", hd_path, e),
            }
        } else {
            eprintln!("Warning: --hd specified but model doesn't support hard disk (use --model kaypro10)");
        }
    }

    // Set up trace log file(s). ROM/BDOS traces go to the specified file;
    // HDC register-level traces go to a companion file with "-hdc" suffix
    // (two separate handles avoid interleaving/overwrite issues).
    let mut trace_log: Option<std::fs::File> = None;
    if let Some(ref log_path) = cli.trace_log {
        use std::io::Write;
        let mut f = std::fs::File::create(log_path)
            .unwrap_or_else(|e| { eprintln!("Failed to create trace log '{}': {}", log_path, e); std::process::exit(1); });
        let _ = writeln!(f, "=== izkaypro trace log ===");
        let _ = writeln!(f, "Config: {}", config.get_description());
        let _ = writeln!(f, "");
        // HDC register-level traces go to a companion file
        let hdc_log_path = format!("{}-hdc.log",
            log_path.strip_suffix(".log").unwrap_or(log_path));
        if let Some(ref mut hd) = machine.hard_disk {
            let hdc_file = std::fs::File::create(&hdc_log_path)
                .expect("failed to create HDC trace log");
            hd.set_trace_file(hdc_file);
        }
        // FDC-level traces go to a companion file with "-fdc" suffix
        let fdc_log_path = format!("{}-fdc.log",
            log_path.strip_suffix(".log").unwrap_or(log_path));
        let fdc_file = std::fs::File::create(&fdc_log_path)
            .expect("failed to create FDC trace log");
        machine.floppy_controller.trace_file = Some(fdc_file);
        eprintln!("Tracing ROM/BDOS to {}", log_path);
        eprintln!("Tracing HDC registers to {}", hdc_log_path);
        eprintln!("Tracing FDC reads to {}", fdc_log_path);
        trace_log = Some(f);
    }

    let mut cpu = Cpu::new_z80();
    cpu.set_trace(trace_cpu);

    // Run boot tests if requested
    if run_boot_test {
        println!("Running boot tests for all Kaypro models...\n");
        let results = diagnostics::run_boot_tests();
        diagnostics::print_results(&results);
        let all_passed = results.iter().all(|r| r.passed);
        std::process::exit(if all_passed { 0 } else { 1 });
    }

    // Trace TurboROM+HD boot if requested
    if cli.trace_turborom_hd {
        diagnostics::trace_turborom_hd_boot();
        return;
    }

    // Debug MAKTURBO if requested
    if cli.debug_makturbo {
        diagnostics::debug_makturbo();
        return;
    }

    // Debug Kaypro 10 floppy access if requested
    if cli.debug_floppy_k10 {
        diagnostics::debug_floppy_k10();
        return;
    }

    // Run diagnostics if requested
    if run_diag {
        println!("{}", welcome);
        // Determine ROM size based on current configuration
        let rom_size = 0x1000; // 4KB for most Kaypro ROMs
        let mut results = diagnostics::run_diagnostics(&mut machine, rom_size);
        // Add VRAM test (requires direct CRTC access)
        results.push(diagnostics::test_vram(&mut machine.crtc));
        // Add VRAM test via port I/O protocol (same as EMUTEST)
        results.push(diagnostics::test_vram_via_ports(&mut machine.crtc));
        // Add Attribute RAM test (fourth video test from diag4.mac)
        results.push(diagnostics::test_attr_ram(&mut machine.crtc));
        diagnostics::print_results(&results);
        return;
    }

    // Open serial device if specified
    if let Some(ref device) = cli.serial {
        match machine.sio.open_serial(device) {
            Ok(()) => println!("Serial port: {}", device),
            Err(e) => eprintln!("Warning: {}", e),
        }
    }

    // Start the cpu
    println!("{}", welcome);
    screen.init();

    let instructions_per_refresh = if any_trace {256*1024} else {2*1024};

    // Clock speed control: None = unlimited, Some(mhz) = fixed speed
    // Average ~4 T-states per instruction, so cycles_per_sec = mhz * 1_000_000
    let mut clock_mhz: Option<f64> = cli.speed.and_then(|mhz| {
        if mhz < 0.0 {
            None
        } else if mhz >= 1.0 && mhz <= 100.0 {
            Some((mhz * 2.0).round() / 2.0)
        } else {
            eprintln!("Warning: --speed must be 1-100 MHz, ignoring");
            None
        }
    });
    let mut cycle_count: u64 = 0;
    let mut speed_start_time = Instant::now();
    const CYCLES_PER_INSTRUCTION: u64 = 4; // Average Z80 cycles per instruction

    let mut counter: u64 = 1;
    let mut nmi_pending = false;
    let mut nmi_deadline: u64 = 0;
    let mut done = false;
    // Runtime BIOS base discovery for universal ROM tracing
    let mut bios_base: Option<u16> = None;
    let mut last_rom_rank = true; // Start in ROM mode
    while !done {

        cpu.execute_instruction(&mut machine);
        counter += 1;
        cycle_count += CYCLES_PER_INSTRUCTION;

        // KayPLUS software clock fixup: intercept the BIOS tick routine
        // at 0x069E (start of the seconds/minutes/hours increment loop).
        // Patch RAM counters with real RTC time and skip past the loop
        // so the display code at 0x06CE reads accurate values.
        if machine.kayplus_clock_fixup
            && machine.is_rom_rank()
            && cpu.registers().pc() == 0x069E
        {
            machine.patch_software_clock();
            cpu.registers().set_pc(0x06CE);
        }

        // Clock speed throttling
        if let Some(mhz) = clock_mhz {
            let target_cycles_per_sec = (mhz * 1_000_000.0) as u64;
            let elapsed = speed_start_time.elapsed();
            let expected_cycles = (elapsed.as_secs_f64() * target_cycles_per_sec as f64) as u64;
            
            if cycle_count > expected_cycles {
                // We're running too fast, need to wait
                let cycles_ahead = cycle_count - expected_cycles;
                let wait_secs = cycles_ahead as f64 / target_cycles_per_sec as f64;
                if wait_secs > 0.0001 {
                    std::thread::sleep(Duration::from_secs_f64(wait_secs));
                }
            }
            
            // Reset counters periodically to avoid drift
            if elapsed.as_secs() >= 1 {
                speed_start_time = Instant::now();
                cycle_count = 0;
            }
        }

        // IO refresh
        if counter % instructions_per_refresh == 0 {
            machine.keyboard.consume_input();
            screen.update(&mut machine, false);
        }

        if !machine.keyboard.commands.is_empty() {
            let commands = machine.keyboard.commands.clone();
            for command in commands {
                match command {
                    Command::Quit => {
                        machine.floppy_controller.media_selected().flush_disk();
                        if let Some(ref mut hd) = machine.hard_disk {
                            hd.flush();
                        }
                        done = true;
                    },
                    Command::Help => {
                        screen.show_help = !screen.show_help;
                    },
                    Command::ShowStatus => {
                        screen.show_status = !screen.show_status;
                    },
                    Command::SelectDiskA => {
                        let prompt = if has_hard_disk {
                            "File to load in Drive C (floppy)"
                        } else {
                            "File to load in Drive A"
                        };
                        if let Some(path) = screen.prompt(&mut machine, prompt) {
                            let res = machine.floppy_controller.media_a_mut().load_disk(path.as_str());
                            if let Err(err) = res {
                                screen.message(&mut machine, &err.to_string())
                            } else {
                                machine.floppy_controller.disk_in_drive = true;
                                machine.floppy_controller.motor_on = true;
                                // Kaypro 10: the ROM cached the floppy drive type at
                                // boot when no disk was present (defaulting to SSDD).
                                // Patch the drive type table at 0xFFF6 to match the
                                // actual format of the inserted disk image.
                                if has_hard_disk {
                                    let format = machine.floppy_controller.media_a().format;
                                    let type_byte = match format {
                                        media::MediaFormat::DsDd => 0x09,
                                        media::MediaFormat::SsDd => 0x05,
                                        _ => 0x01,
                                    };
                                    machine.poke(0xFFF6, type_byte);
                                }
                            }
                        }
                    }
                    Command::SelectDiskB => {
                        if has_hard_disk {
                            screen.message(&mut machine, "Kaypro 10 has only one floppy drive (C:)");
                        } else if let Some(path) = screen.prompt(&mut machine, "File to load in Drive B") {
                            let res = machine.floppy_controller.media_b_mut().load_disk(path.as_str());
                            if let Err(err) = res {
                                screen.message(&mut machine, &err.to_string())
                            }
                        }
                    }
                    Command::SaveMemory => {
                        match machine.save_bios() {
                            Ok(filename) => {
                                screen.message(&mut machine, &format!("BIOS saved as {}", filename));
                            }
                            Err(err) => {
                                screen.message(&mut machine, &format!("Error: {}", err));
                            }
                        }
                    }
                    Command::TraceCPU => {
                        trace_cpu = !trace_cpu;
                        cpu.set_trace(trace_cpu);
                        screen.set_in_place(!trace_cpu && !any_trace);
                    },
                    Command::SetSpeed => {
                        let current = match clock_mhz {
                            Some(mhz) => format!("{:.1}", mhz),
                            None => "-1".to_string(),
                        };
                        let prompt = format!("CPU speed in MHz (1-100, -1=unlimited) [{}]", current);
                        if let Some(input) = screen.prompt(&mut machine, &prompt) {
                            let input = input.trim();
                            if input.is_empty() {
                                // Keep current setting
                            } else if let Ok(mhz) = input.parse::<f64>() {
                                if mhz < 0.0 {
                                    clock_mhz = None;
                                } else if mhz >= 1.0 && mhz <= 100.0 {
                                    // Round to 0.5 MHz resolution
                                    let rounded = (mhz * 2.0).round() / 2.0;
                                    clock_mhz = Some(rounded);
                                    speed_start_time = Instant::now();
                                    cycle_count = 0;
                                }
                                // Invalid range silently ignored
                            }
                            // Invalid parse silently ignored
                        }
                    },
                }
            }
            screen.update(&mut machine, true);
            machine.keyboard.commands.clear();
        }

        // SIO interrupt processing (keyboard)
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
        // The FDC sets raise_nmi when a command completes or a data byte is
        // transferred. We latch it as pending and deliver when:
        //  1. CPU is HALTed (immediate — standard BIOS FDC loops), OR
        //  2. Deadline reached AND vector at 0x0066 is safe (fallback for
        //     programs like DIAG4 that poll FDC without HALTing).
        // KayPLUS (unsafe vector at 0x0066) only gets NMI via path 1.
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
            screen.update(&mut machine, true);
            println!("HALT instruction that will never be interrupted");
            break;
        }

        // Runtime BIOS base discovery: detect ROM→RAM transition
        if (trace_rom || trace_bdos) && has_trace_log {
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
                        if let Some(ref mut f) = trace_log {
                            use std::io::Write;
                            let _ = writeln!(f, "[{:>10}] BIOS base discovered: 0x{:04X}", counter, base);
                        }
                    }
                }
            }

            let pc = cpu.registers().pc();

            // BIOS entry point tracing (runtime, works with any ROM)
            if !in_rom {
                if let Some(base) = bios_base {
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
                            12 => Some("SETDMA".into()),
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
                            if let Some(ref mut f) = trace_log {
                                use std::io::Write;
                                let _ = writeln!(f, "[{:>10}] BIOS: {}", counter, m);
                                let _ = f.flush();
                            } else {
                                println!("BIOS: {}", m);
                            }
                        }
                    }
                }
            }
        }

        if trace_bdos && !machine.is_rom_rank()
                && cpu.registers().pc() == 0x0005 {
            let command = cpu.registers().get8(Reg8::C);
            if command != 0x06 /*C_RAWIO*/ {
                let args = cpu.registers().get16(Reg16::DE);
                let name = if command < BDOS_COMMAND_NAMES.len() as u8 {
                    BDOS_COMMAND_NAMES[command as usize]
                } else {
                    "unknown"
                };

                if let Some(ref mut f) = trace_log {
                    use std::io::Write;
                    let _ = writeln!(f, "[{:>10}] BDOS {}: {}({:04x})", counter, command, name, args);
                    // For file operations, dump FCB filename
                    if command == 15 || command == 17 || command == 22 {
                        let de = args;
                        let fcb_drive = machine.peek(de);
                        let mut filename = String::new();
                        for i in 1..=11u16 {
                            let ch = machine.peek(de.wrapping_add(i)) & 0x7F;
                            if ch >= 0x20 { filename.push(ch as char); }
                        }
                        let _ = writeln!(f, "[{:>10}]   FCB: drive={} file=\"{}\"",
                            counter, fcb_drive, filename.trim());
                    }
                    let _ = f.flush();
                } else {
                    println!("BDOS command {}: {}({:04x})", command, name, args);
                }
            }
        }
    }
}

const BDOS_COMMAND_NAMES: [&str; 50] = [
    // 0
    "P_TERMCPM", "C_READ", "C_WRITE", "A_READ", "A_WRITE",
    "L_WRITE", "C_RAWIO", "A_STATIN", "A_STATOUT", "C_WRITESTR",
    // 10
    "C_READSTR", "C_STAT", "S_BDOSVER", "DRV_ALLRESET", "DRV_SET",
    "F_OPEN", "F_CLOSE", "F_SFIRST", "F_SNEXT", "F_DELETE",
    // 20
    "F_READ", "F_WRITE", "F_MAKE", "F_RENAME", "DRV_LOGINVEC",
    "DRV_GET", "F_DMAOFF", "DRV_ALLOCVEC", "DRV_SETRO", "DRV_ROVEC",
    // 30
    "F_ATTRIB", "DRV_DPB", "F_USERNUM", "F_READRAND", "F_WRITERAND",
    "F_SIZE", "F_RANDREC", "DRV_RESET", "*", "",
    // 40
    "F_WRITEZ", "", "", "", "",
    "F_ERRMODE", "", "", "", "",
    ];
