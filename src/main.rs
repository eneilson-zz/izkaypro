use clap::Parser;
use iz80::*;
use std::time::{Duration, Instant};

mod config;
mod kaypro_machine;
mod floppy_controller;
mod keyboard_unix;
mod media;
mod screen;
mod sy6545;
mod diagnostics;
#[cfg(test)]
mod format_test;

use self::config::Config;
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
        Emulates Kaypro II, 4/83, 2X/4/84, TurboROM, and KayPLUS computers.\n\
        Configuration is loaded from izkaypro.toml; command-line arguments override config file settings.",
    version,
)]
struct Cli {
    /// Kaypro model preset [models: kaypro_ii, kaypro4_83, kaypro4_84, turbo_rom, kayplus_84, custom]
    #[arg(short = 'm', long, value_name = "MODEL")]
    model: Option<String>,

    /// Disk image file for drive A
    #[arg(short = 'a', long, value_name = "FILE")]
    drivea: Option<String>,

    /// Disk image file for drive B
    #[arg(short = 'b', long, value_name = "FILE")]
    driveb: Option<String>,

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
    );

    let welcome = format!(
        "izkaypro - Kaypro Emulator\nhttps://github.com/ivanizag/izkaypro\nConfiguration: {}",
        config.get_description()
    );

    let disk_a_path = config.disk_a.clone()
        .unwrap_or_else(|| config.get_default_disk_a().to_string());
    let disk_b_path = config.disk_b.clone()
        .unwrap_or_else(|| config.get_default_disk_b().to_string());

    let mut trace_cpu = cli.cpu_trace || cli.trace_all;
    let trace_io = cli.io_trace || cli.trace_all;
    let trace_fdc = cli.fdc_trace || cli.trace_all;
    let trace_fdc_rw = cli.fdc_trace_rw || cli.trace_all;
    let trace_system_bits = cli.system_bits || cli.trace_all;
    let trace_rom = cli.rom_trace || cli.trace_all;
    let trace_bdos = cli.bdos_trace || cli.trace_all;
    let trace_crtc = cli.crtc_trace || cli.trace_all;
    let run_diag = cli.diagnostics;
    let run_boot_test = cli.boot_test;

    let any_trace = trace_io
        || trace_cpu
        || trace_fdc
        || trace_fdc_rw
        || trace_rom
        || trace_bdos
        || trace_crtc
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
    let mut screen = Screen::new(!any_trace, config.get_display_name());
    let mut machine = KayproMachine::new(
        config.get_rom_path(),
        config.get_video_mode(),
        floppy_controller,
        trace_io,
        trace_system_bits,
        trace_crtc,
    );
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
    while !done {

        cpu.execute_instruction(&mut machine);
        counter += 1;
        cycle_count += CYCLES_PER_INSTRUCTION;

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
