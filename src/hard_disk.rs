use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};

/// WD1002-05 Winchester Hard Disk Controller emulation for Kaypro 10.
///
/// The WD1002-05 uses an ST412 interface to control a hard drive with
/// 306 cylinders, 4 heads, 17 sectors per track, 512 bytes per sector
/// (total capacity ~10.4 MB). The controller is accessed via I/O ports
/// 0x80-0x87 (task file registers).
///
/// Reference: 61-031050-0030 WD1002-05 HDO OEM Manual (July 1983)

const CYLINDERS: u16 = 306;
const HEADS: u8 = 4;
const SECTORS_PER_TRACK: u8 = 17;
const SECTOR_SIZE: usize = 512;
const DISK_SIZE: usize = CYLINDERS as usize * HEADS as usize
    * SECTORS_PER_TRACK as usize * SECTOR_SIZE;
const NUM_TRACKS: usize = CYLINDERS as usize * HEADS as usize;
const TRACK_SIZE: usize = SECTORS_PER_TRACK as usize * SECTOR_SIZE;

// Status register bits (port 0x87 read)
const STS_BUSY: u8 = 0x80;
const STS_READY: u8 = 0x40;
#[allow(dead_code)]
const STS_WRITE_FAULT: u8 = 0x20;
const STS_SEEK_DONE: u8 = 0x10;
const STS_DRQ: u8 = 0x08;
#[allow(dead_code)]
const STS_CORRECTED: u8 = 0x04;
const STS_ERROR: u8 = 0x01;

// Error register bits (port 0x81 read)
#[allow(dead_code)]
const ERR_BAD_BLOCK: u8 = 0x80;
#[allow(dead_code)]
const ERR_UNCORRECTABLE: u8 = 0x40;
#[allow(dead_code)]
const ERR_CRC: u8 = 0x20;
const ERR_ID_NOT_FOUND: u8 = 0x10;
const ERR_ABORTED: u8 = 0x04;
#[allow(dead_code)]
const ERR_TR000: u8 = 0x02;
#[allow(dead_code)]
const ERR_DAM_NOT_FOUND: u8 = 0x01;

// Diagnostic error codes (written to error register after reset)
#[allow(dead_code)]
const DIAG_PASS: u8 = 0x00;
const DIAG_WD2797: u8 = 0x01;

// Command augment bits
const CMD_MULTI_SEC: u8 = 0x04;
const CMD_LONG: u8 = 0x02;

pub struct HardDisk {
    pub trace: bool,

    // Task file registers (cmdBuf[] in Java)
    data: u8,
    error: u8,
    sector_count: u8,
    sector_number: u8,
    cylinder_low: u8,
    cylinder_high: u8,
    sdh: u8,
    status: u8,

    // Current command being executed
    cur_cmd: u8,
    // Write precompensation (accepted but ignored)
    #[allow(dead_code)]
    precomp: u8,

    // SASI reset state: BUSY is held during diagnostics (~1-2s real HW).
    // We simulate with a countdown decremented on status reads.
    reset_pending: bool,
    busy_countdown: u16,

    // Data transfer buffer and state
    data_buf: Vec<u8>,
    data_length: usize,
    data_ix: usize,
    wr_off: usize, // Disk offset latched at WRITE SECTOR start

    // True when a disk image has been loaded (equivalent to Java's driveFd != null).
    // When false, the controller is detected by the ROM (reset/status/error work)
    // but all commands fail, causing the ROM to fall back to floppy boot.
    drive_present: bool,

    // Backing file for persistent storage
    file: Option<File>,

    // Per-track formatted state. On real hardware, an unformatted track
    // has no sector headers (IDAMs), so READ/WRITE SECTOR fail with
    // ID NOT FOUND. FORMAT TRACK writes the headers, enabling access.
    track_formatted: Vec<bool>,

    // Hard disk image (raw sector data)
    disk_data: Vec<u8>,
}

