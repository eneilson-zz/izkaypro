use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Z84C40 SIO Channel A emulation for Kaypro 4-84 serial port.
///
/// I/O Ports (Kaypro 4-84):
/// - Port 0x04: SIO-1 Channel A Data (Tx/Rx bytes)
/// - Port 0x06: SIO-1 Channel A Control (register access)
/// - Port 0x00: 8116 Baud Rate Generator (4-bit code, lower nibble)
///
/// Register access protocol:
/// - Write to control port targets WR0 by default
/// - WR0 bits D2-D0 set a pointer for the next control write
/// - After the pointed register is written, the pointer resets to WR0
/// - Read from control port targets RR0 by default (WR0 pointer selects RR0-RR2)

const RX_FIFO_CAPACITY: usize = 3; // Real SIO has 3-byte FIFO

pub struct Sio {
    // Write registers
    wr: [u8; 6],        // WR0-WR5
    reg_pointer: u8,    // Next register to write (from WR0 D2-D0)

    // Receive FIFO — shared with reader thread
    rx_fifo: Arc<Mutex<VecDeque<u8>>>,

    // Error flags (RR1 bits, latched until Error Reset command)
    rx_overrun: bool,

    // Transmit state
    tx_ready_at: Instant,
    tx_file: Option<std::fs::File>,

    // Serial device file descriptor for modem control ioctls (Unix only)
    #[cfg(unix)]
    serial_fd: Option<i32>,
    device_name: String,

    // 8116 baud rate generator
    baud_rate_code: u8,
    baud_rate: u32,

    pub trace: bool,
}

impl Sio {
    pub fn new(trace: bool) -> Sio {
        Sio {
            wr: [0; 6],
            reg_pointer: 0,
            rx_fifo: Arc::new(Mutex::new(VecDeque::with_capacity(64))),
            rx_overrun: false,
            tx_ready_at: Instant::now(),
            tx_file: None,
            #[cfg(unix)]
            serial_fd: None,
            device_name: String::new(),
            baud_rate_code: 0x0E, // Default 9600
            baud_rate: 9600,
            trace,
        }
    }

