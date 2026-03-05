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
#[cfg(feature = "gui")]
mod renderer;
#[cfg(test)]
mod format_test;

use self::config::{Config, KayproModel, resolve_path};
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
        Emulates Kaypro II, 4/83, 2X/4/84, TurboROM, TurboROM+HD, and KayPLUS computers.\n\
        Configuration is loaded from izkaypro.toml; command-line arguments override config file settings.",
    version,
)]
struct Cli {
    /// Kaypro model preset [models: kaypro_ii, kaypro4_83, kaypro4_84, turbo_rom, turbo_rom_hd, ultimate, kayplus_84, kaypro10, custom]
    #[arg(short = 'm', long, value_name = "MODEL")]
    model: Option<String>,

    /// Disk image file for drive A
    #[arg(short = 'a', long, value_name = "FILE")]
    drivea: Option<String>,

    /// Disk image file for drive B
    #[arg(short = 'b', long, value_name = "FILE")]
    driveb: Option<String>,

    /// Hard disk image file for WD1002 models (creates blank image if file doesn't exist)
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

    /// Trace WD1002-05 hard disk controller
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

    /// Write HDC/ROM/BDOS traces to a log file (screen keeps working)
    #[arg(long, value_name = "FILE")]
    trace_log: Option<String>,

    /// Run without screen border (fits in 80x26 terminal)
    #[arg(long)]
    no_border: bool,

    /// Launch chargen rendering window (requires 'gui' feature)
    #[arg(long)]
    chargen: bool,

    /// Phosphor color for chargen mode [green, amber, white, blue] (default: green)
    #[arg(long, value_name = "COLOR", default_value = "green")]
    phosphor: String,

    /// Override phosphor foreground color (hex, e.g. "#33FF33")
    #[arg(long, value_name = "HEX")]
    phosphor_fg: Option<String>,

    /// Override phosphor background color (hex, e.g. "#002200")
    #[arg(long, value_name = "HEX")]
    phosphor_bg: Option<String>,

    /// Override phosphor dim color (hex, e.g. "#1A801A")
    #[arg(long, value_name = "HEX")]
    phosphor_dim: Option<String>,
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

    // HD-boot models have no floppies by default; only load if user specified
    let hd_boot = config.model == KayproModel::Kaypro10
        || config.model == KayproModel::TurboRomHd
        || config.model == KayproModel::Ultimate;
    let disk_a_path = if hd_boot {
        config.disk_a.as_deref().map(resolve_path).unwrap_or_default()
    } else {
        resolve_path(config.disk_a.as_deref()
            .unwrap_or(config.get_default_disk_a()))
    };
    let disk_b_path = if hd_boot {
        config.disk_b.as_deref().map(resolve_path).unwrap_or_default()
    } else {
        resolve_path(config.disk_b.as_deref()
            .unwrap_or(config.get_default_disk_b()))
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
    // TurboROM+HD model: controller always present with default image.
    // Without the controller, TurboROM loads the disk-based TURBO-BIOS;
    // with it, TurboROM activates its ROM-resident BIOS which needs a
    // formatted HD parameter sector to operate correctly.
    let is_kaypro10_hardware = config.model == KayproModel::Kaypro10;
    let has_hard_disk = config.model == KayproModel::Kaypro10
        || config.model == KayproModel::TurboRomHd
        || config.model == KayproModel::Ultimate
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
        &resolve_path(config.get_rom_path()),
        config.get_video_mode(),
        floppy_controller,
        has_hard_disk,
        is_kaypro10_hardware,
        trace_io,
        trace_system_bits,
        trace_crtc,
        trace_sio,
        trace_rtc,
        trace_hdc,
    );

    // TurboROM+HD: only LUN 1 should report READY. The ROM probes LUN 2
    // as well, but reporting it READY with the same backing image causes
    // the ROM to see two identical drives (4 partitions instead of 2).
    // LUN 1 only (the default) is correct for a single-drive setup.

    machine.kayplus_clock_fixup = config.model == KayproModel::KayPlus84;

    // HD systems map floppies differently than floppy-only models:
    // Kaypro 10: single floppy is Drive C
    // Advent board (TurboROM+HD, Ultimate): floppies are C and D
    if is_kaypro10_hardware {
        screen.floppy_drive_labels = ('C', 'C');
    } else if has_hard_disk {
        screen.floppy_drive_labels = ('C', 'D');
    }

