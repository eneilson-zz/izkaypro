use clap::Parser;
use iz80::*;
use std::fs::{OpenOptions, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant};

mod config;
mod kaypro_machine;
mod floppy_controller;
mod keyboard_unix;
mod media;
mod hard_disk_image;
mod wd1002;
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
use self::keyboard_unix::Command;

#[derive(Parser)]
#[command(
    name = "izkaypro",
    about = "Kaypro computer emulator for the terminal",
    long_about = "izkaypro - Kaypro Emulator\n\
        https://github.com/ivanizag/izkaypro\n\n\
        Emulates Kaypro II, 4/83, 2X/4/84, TurboROM, KayPLUS, and Kaypro 10 computers.\n\
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

    /// Hard disk image file for Kaypro 10
    #[arg(long, value_name = "FILE")]
    hd: Option<String>,

    /// Seed Kaypro 10 HD image from the default boot floppy (explicit action)
    #[arg(long)]
    seed_k10_hd: bool,

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

    /// Trace WD1002 hard disk controller register/command activity
    #[arg(long)]
    wd_trace: bool,

    /// Trace Kaypro10 BIOS extended dispatch (D=function at ROM 0x0077)
    #[arg(long)]
    k10_bios_trace: bool,

    /// Kaypro10 BIOS trace log file path (used with --k10-bios-trace)
    #[arg(long, value_name = "FILE")]
    k10_bios_trace_log: Option<String>,

    /// WD1002 trace log file path (used with --wd-trace or --trace-all)
    #[arg(long, value_name = "FILE")]
    wd_trace_log: Option<String>,

    /// Guided Kaypro10 disk trace (concise BIOS/map events for HDFMT/PUTSYSU debugging)
    #[arg(long)]
    k10_guided_trace: bool,

    /// Guided Kaypro10 trace log file path (used with --k10-guided-trace)
    #[arg(long, value_name = "FILE")]
    k10_guided_trace_log: Option<String>,

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
        cli.hd.as_deref(),
    );

    let welcome = format!(
        "izkaypro - Kaypro Emulator\nhttps://github.com/ivanizag/izkaypro\nConfiguration: {}",
        config.get_description()
    );

    let disk_a_path = config.disk_a.clone()
        .unwrap_or_else(|| config.get_default_disk_a().to_string());
    let disk_b_path = config.disk_b.clone()
        .unwrap_or_else(|| config.get_default_disk_b().to_string());

    if config.model == KayproModel::Kaypro10 {
        let hd_path = config.get_hard_disk_path();
        if let Err(e) = hard_disk_image::ensure_exists(hd_path) {
            eprintln!("Error: Failed to prepare hard disk image '{}': {}", hd_path, e);
            std::process::exit(1);
        }
        let needs_seed = cli.seed_k10_hd
            || hard_disk_image::is_kaypro10_bootable(hd_path)
                .map(|ok| !ok)
                .unwrap_or(true);
        if needs_seed {
            if let Err(e) =
                hard_disk_image::seed_kaypro10_from_floppy(hd_path, "disks/system/k10u-rld.img")
            {
                eprintln!(
                    "Warning: Failed to seed Kaypro10 hard disk '{}' from floppy image: {}",
                    hd_path, e
                );
            }
        }
    }

    let mut trace_cpu = cli.cpu_trace || cli.trace_all;
    let trace_io = cli.io_trace || cli.trace_all;
    let trace_fdc = cli.fdc_trace || cli.trace_all;
    let trace_fdc_rw = cli.fdc_trace_rw || cli.trace_all;
    let trace_system_bits = cli.system_bits || cli.trace_all;
    let trace_rom = cli.rom_trace || cli.trace_all;
    let trace_bdos = cli.bdos_trace || cli.trace_all;
    let trace_crtc = cli.crtc_trace || cli.trace_all;
    let trace_sio = cli.sio_trace || cli.trace_all;
    let trace_rtc = cli.rtc_trace || cli.trace_all;
    let trace_wd = cli.wd_trace || cli.trace_all;
    let trace_k10_bios = cli.k10_bios_trace;
    let trace_k10_guided = cli.k10_guided_trace;
    let run_diag = cli.diagnostics;
    let run_boot_test = cli.boot_test;

    let any_trace = trace_io
        || trace_cpu
        || trace_fdc
        || trace_fdc_rw
        || trace_rom
        || trace_bdos
        || trace_crtc
        || trace_sio
        || trace_rtc
        || trace_wd
        || trace_k10_bios
        || trace_k10_guided
        || trace_system_bits;

    let mut k10_bios_log: Option<BufWriter<std::fs::File>> = if trace_k10_bios && config.model == KayproModel::Kaypro10 {
        let log_path = cli.k10_bios_trace_log.as_deref().unwrap_or("logs/k10bios.log");
        if let Some(parent) = Path::new(log_path).parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = create_dir_all(parent) {
                    eprintln!("Warning: failed to create bios trace log dir '{}': {}", parent.display(), e);
                }
            }
        }
        match OpenOptions::new().create(true).truncate(true).write(true).open(log_path) {
            Ok(f) => Some(BufWriter::new(f)),
            Err(e) => {
                eprintln!("Warning: failed to open bios trace log '{}': {}", log_path, e);
                None
            }
        }
    } else {
        None
    };

    let mut k10_guided_log: Option<BufWriter<std::fs::File>> =
        if trace_k10_guided && config.model == KayproModel::Kaypro10 {
            let log_path = cli
                .k10_guided_trace_log
                .as_deref()
                .unwrap_or("logs/k10_guided.log");
            if let Some(parent) = Path::new(log_path).parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(e) = create_dir_all(parent) {
                        eprintln!(
                            "Warning: failed to create guided trace log dir '{}': {}",
                            parent.display(),
                            e
                        );
                    }
                }
            }
            match OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(log_path)
            {
                Ok(f) => Some(BufWriter::new(f)),
                Err(e) => {
                    eprintln!("Warning: failed to open guided trace log '{}': {}", log_path, e);
                    None
                }
            }
        } else {
            None
        };

    // Init device with configuration
    let floppy_controller = FloppyController::new(
        &disk_a_path,
        &disk_b_path,
        config.get_disk_format(),
        config.get_side1_sector_base(),
        trace_fdc,
        trace_fdc_rw,
    );
    let mut screen = Screen::new(!any_trace, config.get_display_name());
    let mut machine = KayproMachine::new(
        config.get_rom_path(),
        config.get_video_mode(),
        floppy_controller,
        trace_io,
        trace_system_bits,
        trace_crtc,
        trace_sio,
        trace_rtc,
        trace_wd,
        cli.wd_trace_log.as_deref(),
        config.model == KayproModel::Kaypro10,
        Some(config.get_hard_disk_path()),
    );
    machine.kayplus_clock_fixup = config.model == KayproModel::KayPlus84;
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
    let mut k10_bios_pending: Vec<(u16, u16, u8, u8, u16, u16)> = Vec::new(); // (ret_pc, entry_sp, d, a, bc, de)
    let mut k10_last_map: Option<(u8, u8, u8, u8)> = None; // (FFF4, FFF5, FFF6, FD8C)
    let trace_k10_any = trace_k10_bios || trace_k10_guided;
    while !done {

        if trace_k10_any && machine.kaypro10_mode && machine.is_rom_rank() {
            let regs = cpu.registers();
            let pc_now = regs.pc();
            let sp_now = regs.get16(Reg16::SP);
            let map_now = (
                machine.peek(0xFFF4),
                machine.peek(0xFFF5),
                machine.peek(0xFFF6),
                machine.peek(0xFD8C),
            );
            if let Some(prev) = k10_last_map {
                if prev != map_now {
                    if let Some(log) = k10_bios_log.as_mut() {
                        let _ = writeln!(
                            log,
                            "K10MAP CHG pc={:04X} ctr={} FFF4 {:02X}->{:02X} FFF5 {:02X}->{:02X} FFF6 {:02X}->{:02X} FD8C {:02X}->{:02X}",
                            pc_now,
                            counter,
                            prev.0,
                            map_now.0,
                            prev.1,
                            map_now.1,
                            prev.2,
                            map_now.2,
                            prev.3,
                            map_now.3
                        );
                        let _ = log.flush();
                    }
                    if let Some(log) = k10_guided_log.as_mut() {
                        let _ = writeln!(
                            log,
                            "MAP pc={:04X} ctr={} A:{:02X}->{:02X} B:{:02X}->{:02X} C:{:02X}->{:02X} HDSEL:{:02X}->{:02X}",
                            pc_now,
                            counter,
                            prev.0,
                            map_now.0,
                            prev.1,
                            map_now.1,
                            prev.2,
                            map_now.2,
                            prev.3,
                            map_now.3
                        );
                        let _ = log.flush();
                    }
                }
            }
            k10_last_map = Some(map_now);
            if pc_now == 0x0077 {
                let d = regs.get8(Reg8::D);
                let a = regs.get8(Reg8::A);
                let bc = regs.get16(Reg16::BC);
                let de = regs.get16(Reg16::DE);
                let ret_pc = machine.peek(sp_now) as u16 | ((machine.peek(sp_now.wrapping_add(1)) as u16) << 8);
                if is_k10_disk_fn(d) {
                    k10_bios_pending.push((ret_pc, sp_now, d, a, bc, de));
                    if let Some(log) = k10_bios_log.as_mut() {
                        let drv_a = machine.peek(0xFFF4);
                        let drv_b = machine.peek(0xFFF5);
                        let drv_c = machine.peek(0xFFF6);
                        let active = machine.peek(0xFD8C);
                        let _ = writeln!(
                            log,
                            "K10BIOS CALL pc=0077 fn={:02X}({}) a={:02X} bc={:04X} de={:04X} sp={:04X} ret={:04X} map[FFF4={:02X} FFF5={:02X} FFF6={:02X} FD8C={:02X}]",
                            d,
                            k10_bios_fn_name(d),
                            a,
                            bc,
                            de,
                            sp_now,
                            ret_pc,
                            drv_a,
                            drv_b,
                            drv_c,
                            active
                        );
                        let _ = log.flush();
                    }
                    if is_k10_guided_fn(d) {
                        if let Some(log) = k10_guided_log.as_mut() {
                            let _ = writeln!(
                                log,
                                "CALL fn={:02X}({}) a={:02X} bc={:04X} de={:04X} sp={:04X} ret={:04X} map={:02X}/{:02X}/{:02X} hdsel={:02X}",
                                d,
                                k10_bios_fn_name(d),
                                a,
                                bc,
                                de,
                                sp_now,
                                ret_pc,
                                map_now.0,
                                map_now.1,
                                map_now.2,
                                map_now.3
                            );
                            let _ = log.flush();
                        }
                    }
                }
            }
            if let Some((ret_pc, entry_sp, d, a, bc, de)) = k10_bios_pending.last().copied() {
                if pc_now == ret_pc && sp_now == entry_sp.wrapping_add(2) {
                    let hl = regs.get16(Reg16::HL);
                    let a_out = regs.get8(Reg8::A);
                    if let Some(log) = k10_bios_log.as_mut() {
                        let drv_a = machine.peek(0xFFF4);
                        let drv_b = machine.peek(0xFFF5);
                        let drv_c = machine.peek(0xFFF6);
                        let active = machine.peek(0xFD8C);
                        let _ = writeln!(
                            log,
                            "K10BIOS RET  pc={:04X} fn={:02X}({}) in[a={:02X} bc={:04X} de={:04X}] out[a={:02X} hl={:04X}] sp={:04X} map[FFF4={:02X} FFF5={:02X} FFF6={:02X} FD8C={:02X}]",
                            ret_pc,
                            d,
                            k10_bios_fn_name(d),
                            a,
                            bc,
                            de,
                            a_out,
                            hl,
                            sp_now,
                            drv_a,
                            drv_b,
                            drv_c,
                            active
                        );
                        let _ = log.flush();
                    }
                    if is_k10_guided_fn(d) {
                        if let Some(log) = k10_guided_log.as_mut() {
                            let status = if a_out == 0 { "OK" } else { "ERR" };
                            let _ = writeln!(
                                log,
                                "RET  fn={:02X}({}) {} in[a={:02X} bc={:04X} de={:04X}] out[a={:02X} hl={:04X}] map={:02X}/{:02X}/{:02X} hdsel={:02X}",
                                d,
                                k10_bios_fn_name(d),
                                status,
                                a,
                                bc,
                                de,
                                a_out,
                                hl,
                                machine.peek(0xFFF4),
                                machine.peek(0xFFF5),
                                machine.peek(0xFFF6),
                                machine.peek(0xFD8C)
                            );
                            let _ = log.flush();
                        }
                    }
                    k10_bios_pending.pop();
                }
            }
        }

        cpu.execute_instruction(&mut machine);
        counter += 1;

        if machine.kaypro10_mode {
            if let Some(wd) = machine.wd1002.as_mut() {
                wd.step();
            }
        }
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
                        done = true;
                    },
                    Command::Help => {
                        screen.show_help = !screen.show_help;
                    },
                    Command::ShowStatus => {
                        screen.show_status = !screen.show_status;
                    },
                    Command::SelectDiskA => {
                        if let Some(path) = screen.prompt(&mut machine, "File to load in Drive A") {
                            let res = machine.floppy_controller.media_a_mut().load_disk(path.as_str());
                            if let Err(err) = res {
                                screen.message(&mut machine, &err.to_string())
                            }
                        }
                    }
                    Command::SelectDiskB => {
                        if let Some(path) = screen.prompt(&mut machine, "File to load in Drive B") {
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
        if machine.kaypro10_mode {
            if let Some(wd) = machine.wd1002.as_mut() {
                if wd.take_intrq() {
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
            screen.update(&mut machine, true);
            println!("HALT instruction that will never be interrupted");
            break;
        }

        // Tracing for ROM 81-149c
        /*
        if trace_rom && machine.is_rom_rank(){
            let dma = machine.peek16(0xfc14);
            match cpu.registers().pc() {
                0x004b => println!("EP_COLD"),
                0x0186 => println!("EP_INITDSK"),
                0x0006 => println!("EP_INITVID"),
                0x0009 => println!("EP_INITDEV"),
                0x01d8 => println!("EP_HOME"),
                0x01b4 => println!("EP_SELDSK {}", cpu.registers().get8(Reg8::C)),
                0x01cc => println!("EP_SETTRK {}", cpu.registers().get8(Reg8::C)),
                0x01bb => println!("EP_SETSEC {}", cpu.registers().get8(Reg8::C)),
                0x01c7 => println!("EP_SETDMA"),
                0x01ec => println!("EP_READ {:04x}", dma),
                0x0207 => println!("EP_WRITE {:04x}", dma),
                0x03e4 => println!("EP_SECTRAN"),
                0x040f => println!("EP_DISKON"),
                0x041e => println!("EP_DISKOFF"),
                0xfa00 => println!("FUNC: OS start"),
                _ => {}
            }
        }
        */

        // Tracing for ROM 81-232
        if trace_rom && machine.is_rom_rank(){
            let dma = machine.peek16(0xfc14);
            match cpu.registers().pc() {
                0x004b => println!("EP_COLD"),
                0x0195 => println!("EP_INITDSK"),
                0x0006 => println!("EP_INITVID"),
                0x0009 => println!("EP_INITDEV"),
                0x01e7 => println!("EP_HOME"),
                0x01c3 => println!("EP_SELDSK {}", cpu.registers().get8(Reg8::C)),
                0x01db => println!("EP_SETTRK {}", cpu.registers().get8(Reg8::C)),
                0x01ca => println!("EP_SETSEC {}", cpu.registers().get8(Reg8::C)),
                0x01d6 => println!("EP_SETDMA"),
                0x01fb => println!("EP_READ {:04x}", dma),
                0x0216 => println!("EP_WRITE {:04x}", dma),
                0x0479 => println!("EP_SECTRAN"),
                0x04a2 => println!("EP_DISKON"),
                0x04b1 => println!("EP_DISKOFF"),
                0xfa00 => println!("FUNC: OS start"),
                _ => {}
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

                println!("BDOS command {}: {}({:04x})", command, name, args);
            }
        }
    }
}

fn k10_bios_fn_name(d: u8) -> &'static str {
    match d {
        0x20 => "SELDSK",
        0x21 => "SETTRK",
        0x22 => "SETSEC",
        0x23 => "SETDMA",
        0x24 => "RWPREP",
        0x25 => "FLUSH?",
        0x26 => "READ",
        0x27 => "WRITE",
        0x28 => "HOME",
        0x29 => "COPYBBT",
        0x2A => "UNKNOWN2A",
        0x2B => "UNKNOWN2B",
        0x2C => "UNKNOWN2C",
        0x2D => "UNKNOWN2D",
        0x2E => "READ2",
        0x2F => "WRITE2",
        _ => "OTHER",
    }
}

fn is_k10_disk_fn(d: u8) -> bool {
    (0x20..=0x2F).contains(&d)
}

fn is_k10_guided_fn(d: u8) -> bool {
    matches!(d, 0x20 | 0x21 | 0x22 | 0x23 | 0x24 | 0x26 | 0x27 | 0x28 | 0x29 | 0x2E | 0x2F)
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