    /// Open a serial device and start the background reader thread.
    /// The device path can be a real serial port (/dev/ttyUSB0) or a
    /// pty endpoint (/tmp/kayproA created by socat, etc.).
    #[cfg(unix)]
    pub fn open_serial(&mut self, device_path: &str) -> Result<(), String> {
        use std::os::unix::io::{AsRawFd, FromRawFd};

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(device_path)
            .map_err(|e| format!("Failed to open serial device '{}': {}", device_path, e))?;

        let fd = file.as_raw_fd();

        // Configure termios for raw mode (no echo, no buffering, no signal handling)
        if let Ok(mut termios) = termios::Termios::from_fd(fd) {
            termios.c_iflag &= !(termios::IXON | termios::IXOFF | termios::ICRNL
                | termios::INLCR | termios::IGNCR | termios::ISTRIP | termios::BRKINT);
            termios.c_oflag &= !termios::OPOST;
            termios.c_lflag &= !(termios::ECHO | termios::ICANON | termios::ISIG | termios::IEXTEN);
            termios.c_cflag |= termios::CS8 | termios::CREAD | termios::CLOCAL;
            termios.c_cc[termios::VMIN] = 0;
            termios.c_cc[termios::VTIME] = 1; // 100ms timeout for reads
            let _ = termios::tcsetattr(fd, termios::TCSANOW, &termios);
        }

        // Clone the file descriptor for the reader thread
        let reader_fd = unsafe { libc::dup(fd) };
        if reader_fd < 0 {
            return Err("Failed to duplicate file descriptor".to_string());
        }
        let reader_file = unsafe { std::fs::File::from_raw_fd(reader_fd) };

        self.serial_fd = Some(fd);
        self.device_name = device_path.to_string();

        // Set initial modem control lines (RTS + DTR asserted)
        self.update_modem_signals();

        // Spawn background reader thread
        let rx_fifo = Arc::clone(&self.rx_fifo);
        let trace = self.trace;
        std::thread::spawn(move || {
            use std::io::Read;
            let mut reader = reader_file;
            let mut buf = [0u8; 64];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Ok(n) => {
                        if let Ok(mut fifo) = rx_fifo.lock() {
                            for &byte in &buf[..n] {
                                fifo.push_back(byte);
                                if trace {
                                    println!("SIO A: Serial Rx 0x{:02X} '{}'", byte,
                                        if byte >= 0x20 && byte < 0x7F { byte as char } else { '.' });
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });

        self.tx_file = Some(file);

        if self.trace {
            println!("SIO A: Opened serial device '{}'", device_path);
        }
        Ok(())
    }

    /// Serial port support is not yet available on Windows.
    #[cfg(windows)]
    pub fn open_serial(&mut self, device_path: &str) -> Result<(), String> {
        Err(format!("Serial port '{}' not supported on Windows yet", device_path))
    }

    /// Write to the control port (port 0x06).
    /// Implements the WR0 pointer protocol and command dispatch.
    pub fn write_control(&mut self, value: u8) {
        let reg = self.reg_pointer;
        match reg {
            0 => {
                // WR0: pointer (D2-D0) and command (D5-D3)
                self.reg_pointer = value & 0x07;
                let cmd = (value >> 3) & 0x07;
                match cmd {
                    0 => {} // Null command
                    2 => {  // Reset Ext/Status Interrupts
                        if self.trace {
                            println!("SIO A: Reset Ext/Status Interrupts");
                        }
                    }
                    3 => {  // Channel Reset
                        self.channel_reset();
                    }
                    4 => {  // Enable INT on Next Rx Character
                        if self.trace {
                            println!("SIO A: Enable INT on Next Rx");
                        }
                    }
                    5 => {  // Reset Tx INT Pending
                        if self.trace {
                            println!("SIO A: Reset Tx INT Pending");
                        }
                    }
                    6 => {  // Error Reset
                        self.rx_overrun = false;
                        if self.trace {
                            println!("SIO A: Error Reset");
                        }
                    }
                    7 => {  // Return from INT (Channel A only)
                        if self.trace {
                            println!("SIO A: Return from INT");
                        }
                    }
                    _ => {}
                }
                if self.trace && cmd != 0 {
                    println!("SIO A: WR0 cmd={} ptr={}", cmd, self.reg_pointer);
                }
            }
            1 => {
                self.wr[1] = value;
                self.reg_pointer = 0;
                if self.trace {
                    let rx_mode = (value >> 3) & 0x03;
                    println!("SIO A: WR1=0x{:02X} (ExtInt={}, TxInt={}, RxMode={})",
                        value, value & 0x01, (value >> 1) & 0x01, rx_mode);
                }
            }
            2 => {
                // WR2: Interrupt vector (Channel B only on real hardware,
                // but accept writes silently for compatibility)
                self.reg_pointer = 0;
                if self.trace {
                    println!("SIO A: WR2=0x{:02X} (ignored, Ch B only)", value);
                }
            }
            3 => {
                self.wr[3] = value;
                self.reg_pointer = 0;
                if self.trace {
                    let rx_bits = match (value >> 6) & 0x03 {
                        0 => 5, 1 => 7, 2 => 6, _ => 8,
                    };
                    println!("SIO A: WR3=0x{:02X} (RxEn={}, AutoEn={}, RxBits={})",
                        value, value & 0x01, (value >> 5) & 0x01, rx_bits);
                }
            }
            4 => {
                self.wr[4] = value;
                self.reg_pointer = 0;
                if self.trace {
                    let clock_mode = match (value >> 6) & 0x03 {
                        0 => "x1", 1 => "x16", 2 => "x32", _ => "x64",
                    };
                    let stop_bits = match (value >> 2) & 0x03 {
                        0 => "sync", 1 => "1", 2 => "1.5", _ => "2",
                    };
                    let parity = if value & 0x01 != 0 {
                        if value & 0x02 != 0 { "even" } else { "odd" }
                    } else {
                        "none"
                    };
                    println!("SIO A: WR4=0x{:02X} (clock={}, stop={}, parity={})",
                        value, clock_mode, stop_bits, parity);
                }
            }
            5 => {
                let old_wr5 = self.wr[5];
                self.wr[5] = value;
                self.reg_pointer = 0;

                // Detect Send Break changes
                let old_break = (old_wr5 >> 4) & 0x01;
                let new_break = (value >> 4) & 0x01;
                if new_break != old_break {
                    self.handle_break(new_break != 0);
                }

                // Detect RTS/DTR changes
                if (value ^ old_wr5) & 0x82 != 0 {
                    self.update_modem_signals();
                }

                if self.trace {
                    let tx_bits = match (value >> 5) & 0x03 {
                        0 => 5, 1 => 7, 2 => 6, _ => 8,
                    };
                    println!("SIO A: WR5=0x{:02X} (TxEn={}, RTS={}, DTR={}, Break={}, TxBits={})",
                        value,
                        (value >> 3) & 0x01,
                        (value >> 1) & 0x01,
                        (value >> 7) & 0x01,
                        (value >> 4) & 0x01,
                        tx_bits);
                }
            }
            _ => {
                // WR6, WR7: sync mode registers, accept silently
                self.reg_pointer = 0;
            }
        }
    }

    /// Read from the control port (port 0x06).
    /// Returns RR0 by default, or RR1/RR2 if selected via WR0 pointer.
    pub fn read_control(&mut self) -> u8 {
        let reg = self.reg_pointer;
        self.reg_pointer = 0;

        match reg {
            0 => self.read_rr0(),
            1 => self.read_rr1(),
            _ => {
                if self.trace {
                    println!("SIO A: Read RR{} (unimplemented, returning 0)", reg);
                }
                0
            }
        }
    }

    /// Write to the data port (port 0x04). Transmit a byte.
    pub fn write_data(&mut self, value: u8) {
        if self.trace {
            println!("SIO A: Tx 0x{:02X} '{}'", value,
                if value >= 0x20 && value < 0x7F { value as char } else { '.' });
        }

        // Calculate character time and set tx_ready_at
        let char_time_us = self.character_time_us();
        self.tx_ready_at = Instant::now() + std::time::Duration::from_micros(char_time_us);

        // Forward byte to host serial port
        if let Some(ref mut file) = self.tx_file {
            let _ = file.write_all(&[value]);
            let _ = file.flush();
        }
    }

    /// Read from the data port (port 0x04). Receive a byte.
    pub fn read_data(&mut self) -> u8 {
        let value = if let Ok(mut fifo) = self.rx_fifo.lock() {
            // Check for overrun: if FIFO exceeds hardware capacity,
            // the newest byte overwrites the oldest (per SIO datasheet)
            if fifo.len() > RX_FIFO_CAPACITY {
                self.rx_overrun = true;
                if self.trace {
                    println!("SIO A: Rx overrun (FIFO len={})", fifo.len());
                }
            }
            fifo.pop_front().unwrap_or(0)
        } else {
            0
        };
        if self.trace && value != 0 {
            println!("SIO A: Rx 0x{:02X} '{}'", value,
                if value >= 0x20 && value < 0x7F { value as char } else { '.' });
        }
        value
    }

    /// Write to the baud rate generator port (port 0x00).
    /// Accepts a 4-bit code (lower nibble) that selects the baud rate.
    pub fn set_baud_rate_code(&mut self, code: u8) {
        let code = code & 0x0F;
        self.baud_rate_code = code;
        self.baud_rate = Self::decode_baud_rate(code);
        if self.trace {
            println!("SIO A: Baud rate code 0x{:02X} = {} baud", code, self.baud_rate);
        }
    }

    pub fn is_connected(&self) -> bool {
        self.tx_file.is_some()
    }

    /// Get a short status string for the F2 display.
    pub fn status_string(&self) -> String {
        if self.is_connected() {
            format!("SIO:{} {}bd", self.device_name, self.baud_rate)
        } else {
            "SIO:---".to_string()
        }
    }

    fn channel_reset(&mut self) {
        self.wr = [0; 6];
        self.reg_pointer = 0;
        self.rx_overrun = false;
        if let Ok(mut fifo) = self.rx_fifo.lock() {
            fifo.clear();
        }
        self.tx_ready_at = Instant::now();
        if self.trace {
            println!("SIO A: Channel Reset");
        }
    }

    /// Send or clear a break condition on the serial line.
    #[cfg(unix)]
    fn handle_break(&mut self, send_break: bool) {
        if let Some(fd) = self.serial_fd {
            if send_break {
                unsafe { libc::tcsendbreak(fd, 0); }
                if self.trace {
                    println!("SIO A: Send Break asserted");
                }
            } else {
                if self.trace {
                    println!("SIO A: Send Break cleared");
                }
            }
        }
    }

    #[cfg(windows)]
    fn handle_break(&mut self, _send_break: bool) {}

    /// Update RTS and DTR modem control lines from WR5 state.
    #[cfg(unix)]
    fn update_modem_signals(&self) {
        if let Some(fd) = self.serial_fd {
            let rts = (self.wr[5] >> 1) & 0x01 != 0;
            let dtr = (self.wr[5] >> 7) & 0x01 != 0;

            let mut bits: libc::c_int = 0;
            if rts { bits |= libc::TIOCM_RTS; }
            if dtr { bits |= libc::TIOCM_DTR; }

            unsafe { libc::ioctl(fd, libc::TIOCMSET, &bits); }

            if self.trace {
                println!("SIO A: Modem signals RTS={} DTR={}", rts as u8, dtr as u8);
            }
        }
    }

    #[cfg(windows)]
    fn update_modem_signals(&self) {}

    /// Read modem status lines (CTS, DCD) from the serial device.
    #[allow(dead_code)]
    #[cfg(unix)]
    fn read_modem_signals(&self) -> (bool, bool) {
        if let Some(fd) = self.serial_fd {
            let mut bits: libc::c_int = 0;
            let ret = unsafe { libc::ioctl(fd, libc::TIOCMGET, &mut bits) };
            if ret == 0 {
                let cts = bits & libc::TIOCM_CTS != 0;
                let dcd = bits & libc::TIOCM_CD != 0;
                return (cts, dcd);
            }
        }
        (true, true)
    }

    #[allow(dead_code)]
    #[cfg(windows)]
    fn read_modem_signals(&self) -> (bool, bool) {
        (true, true)
    }

    /// Build RR0 status register.
    /// D0: Rx Char Available
    /// D1: INT Pending (Channel A only) — not used in polled mode
    /// D2: Tx Buffer Empty
    /// D3: DCD (active low on pin, but reported as 1=carrier present)
    /// D4: Sync/Hunt
    /// D5: CTS (active low on pin, but reported as 1=clear to send)
    /// D6: Tx Underrun/EOM
    /// D7: Break/Abort
    fn read_rr0(&self) -> u8 {
        let mut status: u8 = 0;

        // D0: Rx Char Available
        if let Ok(fifo) = self.rx_fifo.lock() {
            if !fifo.is_empty() {
                status |= 0x01;
            }
        }

        // D2: Tx Buffer Empty (ready to accept data)
        if Instant::now() >= self.tx_ready_at {
            status |= 0x04;
        }

        // D3: DCD, D5: CTS — default to asserted when connected.
        // Real modem signal reading (read_modem_signals) is available but
        // not called per-poll due to ioctl syscall overhead. Ptys don't
        // have modem lines anyway; real serial hardware can enable this.
        if self.is_connected() {
            status |= 0x08; // DCD
            status |= 0x20; // CTS
        }

        status
    }

    /// Build RR1 status register.
    /// D0: All Sent (Tx shift register empty)
    /// D4: Parity Error
    /// D5: Rx Overrun Error
    /// D6: Framing Error
    fn read_rr1(&self) -> u8 {
        let mut status: u8 = 0;

        // D0: All Sent — true when Tx is idle
        if Instant::now() >= self.tx_ready_at {
            status |= 0x01;
        }

        // D5: Rx Overrun Error (latched, cleared by Error Reset command)
        if self.rx_overrun {
            status |= 0x20;
        }

        status
    }

    /// Decode 8116 baud rate generator code to actual baud rate.
    fn decode_baud_rate(code: u8) -> u32 {
        match code {
            0x00 => 50,
            0x01 => 75,
            0x02 => 110,
            0x03 => 135,
            0x04 => 150,
            0x05 => 300,
            0x06 => 600,
            0x07 => 1200,
            0x08 => 1800,
            0x09 => 2000,
            0x0A => 2400,
            0x0B => 3600,
            0x0C => 4800,
            0x0D => 7200,
            0x0E => 9600,
            0x0F => 19200,
            _ => 9600,
        }
    }

    /// Calculate character transmission time in microseconds based on
    /// current baud rate and WR4/WR3/WR5 settings.
    fn character_time_us(&self) -> u64 {
        if self.baud_rate == 0 {
            return 0;
        }

        let data_bits = match (self.wr[5] >> 5) & 0x03 {
            0 => 5u64,
            1 => 7,
            2 => 6,
            _ => 8,
        };

        let parity_bits = if self.wr[4] & 0x01 != 0 { 1u64 } else { 0 };

        let stop_bits = match (self.wr[4] >> 2) & 0x03 {
            0 => 0u64,   // sync mode
            1 => 10,     // 1 stop bit (x10 for fixed-point)
            2 => 15,     // 1.5 stop bits
            _ => 20,     // 2 stop bits
        };

        // Total bits = 1 start + data + parity + stop (stop is in tenths)
        let total_tenths = (1 + data_bits + parity_bits) * 10 + stop_bits;

        // Time = total_bits / baud_rate, in microseconds
        // = (total_tenths / 10) / baud_rate * 1_000_000
        // = total_tenths * 100_000 / baud_rate
        total_tenths * 100_000 / self.baud_rate as u64
    }
}
