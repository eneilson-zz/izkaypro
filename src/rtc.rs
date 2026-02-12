use std::time::SystemTime;

/// MM58167A Real Time Clock emulation for Kaypro 4-84.
///
/// The RTC is accessed indirectly through a Z80 PIO (U35):
/// - Port 0x20 (CLKADD): Write the MM58167A register number to select
/// - Port 0x22 (CLKCTL): PIO control port (accepts init writes silently)
/// - Port 0x24 (CLKDAT): Read/write the selected register's BCD value
///
/// On boot, counters are populated from the host system clock.
/// If the user sets the clock, an offset is stored so the clock
/// continues ticking from the user-set value for the session.

pub struct Rtc {
    reg_select: u8,
    ram: [u8; 8],           // Alarm/RAM latch registers (0x08-0x0F)
    time_offset_secs: i64,  // Offset from host time (set by user writes)
    pub trace: bool,
}

impl Rtc {
    pub fn new(trace: bool) -> Rtc {
        Rtc {
            reg_select: 0,
            ram: [0; 8],
            time_offset_secs: 0,
            trace,
        }
    }

    /// Write to port 0x20 (CLKADD) — select an RTC register.
    pub fn write_addr(&mut self, value: u8) {
        self.reg_select = value & 0x1F;
        if self.trace {
            println!("RTC: Select register 0x{:02X}", self.reg_select);
        }
    }

    /// Read from port 0x20 (CLKADD) — echo back selected register.
    /// kayclk.com uses this for RTC detection: writes a register number,
    /// reads it back, and checks if the low nibble matches.
    pub fn read_addr(&self) -> u8 {
        let value = self.reg_select;
        if self.trace {
            println!("RTC: Read addr = 0x{:02X}", value);
        }
        value
    }

    /// Write to port 0x22 (CLKCTL) — PIO control, accepted silently.
    pub fn write_control(&mut self, value: u8) {
        if self.trace {
            println!("RTC: PIO control write 0x{:02X} (ignored)", value);
        }
    }

    /// Write to port 0x24 (CLKDAT) — write to the selected register.
    pub fn write_data(&mut self, value: u8) {
        let reg = self.reg_select;
        if self.trace {
            println!("RTC: Write reg 0x{:02X} = 0x{:02X}", reg, value);
        }
        match reg {
            0x00..=0x07 => {
                self.set_counter(reg, value);
            }
            0x08..=0x0F => {
                self.ram[(reg - 0x08) as usize] = value;
            }
            0x12 => {
                // Counters Reset — reset offset so clock matches host time
                if value == 0xFF {
                    self.time_offset_secs = 0;
                    if self.trace {
                        println!("RTC: Counters reset");
                    }
                }
            }
            0x13 => {
                // RAM Reset
                if value == 0xFF {
                    self.ram = [0; 8];
                    if self.trace {
                        println!("RTC: RAM reset");
                    }
                }
            }
            _ => {}
        }
    }

    /// Read from port 0x24 (CLKDAT) — read the selected register.
    pub fn read_data(&self) -> u8 {
        let reg = self.reg_select;
        let value = match reg {
            0x00..=0x07 => self.read_counter(reg),
            0x08..=0x0F => self.ram[(reg - 0x08) as usize],
            0x10 => 0,    // Interrupt Status (no interrupts in Phase 1)
            0x11 => 0,    // Interrupt Control
            0x14 => 0,    // Status Bit — no rollover
            _ => 0,
        };
        if self.trace {
            println!("RTC: Read reg 0x{:02X} = 0x{:02X}", reg, value);
        }
        value
    }

    /// Read a counter register by computing current time from host clock + offset.
    fn read_counter(&self, reg: u8) -> u8 {
        let (ms, sec, min, hour, dow, day, month) = self.current_time();
        match reg {
            0x00 => to_bcd((ms % 1000) as u8),
            0x01 => to_bcd((ms / 10 % 100) as u8),
            0x02 => to_bcd(sec),
            0x03 => to_bcd(min),
            0x04 => to_bcd(hour),
            0x05 => dow,
            0x06 => to_bcd(day),
            0x07 => to_bcd(month),
            _ => 0,
        }
    }

    /// Set a counter register by computing the offset needed to shift host time.
    fn set_counter(&mut self, reg: u8, bcd_value: u8) {
        let (_, cur_sec, cur_min, cur_hour, _, cur_day, cur_month) = self.current_time();
        let val = from_bcd(bcd_value);

        let delta: i64 = match reg {
            0x02 => (val as i64) - (cur_sec as i64),
            0x03 => ((val as i64) - (cur_min as i64)) * 60,
            0x04 => ((val as i64) - (cur_hour as i64)) * 3600,
            0x06 => ((val as i64) - (cur_day as i64)) * 86400,
            0x07 => ((val as i64) - (cur_month as i64)) * 30 * 86400,
            0x00 | 0x01 | 0x05 => 0, // Sub-second and day-of-week: ignore writes
            _ => 0,
        };
        self.time_offset_secs += delta;
    }

    /// Get the current RTC time as (ms, sec, min, hour, dow, day, month).
    fn current_time(&self) -> (u16, u8, u8, u8, u8, u8, u8) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();

        let total_secs = now.as_secs() as i64 + self.time_offset_secs;
        let ms = (now.subsec_millis()) as u16;

        // Convert Unix timestamp to broken-down time components
        // Using a simplified algorithm (no timezone — uses UTC, but we'll
        // apply local timezone offset)
        let local_offset = local_utc_offset_secs();
        let local_secs = total_secs + local_offset;

        let (year, month, day) = civil_from_days(local_secs / 86400);
        let day_secs = ((local_secs % 86400) + 86400) % 86400;
        let hour = (day_secs / 3600) as u8;
        let min = ((day_secs % 3600) / 60) as u8;
        let sec = (day_secs % 60) as u8;

        // Day of week: 0=Sunday in civil_from_days epoch
        let dow = day_of_week(year, month, day);

        (ms, sec, min, hour, dow, day as u8, month as u8)
    }

    #[allow(dead_code)]
    pub fn status_string(&self) -> String {
        let (_, sec, min, hour, _, day, month) = self.current_time();
        format!("RTC:{:02X}/{:02X} {:02X}:{:02X}:{:02X}",
            to_bcd(month), to_bcd(day),
            to_bcd(hour), to_bcd(min), to_bcd(sec))
    }
}

fn to_bcd(val: u8) -> u8 {
    ((val / 10) << 4) | (val % 10)
}

fn from_bcd(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

/// Convert days since Unix epoch to (year, month, day).
/// Algorithm from Howard Hinnant's chrono-compatible date algorithms.
fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + (era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u8, d as u8)
}

/// Zeller-style day of week: 1=Sunday..7=Saturday (MM58167A format)
fn day_of_week(year: i32, month: u8, day: u8) -> u8 {
    let (mut y, mut m) = (year as i64, month as i64);
    if m < 3 {
        m += 12;
        y -= 1;
    }
    let dow = (day as i64 + (13 * (m + 1)) / 5 + y + y / 4 - y / 100 + y / 400) % 7;
    // Zeller: 0=Sat, 1=Sun, 2=Mon, ..., 6=Fri
    // MM58167A: 1=Sunday..7=Saturday
    match dow {
        0 => 7, // Saturday
        1 => 1, // Sunday
        n => n as u8,
    }
}

/// Get the local timezone offset from UTC in seconds.
/// Uses libc localtime_r on Unix.
fn local_utc_offset_secs() -> i64 {
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        tm.tm_gmtoff as i64
    }
}