impl HardDisk {
    pub fn new(trace: bool) -> HardDisk {
        HardDisk {
            trace,
            data: 0,
            error: 0,
            sector_count: 0,
            sector_number: 0,
            cylinder_low: 0,
            cylinder_high: 0,
            sdh: 0,
            status: 0,
            cur_cmd: 0,
            precomp: 0,
            reset_pending: false,
            busy_countdown: 0,
            data_buf: vec![0u8; SECTOR_SIZE + 4], // +4 for ECC in LONG mode
            data_length: SECTOR_SIZE,
            data_ix: 0,
            wr_off: 0,
            drive_present: false,
            file: None,
            track_formatted: Vec::new(),
            disk_data: Vec::new(),
        }
    }

    pub fn load_image(&mut self, path: &str) -> std::io::Result<()> {
        if std::path::Path::new(path).exists() {
            let mut file = OpenOptions::new().read(true).write(true).open(path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            if data.len() != DISK_SIZE {
                eprintln!("HDC: Warning: disk image size {} does not match expected {} bytes",
                    data.len(), DISK_SIZE);
            }
            // Pad or truncate to expected size to prevent out-of-bounds access
            data.resize(DISK_SIZE, 0);
            self.disk_data = data;
            // Detect which tracks are formatted: on real hardware an
            // unformatted surface is all zeros (no flux transitions).
            // A track with any non-zero byte has been formatted/written.
            self.track_formatted = self.detect_formatted_tracks();
            let formatted_count = self.track_formatted.iter().filter(|&&f| f).count();
            eprintln!("HDC: Loaded hard disk image: {} ({} bytes, {}/{} tracks formatted)",
                path, self.disk_data.len(), formatted_count, NUM_TRACKS);
            self.file = Some(file);
        } else {
            let mut file = File::create(path)?;
            self.disk_data = vec![0u8; DISK_SIZE];
            file.write_all(&self.disk_data)?;
            eprintln!("HDC: Created blank hard disk image: {}", path);
            // All tracks unformatted on a new image
            self.track_formatted = vec![false; NUM_TRACKS];
            // Re-open read/write for future seeks
            let file = OpenOptions::new().read(true).write(true).open(path)?;
            self.file = Some(file);
        }
        self.drive_present = true;
        Ok(())
    }

    /// Scan disk_data to determine which tracks contain data (formatted).
    /// On real hardware, an unformatted track has no flux transitions
    /// (all zeros in our representation). Any non-zero byte means the
    /// track has been formatted and written with sector headers.
    fn detect_formatted_tracks(&self) -> Vec<bool> {
        let mut formatted = vec![false; NUM_TRACKS];
        for track_idx in 0..NUM_TRACKS {
            let off = track_idx * TRACK_SIZE;
            let end = (off + TRACK_SIZE).min(self.disk_data.len());
            if off < self.disk_data.len() {
                formatted[track_idx] = self.disk_data[off..end].iter().any(|&b| b != 0);
            }
        }
        formatted
    }

    pub fn flush(&mut self) {
        if let Some(ref mut f) = self.file {
            if f.seek(SeekFrom::Start(0)).is_ok() {
                let _ = f.write_all(&self.disk_data);
            }
        }
    }

    // --- Geometry helpers ---

    fn get_cyl(&self) -> u16 {
        ((self.cylinder_high as u16) << 8) | self.cylinder_low as u16
    }

    /// Extract head number from SDH register (bits 2:0).
    /// Both the 81-478c ROM and HDFMT put LUN=1 in bits 4:3
    /// (via unit<<3) and the actual head number in bits 2:0.
    /// This matches the standard WD1002-05 encoding and the
    /// Java reference implementation (getHead() = sdh & 0x07).
    fn get_head(&self) -> u8 {
        self.sdh & 0x07
    }

    /// CHS → track index (cyl * HEADS + head).
    fn get_track_index(&self) -> usize {
        self.get_cyl() as usize * HEADS as usize + self.get_head() as usize
    }

    /// Check if the current track has been formatted (sector headers exist).
    #[allow(dead_code)]
    fn is_track_formatted(&self) -> bool {
        let idx = self.get_track_index();
        idx < self.track_formatted.len() && self.track_formatted[idx]
    }

    /// CHS → byte offset into disk_data.
    fn get_off(&self) -> usize {
        let cyl = self.get_cyl() as usize;
        let head = self.get_head() as usize;
        let sec = self.sector_number as usize;
        ((cyl * HEADS as usize + head) * SECTORS_PER_TRACK as usize + sec)
            * SECTOR_SIZE
    }

    /// Check if the current CHS address is within disk capacity.
    fn address_valid(&self) -> bool {
        self.get_off() + SECTOR_SIZE <= DISK_SIZE
    }

    // --- Status helpers (bit-level mutation like real hardware) ---

    fn set_done(&mut self) {
        self.status &= !(STS_DRQ | STS_BUSY);
    }

    fn set_error(&mut self, err: u8) {
        self.error |= err;
        self.status |= STS_ERROR;
        self.set_done();
    }

    // --- Reset ---

    /// Called when port 0x14 bit 1 goes LOW (active-low at port level).
    /// Through the 7406 inverter (U13), this asserts MR HIGH on the
    /// WD1002-05. Per the spec, BUSY is set within 200ns and held
    /// for ~1-2 seconds while internal diagnostics run.
    pub fn sasi_reset(&mut self) {
        if self.trace {
            eprintln!("HDC: SASI reset asserted");
        }
        self.data_ix = 0;
        self.cur_cmd = 0;
        // Clear all task file registers
        self.data = 0;
        self.error = 0;
        self.sector_count = 0;
        self.sector_number = 0;
        self.cylinder_low = 0;
        self.cylinder_high = 0;
        self.sdh = 0;
        // BUSY set immediately; cleared when diagnostics complete
        self.status = STS_BUSY;
        self.reset_pending = true;
        // The ROM polls status in a tight loop (~0x6000 iterations with
        // delay calls). We use a countdown decremented on each status
        // read so BUSY clears after a realistic number of polls.
        self.busy_countdown = 20;
    }

    // --- Port I/O ---

    /// Write to task file register (ports 0x80-0x87).
    pub fn write_register(&mut self, port: u8, value: u8) {
        let reg = port & 0x07;
        match reg {
            0 => {
                // Data Register — feed into write buffer
                self.put_data(value);
            }
            1 => {
                // Write Precompensation — accepted, ignored
                self.precomp = value;
            }
            2 => {
                self.sector_count = value;
                if self.trace {
                    eprintln!("HDC: Sector Count = {}", value);
                }
            }
            3 => {
                self.sector_number = value;
                if self.trace {
                    eprintln!("HDC: Sector Number = {}", value);
                }
            }
            4 => {
                self.cylinder_low = value;
                if self.trace {
                    eprintln!("HDC: Cylinder Low = 0x{:02X}", value);
                }
            }
            5 => {
                self.cylinder_high = value;
                if self.trace {
                    eprintln!("HDC: Cylinder High = 0x{:02X}", value);
                }
            }
            6 => {
                // SDH Register — drive/head select
                // Bits 7:5 = sector size (101 = 512 bytes)
                // Bits 4:3 = LUN (always 1 for Kaypro 10 HD)
                // Bits 2:0 = head number
                if self.trace {
                    let head = value & 0x07;
                    let lun = (value >> 3) & 0x03;
                    eprintln!("HDC: SDH = 0x{:02X} (LUN={}, head={})", value, lun, head);
                }
                // Always READY — single drive, all head values valid.
                // The ROM's LUN 3 probe (SDH=0xB8) also gets READY,
                // which tells the ROM the controller board is present.
                self.status |= STS_READY;
                self.sdh = value;
            }
            7 => {
                // Command Register — clear error state and dispatch
                self.cur_cmd = value;
                self.status &= !STS_ERROR;
                self.error = 0;
                self.process_cmd();
            }
            _ => {}
        }
    }

    /// Read from task file register (ports 0x80-0x87).
    pub fn read_register(&mut self, port: u8) -> u8 {
        let reg = port & 0x07;
        let val = match reg {
            0 => {
                // Data Register — read advances the transfer buffer
                let v = self.data;
                self.get_data();
                v
            }
            1 => self.error,
            2 => self.sector_count,
            3 => self.sector_number,
            4 => self.cylinder_low,
            5 => self.cylinder_high,
            6 => self.sdh,
            7 => return self.read_status(), // has its own trace
            _ => 0xFF,
        };
        if self.trace && reg <= 1 {
            let name = match reg {
                0 => "Data",
                1 => "Error",
                _ => "",
            };
            eprintln!("HDC: Read {} = 0x{:02X}", name, val);
        }
        val
    }

    fn read_status(&mut self) -> u8 {
        // Handle busy countdown from SASI reset diagnostics
        if self.busy_countdown > 0 {
            self.busy_countdown -= 1;
            if self.busy_countdown == 0 {
                if self.reset_pending {
                    // Diagnostics complete. Per WD spec section 5.4:
                    // error register gets diagnostic code, but the ERROR
                    // status bit is NOT set (even for non-zero codes).
                    self.error = DIAG_WD2797; // 0x01: no WD2797 floppy chip
                    self.reset_pending = false;
                    self.status = STS_READY | STS_SEEK_DONE;
                    if self.trace {
                        eprintln!("HDC: Reset diagnostics complete, error=0x{:02X}, status=0x{:02X}",
                            self.error, self.status);
                    }
                }
            }
        }
        if self.trace {
            eprintln!("HDC: Read Status = 0x{:02X}", self.status);
        }
        self.status
    }

    // --- Command processing ---

    fn process_cmd(&mut self) {
        let cmd_type = self.cur_cmd & 0xF0;

        // No disk image loaded — all commands fail (Java: driveFd == null)
        if !self.drive_present {
            if self.trace {
                eprintln!("HDC: Command 0x{:02X} rejected — no disk image loaded", self.cur_cmd);
            }
            self.set_error(ERR_ABORTED);
            return;
        }

        if self.trace {
            eprintln!("HDC: Command 0x{:02X} (C={}, H={}, S={}, N={})",
                self.cur_cmd, self.get_cyl(), self.get_head(),
                self.sector_number, self.sector_count);
        }

        match cmd_type {
            0x10 => self.cmd_restore(),
            0x20 => self.cmd_read_sector(),
            0x30 => self.cmd_write_sector(),
            0x50 => self.cmd_format_track(),
            0x70 => self.cmd_seek(),
            0x90 => self.cmd_test(),
            _ => {
                if self.trace {
                    eprintln!("HDC: Unknown command 0x{:02X}", self.cur_cmd);
                }
                self.set_error(ERR_ABORTED);
            }
        }
    }

    /// RESTORE (0x1r): Recalibrate — move heads to cylinder 0.
    /// Per spec and Java: immediate completion, sets SEEK_DONE.
    fn cmd_restore(&mut self) {
        self.cylinder_low = 0;
        self.cylinder_high = 0;
        self.status |= STS_SEEK_DONE;
        if self.trace {
            eprintln!("HDC: RESTORE complete");
        }
    }

    /// SEEK (0x7r): Position heads at cylinder specified in task file.
    /// Per spec and Java: immediate completion, sets SEEK_DONE.
    fn cmd_seek(&mut self) {
        if !self.address_valid() {
            if self.trace {
                eprintln!("HDC: SEEK failed — address out of range");
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }
        self.status |= STS_SEEK_DONE;
        if self.trace {
            eprintln!("HDC: SEEK complete — C={}, H={}", self.get_cyl(), self.get_head());
        }
    }

    /// TEST (0x90): Internal diagnostics. Immediate pass.
    fn cmd_test(&mut self) {
        self.error = DIAG_WD2797; // Same as reset: no floppy chip
        // Per spec: ERROR status bit is NOT set for diagnostic codes
        if self.trace {
            eprintln!("HDC: TEST complete, diag=0x{:02X}", self.error);
        }
    }

    /// READ SECTOR (0x2x): Read sector(s) from disk to host via PIO.
    /// Bit 3 (D): 0=PIO, 1=DMA (we only support PIO)
    /// Bit 2 (M): multi-sector
    /// Bit 1 (L): long (include 4 ECC bytes)
    fn cmd_read_sector(&mut self) {
        let off = self.get_off();
        if off + SECTOR_SIZE > DISK_SIZE {
            if self.trace {
                eprintln!("HDC: READ SECTOR failed — address out of range");
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // On real hardware, unformatted sectors have no ID address marks.
        // We detect this by checking if the sector data is all zeros —
        // a written sector will have non-zero content. This allows the
        // ROM to detect a blank disk (all zeros → ID NOT FOUND → floppy
        // fallback) while letting HDFMT read back sectors it has written.
        if self.disk_data[off..off + SECTOR_SIZE].iter().all(|&b| b == 0) {
            if self.trace {
                eprintln!("HDC: READ SECTOR failed — sector is blank (C={}, H={}, S={}, SDH=0x{:02X})",
                    self.get_cyl(), self.get_head(), self.sector_number, self.sdh);
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // Copy sector data into transfer buffer
        self.data_buf[..SECTOR_SIZE].copy_from_slice(&self.disk_data[off..off + SECTOR_SIZE]);
        self.data_length = SECTOR_SIZE;

        // LONG mode: append 4 fake ECC bytes
        if self.cur_cmd & CMD_LONG != 0 {
            self.data_buf[SECTOR_SIZE] = 0;
            self.data_buf[SECTOR_SIZE + 1] = 0;
            self.data_buf[SECTOR_SIZE + 2] = 0;
            self.data_buf[SECTOR_SIZE + 3] = 0;
            self.data_length += 4;
        }

        self.data_ix = 0;
        // Prime first byte and set DRQ (PIO mode: DRQ set, BUSY cleared)
        self.get_data();

        if self.trace {
            eprintln!("HDC: READ SECTOR — offset 0x{:X}, {} bytes", off, self.data_length);
        }
    }

    /// WRITE SECTOR (0x3x): Write sector(s) from host to disk via PIO.
    /// Bit 2 (M): multi-sector
    /// Bit 1 (L): long (expect 4 extra ECC bytes)
    fn cmd_write_sector(&mut self) {
        self.wr_off = self.get_off();
        if self.wr_off + SECTOR_SIZE > DISK_SIZE {
            if self.trace {
                eprintln!("HDC: WRITE SECTOR failed — address out of range");
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        self.data_length = SECTOR_SIZE;
        if self.cur_cmd & CMD_LONG != 0 {
            self.data_length += 4;
        }
        self.data_ix = 0;
        // Set DRQ and BUSY — host must fill the buffer
        self.status |= STS_DRQ | STS_BUSY;

        if self.trace {
            eprintln!("HDC: WRITE SECTOR — offset 0x{:X}, expecting {} bytes",
                self.wr_off, self.data_length);
        }
    }

    /// FORMAT TRACK (0x50): Format a track using interleave table from host.
    /// Host writes 2 bytes per sector (bad block byte + logical sector number).
    fn cmd_format_track(&mut self) {
        if !self.address_valid() {
            if self.trace {
                eprintln!("HDC: FORMAT failed — address out of range");
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // HDFMT sends 512 bytes (two Z80 OTIRs of 256 bytes each):
        // 34 bytes of interleave table followed by padding. The controller
        // must absorb all bytes before completing. Java also uses 512.
        self.data_length = SECTOR_SIZE;
        self.data_ix = 0;
        self.status |= STS_DRQ | STS_BUSY;

        if self.trace {
            eprintln!("HDC: FORMAT TRACK — C={}, H={}, expecting {} bytes interleave table",
                self.get_cyl(), self.get_head(), self.data_length);
        }
    }

    // --- Data transfer (PIO) ---

    /// Called when the host reads the Data Register (port 0x80).
    /// Advances the read buffer index. When the buffer is exhausted,
    /// handles multi-sector continuation or completes the transfer.
    fn get_data(&mut self) {
        if self.data_ix < self.data_length {
            self.data = self.data_buf[self.data_ix];
            self.data_ix += 1;
            self.status |= STS_DRQ;
            return;
        }

        // Buffer exhausted
        self.data_ix = 0;

        if self.cur_cmd & CMD_MULTI_SEC != 0 && self.sector_count > 0 {
            self.sector_count -= 1;
            self.sector_number = self.sector_number.wrapping_add(1);
            if self.sector_number >= SECTORS_PER_TRACK {
                self.sector_number = 0;
            }
            if self.sector_count > 0 {
                // Continue with next sector
                self.process_cmd();
                return;
            }
        }
        self.set_done();
    }

    /// Called when the host writes the Data Register (port 0x80).
    /// Fills the write buffer. When full, commits data to disk and
    /// handles multi-sector continuation.
    fn put_data(&mut self, val: u8) {
        self.data = val;

        if self.data_ix < self.data_length {
            self.data_buf[self.data_ix] = val;
            self.data_ix += 1;
            if self.data_ix < self.data_length {
                self.status |= STS_DRQ;
            } else {
                // Buffer full — process the received data
                self.process_data();
            }
            return;
        }
        self.set_done();
    }

    /// Called when a write buffer is full. Commits data to disk for
    /// WRITE SECTOR, or handles FORMAT completion.
    fn process_data(&mut self) {
        let cmd_type = self.cur_cmd & 0xF0;
        match cmd_type {
            0x30 => {
                // WRITE SECTOR — commit buffer to disk image
                // Only write the actual sector data (not ECC bytes)
                let write_len = SECTOR_SIZE.min(self.data_length);
                let off = self.wr_off;
                if off + write_len <= DISK_SIZE {
                    self.disk_data[off..off + write_len]
                        .copy_from_slice(&self.data_buf[..write_len]);
                    if self.trace {
                        eprintln!("HDC: WRITE SECTOR committed {} bytes at offset 0x{:X}",
                            write_len, off);
                    }
                    // Persist to backing file
                    if let Some(ref mut f) = self.file {
                        if f.seek(SeekFrom::Start(off as u64)).is_ok() {
                            let _ = f.write_all(&self.disk_data[off..off + write_len]);
                        }
                    }
                }

                self.data_ix = 0;
                if self.cur_cmd & CMD_MULTI_SEC != 0 && self.sector_count > 0 {
                    self.sector_count -= 1;
                    self.sector_number = self.sector_number.wrapping_add(1);
                    if self.sector_number >= SECTORS_PER_TRACK {
                        self.sector_number = 0;
                    }
                    if self.sector_count > 0 {
                        // Re-latch offset for next sector and request more data
                        self.wr_off = self.get_off();
                        self.status |= STS_DRQ | STS_BUSY;
                        return;
                    }
                }
                self.set_done();
            }
            0x50 => {
                // FORMAT TRACK — interleave table received (data ignored).
                // Mark the track as formatted so subsequent READ/WRITE
                // commands succeed. Actual data is written by WRITE SECTOR.
                // This matches the Java reference which just sets
                // formatted=true and calls setDone().
                let track_idx = self.get_track_index();
                if track_idx < self.track_formatted.len() {
                    self.track_formatted[track_idx] = true;
                }
                let formatted_count = self.track_formatted.iter().filter(|&&f| f).count();
                eprintln!("HDC: FORMAT TRACK complete — C={}, H={}, SDH=0x{:02X}, track_idx={}, total_formatted={}",
                    self.get_cyl(), self.get_head(), self.sdh, track_idx, formatted_count);
                self.set_done();
            }
            _ => {
                self.set_done();
            }
        }
    }
}