    // Kaypro 10 boot priority: the ROM checks FDC NOT READY at power-on.
    // NOT READY → HD boot, READY → floppy boot. Set disk_in_drive=false
    // when HD is present and no user floppy was specified, so the ROM
    // boots from the hard disk.
    if is_kaypro10_hardware && cli.drivea.is_none() {
        machine.floppy_controller.disk_in_drive = false;
    }

    // Load hard disk image: use --hd path if specified, otherwise use model defaults.
    let hd_path = cli.hd.clone().or_else(|| {
        match config.model {
            KayproModel::Kaypro10 => Some(resolve_path("disks/system/kaypro10.hd")),
            KayproModel::TurboRomHd => Some(resolve_path("disks/system/turborom.hd")),
            KayproModel::Ultimate => Some(resolve_path("disks/system/turborom_nz.hd")),
            _ => None,
        }
    });
    if let Some(ref hd_path) = hd_path {
        if let Some(ref mut hd) = machine.hard_disk {
            match hd.load_image(hd_path) {
                Ok(()) => {},
                Err(e) => eprintln!("Warning: Failed to load hard disk image '{}': {}", hd_path, e),
            }
        } else {
            eprintln!("Warning: --hd specified but model doesn't support hard disk (use --model kaypro10|turbo_rom_hd|turbo_rom)");
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
                .unwrap_or_else(|e| {
                    eprintln!("Failed to create HDC trace log '{}': {}", hdc_log_path, e);
                    std::process::exit(1);
                });
            hd.set_trace_file(hdc_file);
        }
        // FDC-level traces go to a companion file with "-fdc" suffix
        let fdc_log_path = format!("{}-fdc.log",
            log_path.strip_suffix(".log").unwrap_or(log_path));
        let fdc_file = std::fs::File::create(&fdc_log_path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to create FDC trace log '{}': {}", fdc_log_path, e);
                std::process::exit(1);
            });
        machine.floppy_controller.trace_file = Some(fdc_file);
        // RTC traces go to a companion file with "-rtc" suffix
        let rtc_log_path = format!("{}-rtc.log",
            log_path.strip_suffix(".log").unwrap_or(log_path));
        let rtc_file = std::fs::File::create(&rtc_log_path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to create RTC trace log '{}': {}", rtc_log_path, e);
                std::process::exit(1);
            });
        machine.rtc.trace_file = Some(rtc_file);
        eprintln!("Tracing ROM/BDOS to {}", log_path);
        eprintln!("Tracing HDC registers to {}", hdc_log_path);
        eprintln!("Tracing FDC reads to {}", fdc_log_path);
        eprintln!("Tracing RTC to {}", rtc_log_path);
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

    // Chargen mode: launch graphical window instead of terminal rendering
    #[cfg(feature = "gui")]
    if cli.chargen {
        let mut phosphor = renderer::PhosphorColors::from_name(&cli.phosphor).unwrap_or_else(|| {
            eprintln!("Unknown phosphor color '{}', using green. Options: green, amber, white, blue", cli.phosphor);
            renderer::PHOSPHOR_GREEN
        });
        if let Some(ref hex) = cli.phosphor_fg {
            match renderer::PhosphorColors::parse_hex(hex) {
                Some(c) => phosphor.fg = c,
                None => eprintln!("Invalid --phosphor-fg '{}', expected hex like #33FF33", hex),
            }
        }
        if let Some(ref hex) = cli.phosphor_bg {
            match renderer::PhosphorColors::parse_hex(hex) {
                Some(c) => phosphor.bg = c,
                None => eprintln!("Invalid --phosphor-bg '{}', expected hex like #002200", hex),
            }
        }
        if let Some(ref hex) = cli.phosphor_dim {
            match renderer::PhosphorColors::parse_hex(hex) {
                Some(c) => phosphor.dim = c,
                None => eprintln!("Invalid --phosphor-dim '{}', expected hex like #1A801A", hex),
            }
        }
        println!("{}", welcome);
        run_gui(&config, machine, cpu, trace_cpu, is_kaypro10_hardware, cli.speed, screen.floppy_drive_labels, phosphor);
        return;
    }
    #[cfg(not(feature = "gui"))]
    if cli.chargen {
        eprintln!("Error: --chargen requires building with: cargo run --features gui");
        std::process::exit(1);
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
                        let (la, _) = screen.floppy_drive_labels;
                        let prompt = format!("File to load in Drive {}", la);
                        if let Some(path) = screen.prompt(&mut machine, &prompt) {
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
                                if is_kaypro10_hardware {
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
                        if is_kaypro10_hardware {
                            screen.message(&mut machine, "Kaypro 10 has only one floppy drive (C:)");
                        } else {
                            let (_, lb) = screen.floppy_drive_labels;
                            let prompt = format!("File to load in Drive {}", lb);
                            if let Some(path) = screen.prompt(&mut machine, &prompt) {
                                let res = machine.floppy_controller.media_b_mut().load_disk(path.as_str());
                                if let Err(err) = res {
                                    screen.message(&mut machine, &err.to_string())
                                }
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

#[cfg(feature = "gui")]
fn run_gui(
    config: &Config,
    mut machine: KayproMachine,
    mut cpu: iz80::Cpu,
    mut trace_cpu: bool,
    _is_kaypro10_hardware: bool,
    speed: Option<f64>,
    floppy_drive_labels: (char, char),
    phosphor: renderer::PhosphorColors,
) {
    use minifb::{Key, KeyRepeat, Window, WindowOptions, Scale};

    let mut renderer = renderer::Renderer::new(&resolve_path(config.get_chargen_path()), phosphor);

    // Kaypro II/4-83: 640×192 native → double scanlines to 640×384 for CRT
    // aspect ratio, then Scale::X2 → 1280×768. Other models: 640×400 × X2.
    let scale = Scale::X2;

    let (display_w, display_h) = renderer.display_size();
    let mut window = Window::new(
        &format!("izkaypro — {}", config.get_display_name()),
        display_w,
        display_h,
        WindowOptions {
            scale,
            resize: true,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| {
        eprintln!("Failed to create window: {}", e);
        std::process::exit(1);
    });

    // minifb omits NSWindowStyleMaskMiniaturizable on macOS, so the yellow
    // minimize traffic-light is grayed out. Patch it via Objective-C runtime.
    // On Windows/Linux the minimize button is already present natively.
    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_void;
        extern "C" {
            fn objc_msgSend();
            fn sel_registerName(name: *const u8) -> *mut c_void;
        }
        unsafe {
            // Typed function pointers for ARM64 ABI (variadic decl is wrong on aarch64)
            let get_mask: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u64 =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let set_mask: unsafe extern "C" fn(*mut c_void, *mut c_void, u64) =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());

            let nswindow = window.get_window_handle();
            let style_sel = sel_registerName(b"styleMask\0".as_ptr());
            let set_sel = sel_registerName(b"setStyleMask:\0".as_ptr());
            let current = get_mask(nswindow, style_sel);
            const NS_MINIATURIZABLE: u64 = 1 << 2;
            set_mask(nswindow, set_sel, current | NS_MINIATURIZABLE);
        }
    }

    window.set_target_fps(60);

    // Clock speed control
    let clock_mhz: Option<f64> = speed.and_then(|mhz| {
        if mhz < 0.0 { None }
        else if mhz >= 1.0 && mhz <= 100.0 { Some((mhz * 2.0).round() / 2.0) }
        else { None }
    });
    let mut cycle_count: u64 = 0;
    let mut speed_start_time = Instant::now();
    const CYCLES_PER_INSTRUCTION: u64 = 4;

    let mut counter: u64 = 1;
    let mut nmi_pending = false;
    let mut nmi_deadline: u64 = 0;

    // Run enough instructions per frame for responsive emulation.
    // At unlimited speed, execute ~166K instructions per 60fps frame
    // (~10M inst/sec, enough for Kaypro 10 to boot in ~1 second).
    // With clock speed set, the throttling logic handles pacing.
    let instructions_per_frame: u64 = 166_000;

    // GUI mode: keyboard input comes from minifb window, not stdin.
    machine.keyboard.gui_mode = true;

    let mut show_help = false;
    let mut show_status = false;
    let mut speed_input: Option<String> = None; // Some = F9 input mode active
    let mut clock_mhz = clock_mhz; // make mutable for speed changes

    let mut prev_f5_down = false;
    let mut prev_f6_down = false;

    while window.is_open() {
        // Compute instructions per frame based on clock speed.
        // At a fixed MHz, execute exactly enough cycles for one 60fps frame.
        // At unlimited speed, run a large batch for fast emulation.
        // When tracing, reduce batch so stdout doesn't block the GUI loop.
        let batch: u64 = if trace_cpu {
            1_000
        } else if let Some(mhz) = clock_mhz {
            // cycles_per_frame = target_cycles_per_sec / 60
            let cycles_per_frame = (mhz * 1_000_000.0 / 60.0) as u64;
            (cycles_per_frame / CYCLES_PER_INSTRUCTION).max(1000)
        } else {
            instructions_per_frame
        };

        // When idle polling exceeds the threshold (same 50K as terminal mode),
        // reduce the batch to match ~4 MHz execution speed (~67K cycles/frame
        // at 60fps = ~17K instructions). This prevents TurboROM's software
        // timeout counter from firing prematurely at unlimited speed, while
        // maintaining full speed during real work. idle_polls resets to 0
        // automatically in is_key_pressed() when a key is available.
        let effective_batch = if machine.keyboard.idle_polls > 50_000 {
            batch.min(17_000)
        } else {
            batch
        };

        // Execute a batch of CPU instructions per frame
        for i in 0..effective_batch {
            // Simulate CRTC vertical retrace timing within the batch.
            // TurboROM has a two-phase VRT wait: first wait for VRT=0 (retrace
            // end), then wait for VRT=1 (retrace start). We pulse VRT high for
            // the last ~10% of the batch, matching real CRT timing where retrace
            // occupies ~10% of each frame period.
            let vrt = i >= batch * 9 / 10;
            if vrt != machine.crtc.vertical_retrace {
                machine.crtc.set_vertical_retrace(vrt);
            }
            cpu.execute_instruction(&mut machine);
            counter += 1;
            cycle_count += CYCLES_PER_INSTRUCTION;

            // KayPLUS software clock fixup
            if machine.kayplus_clock_fixup
                && machine.is_rom_rank()
                && cpu.registers().pc() == 0x069E
            {
                machine.patch_software_clock();
                cpu.registers().set_pc(0x06CE);
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
            }

            // SIO interrupt processing
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
        }

        // Clock speed throttling: sleep once per frame if we're ahead of schedule.
        if let Some(mhz) = clock_mhz {
            let target_cycles_per_sec = (mhz * 1_000_000.0) as u64;
            let elapsed = speed_start_time.elapsed();
            let expected_cycles = (elapsed.as_secs_f64() * target_cycles_per_sec as f64) as u64;
            if cycle_count > expected_cycles {
                let cycles_ahead = cycle_count - expected_cycles;
                let wait_secs = cycles_ahead as f64 / target_cycles_per_sec as f64;
                if wait_secs > 0.0001 {
                    std::thread::sleep(Duration::from_secs_f64(wait_secs));
                }
            }
            if elapsed.as_secs() >= 1 {
                speed_start_time = Instant::now();
                cycle_count = 0;
            }
        }

        // Render frame (update_with_buffer pumps macOS events, must come
        // before key polling so get_keys_pressed returns current state).
        renderer.tick_frame();
        renderer.render(&machine);

        // Overlay: help and/or status
        if show_help {
            let (la, lb) = floppy_drive_labels;
            let help_lines: Vec<String> = vec![
                "izkaypro: Kaypro Emulator".into(),
                "".into(),
                format!("F1: Help  F2: Status  F4: Quit"),
                format!("F5: Drive {}  F6: Drive {}  F7: Save BIOS", la, lb),
                format!("F8: CPU Trace  F9: Set Speed"),
                "".into(),
                "Delete=DEL  Insert=LINEFEED".into(),
                "ESC: Close window".into(),
            ];
            let refs: Vec<&str> = help_lines.iter().map(|s| s.as_str()).collect();
            renderer.render_overlay(&refs, 2);
        }
        if show_status {
            let (la, lb) = floppy_drive_labels;
            let speed_str = match clock_mhz {
                Some(mhz) => format!("{:.1} MHz", mhz),
                None => "unlimited".to_string(),
            };
            let status_lines: Vec<String> = vec![
                format!("{}: {}", la, machine.floppy_controller.media_a().info()),
                format!("{}: {}", lb, machine.floppy_controller.media_b().info()),
                format!("CPU speed: {}", speed_str),
            ];
            let start = if show_help { 14 } else { 2 };
            let refs: Vec<&str> = status_lines.iter().map(|s| s.as_str()).collect();
            renderer.render_overlay(&refs, start);
        }
        if let Some(ref buf) = speed_input {
            let current = match clock_mhz {
                Some(mhz) => format!("{:.1}", mhz),
                None => "-1".to_string(),
            };
            let prompt = format!("CPU speed MHz (1-100, -1=unlimited) [{}]", current);
            let input_line = format!("> {}_", buf);
            let lines = [prompt.as_str(), input_line.as_str(), "", "Enter=confirm  ESC=cancel"];
            let start = if show_help || show_status { 19 } else { 9 };
            renderer.render_overlay(&lines, start);
        }

        let (dw, dh) = renderer.display_size();
        let buffer = renderer.render_to_display_buffer_only();
        window.update_with_buffer(buffer, dw, dh)
            .unwrap_or_else(|e| eprintln!("Display error: {}", e));

        // ESC: dismiss overlays/input first, otherwise send ESC to CP/M.
        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            if speed_input.is_some() {
                speed_input = None;
            } else if show_help || show_status {
                show_help = false;
                show_status = false;
            } else {
                machine.keyboard.gui_key_queue.push(0x1b);
            }
        }

        // Poll keyboard from minifb window
        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);

        if let Some(ref mut buf) = speed_input {
            // Speed input mode: capture keys into the input buffer
            for key in window.get_keys_pressed(KeyRepeat::Yes) {
                if let Some(digit) = minifb_key_digit(key) {
                    buf.push((b'0' + digit) as char);
                } else {
                    match key {
                        Key::Period => buf.push('.'),
                        Key::Minus  => buf.push('-'),
                        Key::Backspace => { buf.pop(); },
                        Key::Enter => {
                            let input = buf.trim().to_string();
                            if input.is_empty() {
                                // Keep current setting
                            } else if let Ok(mhz) = input.parse::<f64>() {
                                if mhz < 0.0 {
                                    clock_mhz = None;
                                } else if mhz >= 1.0 && mhz <= 100.0 {
                                    let rounded = (mhz * 2.0).round() / 2.0;
                                    clock_mhz = Some(rounded);
                                    speed_start_time = Instant::now();
                                    cycle_count = 0;
                                }
                            }
                            let speed_str = match clock_mhz {
                                Some(mhz) => format!("{:.1} MHz", mhz),
                                None => "unlimited".to_string(),
                            };
                            window.set_title(&format!("izkaypro — {} — Speed: {}", config.get_display_name(), speed_str));
                            // Can't set speed_input = None here (borrowed),
                            // so mark for clearing below.
                            break;
                        },
                        _ => {}
                    }
                }
            }
            // Clear input mode if Enter was pressed (check via Enter key state)
            if window.is_key_pressed(Key::Enter, KeyRepeat::No) {
                speed_input = None;
            }
        } else {
            // Normal mode: keys go to the emulator
            // Character keys (with auto-repeat)
            for key in window.get_keys_pressed(KeyRepeat::Yes) {
                let byte: Option<u8> = if let Some(base) = minifb_key_letter(key) {
                    if ctrl {
                        Some(base + 1) // Ctrl+A=0x01 .. Ctrl+Z=0x1A
                    } else if shift {
                        Some(base + b'A')
                    } else {
                        Some(base + b'a')
                    }
                } else if let Some(digit) = minifb_key_digit(key) {
                    if shift {
                        Some(match digit {
                            0 => b')', 1 => b'!', 2 => b'@', 3 => b'#',
                            4 => b'$', 5 => b'%', 6 => b'^', 7 => b'&',
                            8 => b'*', 9 => b'(', _ => unreachable!(),
                        })
                    } else {
                        Some(b'0' + digit)
                    }
                } else {
                    match key {
                        Key::Enter     => Some(0x0D),
                        Key::Backspace => Some(0x08),
                        Key::Tab       => Some(0x09),
                        Key::Space     => Some(0x20),
                        Key::Delete    => Some(0x7F),
                        Key::Insert    => Some(0x0A),
                        Key::Up        => Some(0xF1),
                        Key::Down      => Some(0xF2),
                        Key::Left      => Some(0xF3),
                        Key::Right     => Some(0xF4),
                        Key::Comma        => Some(if shift { b'<' } else { b',' }),
                        Key::Period       => Some(if shift { b'>' } else { b'.' }),
                        Key::Slash        => Some(if shift { b'?' } else { b'/' }),
                        Key::Semicolon    => Some(if shift { b':' } else { b';' }),
                        Key::Apostrophe   => Some(if shift { b'"' } else { b'\'' }),
                        Key::LeftBracket  => Some(if shift { b'{' } else { b'[' }),
                        Key::RightBracket => Some(if shift { b'}' } else { b']' }),
                        Key::Backslash    => Some(if shift { b'|' } else { b'\\' }),
                        Key::Minus        => Some(if shift { b'_' } else { b'-' }),
                        Key::Equal        => Some(if shift { b'+' } else { b'=' }),
                        Key::Backquote    => Some(if shift { b'~' } else { b'`' }),
                        _ => None,
                    }
                };
                if let Some(b) = byte {
                    machine.keyboard.gui_key_queue.push(b);
                }
            }

            // Command keys (no auto-repeat)
            for key in window.get_keys_pressed(KeyRepeat::No) {
                match key {
                    Key::F1 => machine.keyboard.gui_command_queue.push(Command::Help),
                    Key::F2 => machine.keyboard.gui_command_queue.push(Command::ShowStatus),
                    Key::F4 => machine.keyboard.gui_command_queue.push(Command::Quit),
                    Key::F7 => machine.keyboard.gui_command_queue.push(Command::SaveMemory),
                    Key::F8 => machine.keyboard.gui_command_queue.push(Command::TraceCPU),
                    Key::F9 => machine.keyboard.gui_command_queue.push(Command::SetSpeed),
                    _ => {}
                }
            }

            // F5/F6: detect press, but defer the command until the key is
            // released. This ensures minifb sees the key-up event BEFORE the
            // rfd modal dialog opens (which swallows it on macOS, creating a
            // phantom key-down that corrupts subsequent edge detection).
            if window.is_key_down(Key::F5) {
                prev_f5_down = true;
            } else if prev_f5_down {
                prev_f5_down = false;
                machine.keyboard.gui_command_queue.push(Command::SelectDiskA);
            }
            if window.is_key_down(Key::F6) {
                prev_f6_down = true;
            } else if prev_f6_down {
                prev_f6_down = false;
                machine.keyboard.gui_command_queue.push(Command::SelectDiskB);
            }
        }

        // Drain GUI queues into internal buffers
        machine.keyboard.consume_input();

        // Handle emulator commands
        if !machine.keyboard.commands.is_empty() {
            let commands = machine.keyboard.commands.clone();
            for command in commands {
                match command {
                    Command::Quit => {
                        machine.floppy_controller.media_selected().flush_disk();
                        if let Some(ref mut hd) = machine.hard_disk {
                            hd.flush();
                        }
                        return;
                    },
                    Command::Help => {
                        show_help = !show_help;
                    },
                    Command::ShowStatus => {
                        show_status = !show_status;
                    },
                    Command::TraceCPU => {
                        trace_cpu = !trace_cpu;
                        cpu.set_trace(trace_cpu);
                        let state = if trace_cpu { "ON" } else { "OFF" };
                        window.set_title(&format!("izkaypro — {} — CPU trace: {}", config.get_display_name(), state));
                    },
                    Command::SelectDiskA => {
                        let (la, _) = floppy_drive_labels;
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title(&format!("Select disk image for Drive {}", la))
                            .add_filter("Disk Images", &["dsk", "img", "cpm", "raw"])
                            .add_filter("All Files", &["*"])
                            .pick_file()
                        {
                            let path_str = path.to_string_lossy();
                            match machine.floppy_controller.media_a_mut().load_disk(&path_str) {
                                Ok(_) => {
                                    machine.floppy_controller.disk_in_drive = true;
                                    machine.floppy_controller.motor_on = true;
                                    if _is_kaypro10_hardware {
                                        let format = machine.floppy_controller.media_a().format;
                                        let type_byte = match format {
                                            media::MediaFormat::DsDd => 0x09,
                                            media::MediaFormat::SsDd => 0x05,
                                            _ => 0x01,
                                        };
                                        machine.poke(0xFFF6, type_byte);
                                    }
                                    let name = path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path_str.to_string());
                                    window.set_title(&format!("izkaypro — {} — {}: {}", config.get_display_name(), la, name));
                                }
                                Err(err) => {
                                    window.set_title(&format!("izkaypro — {} — Error: {}", config.get_display_name(), err));
                                }
                            }
                        }
                    },
                    Command::SelectDiskB => {
                        if _is_kaypro10_hardware {
                            window.set_title(&format!("izkaypro — {} — Kaypro 10 has only one floppy drive (C:)", config.get_display_name()));
                        } else {
                            let (_, lb) = floppy_drive_labels;
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title(&format!("Select disk image for Drive {}", lb))
                                .add_filter("Disk Images", &["dsk", "img", "cpm", "raw"])
                                .add_filter("All Files", &["*"])
                                .pick_file()
                            {
                                let path_str = path.to_string_lossy();
                                match machine.floppy_controller.media_b_mut().load_disk(&path_str) {
                                    Ok(_) => {
                                        let name = path.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| path_str.to_string());
                                        window.set_title(&format!("izkaypro — {} — {}: {}", config.get_display_name(), lb, name));
                                    }
                                    Err(err) => {
                                        window.set_title(&format!("izkaypro — {} — Error: {}", config.get_display_name(), err));
                                    }
                                }
                            }
                        }
                    },
                    Command::SaveMemory => {
                        match machine.save_bios() {
                            Ok(filename) => {
                                window.set_title(&format!("izkaypro — {} — BIOS saved as {}", config.get_display_name(), filename));
                            }
                            Err(err) => {
                                window.set_title(&format!("izkaypro — {} — Error: {}", config.get_display_name(), err));
                            }
                        }
                    },
                    Command::SetSpeed => {
                        speed_input = Some(String::new());
                    },
                }
            }
            machine.keyboard.commands.clear();
        }

        // Clear VRAM dirty flags
        if machine.video_mode == kaypro_machine::VideoMode::Sy6545Crtc {
            machine.crtc.vram_dirty = false;
        } else {
            machine.vram_dirty = false;
        }
    }

    // Clean shutdown
    machine.floppy_controller.media_selected().flush_disk();
    if let Some(ref mut hd) = machine.hard_disk {
        hd.flush();
    }
}

