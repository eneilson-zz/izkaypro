use clap::{Arg, App};
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

use self::config::Config;
use self::kaypro_machine::KayproMachine;
use self::floppy_controller::FloppyController;
use self::screen::Screen;
use self::keyboard_unix::Command;


fn main() {
    // Load configuration from file (or use defaults)
    let config = Config::load();
    let welcome = format!(
        "izkaypro - Kaypro Emulator\nhttps://github.com/ivanizag/izkaypro\nConfiguration: {}",
        config.get_description()
    );
    
    // Parse arguments
    let matches = App::new(&welcome[..])
        .arg(Arg::with_name("DISKA")
            .help("Disk A: image file. Empty or $ to use config default")
            .required(false)
            .index(1))
        .arg(Arg::with_name("DISKB")
            .help("Disk B: image file. Empty to use config default")
            .required(false)
            .index(2))
        .arg(Arg::with_name("cpu_trace")
            .short("c")
            .long("cpu-trace")
            .help("Traces CPU instructions execuions"))
        .arg(Arg::with_name("io_trace")
            .short("i")
            .long("io-trace")
            .help("Traces ports IN and OUT"))
        .arg(Arg::with_name("fdc_trace")
            .short("f")
            .long("fdc-trace")
            .help("Traces access to the floppy disk controller"))
        .arg(Arg::with_name("fdc_trace_rw")
            .short("w")
            .long("fdc-trace-rw")
            .help("Traces RW access to the floppy disk controller"))
        .arg(Arg::with_name("system_bits")
            .short("s")
            .long("system-bits")
            .help("Traces changes to the system bits values"))
        .arg(Arg::with_name("rom_trace")
            .short("r")
            .long("rom-trace")
            .help("Traces calls to the ROM entrypoints"))
        .arg(Arg::with_name("bdos_trace")
            .short("b")
            .long("bdos-trace")
            .help("Traces calls to the CP/M BDOS entrypoints"))
        .arg(Arg::with_name("crtc_trace")
            .short("v")
            .long("crtc-trace")
            .help("Traces SY6545 CRTC VRAM writes"))
        .arg(Arg::with_name("run_diag")
            .short("d")
            .long("diagnostics")
            .help("Run ROM and RAM diagnostics then exit"))
        .get_matches();

    // Command line disk overrides (or use config defaults)
    let disk_a_path = matches.value_of("DISKA")
        .filter(|s| *s != "$")
        .map(|s| s.to_string())
        .or_else(|| config.disk_a.clone())
        .unwrap_or_else(|| config.get_default_disk_a().to_string());
    
    let disk_b_path = matches.value_of("DISKB")
        .map(|s| s.to_string())
        .or_else(|| config.disk_b.clone())
        .unwrap_or_else(|| config.get_default_disk_b().to_string());
    
    let mut trace_cpu = matches.is_present("cpu_trace");
    let trace_io = matches.is_present("io_trace");
    let trace_fdc = matches.is_present("fdc_trace");
    let trace_fdc_rw = matches.is_present("fdc_trace_rw");
    let trace_system_bits = matches.is_present("system_bits");
    let trace_rom = matches.is_present("rom_trace");
    let trace_bdos = matches.is_present("bdos_trace");
    let trace_crtc = matches.is_present("crtc_trace");
    let run_diag = matches.is_present("run_diag");

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
    let mut clock_mhz: Option<f64> = None;
    let mut cycle_count: u64 = 0;
    let mut speed_start_time = Instant::now();
    const CYCLES_PER_INSTRUCTION: u64 = 4; // Average Z80 cycles per instruction

    let mut counter: u64 = 1;
    let mut next_signal: u64 = 0;
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

        // NMI processing
        if machine.floppy_controller.raise_nmi {
            machine.floppy_controller.raise_nmi = false;
            next_signal = counter + 10_000_000;
        }
        let mut nmi_signaled = false;
        if next_signal != 0 && counter >= next_signal {
            cpu.signal_nmi();
            next_signal = 0;
            nmi_signaled = true;
        }
        if next_signal != 0 && cpu.is_halted() {
            // CPU is halted waiting for interrupt - signal NMI immediately
            cpu.signal_nmi();
            next_signal = 0;
            nmi_signaled = true;
        }
        // Only check for uninterruptible halt if we didn't just signal NMI
        // (the CPU needs at least one cycle to process the NMI and exit HALT)
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
