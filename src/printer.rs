//! Centronics parallel printer (LST: device) capture.
//!
//! Every Kaypro has a Centronics parallel port on Z80 PIO #1. The BIOS `LIST`
//! routine streams bytes to it; izkaypro captures that raw byte stream to a
//! file in the user's home directory: **`~/kaypro.out`**. The file is created
//! on the first printed byte and appended to thereafter (and across runs), so
//! print jobs accumulate rather than overwrite.
//!
//! Captured bytes are masked to 7 bits (`& 0x7F`) — the standard behavior for a
//! CP/M text printer. CP/M software (WordStar, NZ-COM/ZCPR3 prompts, utility
//! banners, …) sets the high bit on characters as formatting markers / soft
//! spaces; the Kaypro video masks it for display but the LIST routine emits the
//! raw byte, so without masking a printed space (0xA0) would show as a stray
//! high-bit character. Masking yields clean readable text. Control codes
//! (CR/LF/FF/TAB, all < 0x80) pass through unchanged. (When graphics printing
//! is added later it will need the raw 8-bit stream, reintroduced as a mode.)
//!
//! Two ROM families drive the port differently (resolved in `kaypro_machine`):
//!   - Old (Kaypro II / 4-83): data port 0x08, strobe = port 0x1C bit 4 (active
//!     high), ready = port 0x1C bit 3 = 1.
//!   - New (4-84 / Kaypro 10 / …): data port 0x18, strobe = port 0x14 bit 3
//!     (active low), ready = port 0x14 bit 6 = 0.
//! This module is family-agnostic: it just appends the latched data bytes.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

const OUTPUT_FILE: &str = "kaypro.out";

/// Resolve the user's home directory without an external crate.
fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

/// Captures the parallel-printer (LST:) byte stream to `~/kaypro.out`.
pub struct Printer {
    path: PathBuf,
    /// Opened lazily on the first byte so idle runs don't create the file.
    file: Option<File>,
    /// Set if opening failed once, to avoid retrying/spamming every byte.
    failed: bool,
    pub bytes_written: u64,
}

impl Printer {
    /// Create a printer targeting `~/kaypro.out` (falls back to the current
    /// directory if the home directory can't be determined). The file is not
    /// opened until the first byte is printed.
    pub fn new() -> Printer {
        let path = home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(OUTPUT_FILE);
        Printer {
            path,
            file: None,
            failed: false,
            bytes_written: 0,
        }
    }

    /// The resolved output path (for the startup message).
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Append one byte sent to the Centronics port. Opens the file on first
    /// use (create + append). Failures are reported once, then bytes are
    /// silently dropped — printing must never block the emulated machine.
    pub fn write_byte(&mut self, byte: u8) {
        if self.failed {
            return;
        }
        // Mask to 7 bits: CP/M sets the high bit as a formatting marker / soft
        // space (e.g. 0xA0), which the printer path emits raw. Strip it so text
        // prints cleanly (0xA0 -> space). Control codes (< 0x80) are unaffected.
        let byte = byte & 0x7F;
        // If kaypro.out was deleted out from under us mid-run, the OS keeps the
        // unlinked inode alive for our still-open handle: writes would silently
        // succeed into the orphaned inode and no kaypro.out would ever reappear
        // in the directory. Detect the vanished path and drop the stale handle
        // so the block below re-creates the file.
        if self.file.is_some() && !self.path.exists() {
            self.file = None;
        }
        if self.file.is_none() {
            match OpenOptions::new().create(true).append(true).open(&self.path) {
                Ok(f) => self.file = Some(f),
                Err(e) => {
                    eprintln!(
                        "Printer: cannot open {} ({}); LST: output discarded",
                        self.path.display(),
                        e
                    );
                    self.failed = true;
                    return;
                }
            }
        }
        if let Some(f) = &mut self.file {
            // std::fs::File is unbuffered, so each byte hits the OS directly —
            // fine for the low data rate of a CP/M printer.
            if let Err(e) = f.write_all(&[byte]) {
                eprintln!("Printer: write error ({}); LST: output discarded", e);
                self.failed = true;
                return;
            }
            self.bytes_written += 1;
        }
    }
}