#[cfg(feature = "gui")]
fn minifb_key_letter(key: minifb::Key) -> Option<u8> {
    use minifb::Key;
    match key {
        Key::A => Some(0), Key::B => Some(1), Key::C => Some(2), Key::D => Some(3),
        Key::E => Some(4), Key::F => Some(5), Key::G => Some(6), Key::H => Some(7),
        Key::I => Some(8), Key::J => Some(9), Key::K => Some(10), Key::L => Some(11),
        Key::M => Some(12), Key::N => Some(13), Key::O => Some(14), Key::P => Some(15),
        Key::Q => Some(16), Key::R => Some(17), Key::S => Some(18), Key::T => Some(19),
        Key::U => Some(20), Key::V => Some(21), Key::W => Some(22), Key::X => Some(23),
        Key::Y => Some(24), Key::Z => Some(25),
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn minifb_key_digit(key: minifb::Key) -> Option<u8> {
    use minifb::Key;
    match key {
        Key::Key0 => Some(0), Key::Key1 => Some(1), Key::Key2 => Some(2),
        Key::Key3 => Some(3), Key::Key4 => Some(4), Key::Key5 => Some(5),
        Key::Key6 => Some(6), Key::Key7 => Some(7), Key::Key8 => Some(8),
        Key::Key9 => Some(9),
        _ => None,
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
