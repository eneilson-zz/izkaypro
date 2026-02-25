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

macro_rules! hdc_log {
    ($file:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        if let Some(ref mut f) = $file {
            let _ = writeln!(f, "{}", msg);
        } else {
            eprintln!("{}", msg);
        }
    }};
}

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

    // INTRQ state — per spec, asserted on command completion, cleared
    // when host reads Status register or writes a new command.
    // Not wired to the Z80 interrupt chain in the Kaypro, but tracked
    // for strict spec compliance.
    intrq: bool,

    // Bitmask of LUNs (drive select, SDH bits 4:3) that report READY and
    // accept commands. Default is LUN1 only (bit 1), matching Kaypro 10.
    ready_lun_mask: u8,

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
    pub disk_data: Vec<u8>,

    // Optional trace log file (when set, traces go here instead of stderr)
    pub trace_file: Option<File>,
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
            intrq: false,
            ready_lun_mask: 1 << 1,
            data_buf: vec![0u8; 1024 + 4], // max sector size (1024) + 4 ECC bytes
            data_length: SECTOR_SIZE,
            data_ix: 0,
            wr_off: 0,
            drive_present: false,
            file: None,
            track_formatted: Vec::new(),
            disk_data: Vec::new(),
            trace_file: None,
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
            // An existing image has been through HDFMT which formats all
            // tracks. Mark every track as formatted so READ/WRITE succeed
            // everywhere. (The old heuristic — check for non-zero bytes —
            // fails for tracks that were formatted but never written.)
            self.track_formatted = vec![true; NUM_TRACKS];
            eprintln!("HDC: Loaded hard disk image: {} ({} bytes)",
                path, self.disk_data.len());
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

    pub fn set_trace_file(&mut self, file: File) {
        self.trace_file = Some(file);
    }

    /// Configure which LUNs report READY and accept commands.
    /// Bit N corresponds to LUN N (0..3).
    #[allow(dead_code)]
    pub fn set_ready_lun_mask(&mut self, mask: u8) {
        self.ready_lun_mask = mask & 0x0F;
    }

    fn lun_is_ready(&self, lun: u8) -> bool {
        self.drive_present && (self.ready_lun_mask & (1u8 << lun)) != 0
    }

    /// Scan disk_data to determine which tracks contain data (formatted).
    /// On real hardware, an unformatted track has no flux transitions
    /// (all zeros in our representation). Any non-zero byte means the
    /// track has been formatted and written with sector headers.
    #[allow(dead_code)]
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

    /// Extract LUN (drive select) from SDH register (bits 4:3).
    fn get_lun(&self) -> u8 {
        (self.sdh >> 3) & 0x03
    }

    /// Check if the current track has been formatted (sector headers exist).
    fn is_track_formatted(&self) -> bool {
        let idx = self.get_track_index();
        idx < self.track_formatted.len() && self.track_formatted[idx]
    }

    /// CHS → byte offset into disk_data.
    /// The sector stride comes from the SDH register's sector size field
    /// (bits 6:5): 256, 512, 1024, or 128 bytes. When HDFMT formats with
    /// 256-byte sectors (SDH=0x88), sector N starts at N×256 within the
    /// track, allowing 34 sectors per track instead of 17.
    fn get_off(&self) -> usize {
        let cyl = self.get_cyl() as usize;
        let head = self.get_head() as usize;
        let sec = self.sector_number as usize;
        let sector_size = self.get_sector_size();
        (cyl * HEADS as usize + head) * TRACK_SIZE + sec * sector_size
    }

    // --- Status helpers (bit-level mutation like real hardware) ---

    fn set_done(&mut self) {
        self.status &= !(STS_DRQ | STS_BUSY);
        self.intrq = true;
    }

    fn set_error(&mut self, err: u8) {
        self.error |= err;
        self.status |= STS_ERROR;
        self.set_done();
    }

    /// Advance CHS to the next sector for multi-sector operations.
    /// Wraps sector → head → cylinder per WD spec.
    fn advance_sector(&mut self) {
        self.sector_number = self.sector_number.wrapping_add(1);
        if self.sector_number >= SECTORS_PER_TRACK {
            self.sector_number = 0;
            let mut head = self.get_head() + 1;
            if head >= HEADS {
                head = 0;
                let cyl = self.get_cyl() + 1;
                self.cylinder_low = cyl as u8;
                self.cylinder_high = (cyl >> 8) as u8;
            }
            // Update head in SDH register (bits 2:0)
            self.sdh = (self.sdh & 0xF8) | (head & 0x07);
        }
    }

    // --- Reset ---

    /// Called when port 0x14 bit 1 goes LOW (active-low at port level).
    /// Through the 7406 inverter (U13), this asserts MR HIGH on the
    /// WD1002-05. Per the spec, BUSY is set within 200ns and held
    /// for ~1-2 seconds while internal diagnostics run.
    pub fn sasi_reset(&mut self) {
        if self.trace {
            hdc_log!(self.trace_file, "HDC: SASI reset asserted");
        }
        self.data_ix = 0;
        self.cur_cmd = 0;
        self.intrq = false;
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
                    hdc_log!(self.trace_file, "HDC: Sector Count = {}", value);
                }
            }
            3 => {
                self.sector_number = value;
                if self.trace {
                    hdc_log!(self.trace_file, "HDC: Sector Number = {}", value);
                }
            }
            4 => {
                self.cylinder_low = value;
                if self.trace {
                    hdc_log!(self.trace_file, "HDC: Cylinder Low = 0x{:02X}", value);
                }
            }
            5 => {
                self.cylinder_high = value;
                if self.trace {
                    hdc_log!(self.trace_file, "HDC: Cylinder High = 0x{:02X}", value);
                }
            }
            6 => {
                // SDH Register — drive/head select
                // Bit 7 = ECC/CRC mode
                // Bits 6:5 = sector size
                // Bits 4:3 = LUN (drive select)
                // Bits 2:0 = head number
                self.sdh = value;
                let lun = (value >> 3) & 0x03;
                // Per spec: READY and SEEK COMPLETE reflect the drive's
                // status signals. A present, spun-up drive with settled
                // heads reports both when the selected LUN is configured.
                if self.lun_is_ready(lun) {
                    self.status |= STS_READY | STS_SEEK_DONE;
                } else {
                    self.status &= !(STS_READY | STS_SEEK_DONE);
                }
                if self.trace {
                    let head = value & 0x07;
                    hdc_log!(self.trace_file, "HDC: SDH = 0x{:02X} (LUN={}, head={}, ready={})",
                        value, lun, head, self.status & STS_READY != 0);
                }
            }
            7 => {
                // Command Register — per spec: writing a command sets BUSY,
                // clears SEEK_DONE, clears INTRQ, and clears error state.
                self.cur_cmd = value;
                self.status |= STS_BUSY;
                self.status &= !(STS_ERROR | STS_SEEK_DONE);
                self.error = 0;
                self.intrq = false;
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
        if self.trace {
            let name = match reg {
                0 => "Data",
                1 => "Error",
                2 => "SectorCount",
                3 => "SectorNum",
                4 => "CylLow",
                5 => "CylHigh",
                6 => "SDH",
                _ => "",
            };
            hdc_log!(self.trace_file, "HDC: Read {} = 0x{:02X}", name, val);
        }
        val
    }

    fn read_status(&mut self) -> u8 {
        // Per spec: reading Status register clears INTRQ
        self.intrq = false;

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
                    // Set READY if the selected LUN is configured
                    self.status = STS_SEEK_DONE;
                    if self.lun_is_ready(self.get_lun()) {
                        self.status |= STS_READY;
                    }
                    if self.trace {
                        hdc_log!(self.trace_file, "HDC: Reset diagnostics complete, error=0x{:02X}, status=0x{:02X}",
                            self.error, self.status);
                    }
                }
            }
        }
        if self.trace {
            hdc_log!(self.trace_file, "HDC: Read Status = 0x{:02X}", self.status);
        }
        self.status
    }

    // --- Command processing ---

    fn process_cmd(&mut self) {
        let cmd_type = self.cur_cmd & 0xF0;

        // TEST (0x90) runs internal diagnostics regardless of drive state
        if cmd_type == 0x90 {
            self.cmd_test();
            return;
        }

        // No disk image loaded — all commands fail (Java: driveFd == null)
        if !self.drive_present {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: Command 0x{:02X} rejected — no disk image loaded", self.cur_cmd);
            }
            self.set_error(ERR_ABORTED);
            return;
        }

        // Per spec: commands abort if drive not ready (unconfigured LUN)
        if !self.lun_is_ready(self.get_lun()) {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: Command 0x{:02X} rejected — drive not ready (LUN {} mask=0x{:X})",
                    self.cur_cmd, self.get_lun(), self.ready_lun_mask);
            }
            self.set_error(ERR_ABORTED);
            return;
        }

        if self.trace {
            hdc_log!(self.trace_file, "HDC: Command 0x{:02X} (C={}, H={}, S={}, N={})",
                self.cur_cmd, self.get_cyl(), self.get_head(),
                self.sector_number, self.sector_count);
        }

        match cmd_type {
            0x10 => self.cmd_restore(),
            0x20 => self.cmd_read_sector(),
            0x30 => self.cmd_write_sector(),
            0x50 => self.cmd_format_track(),
            0x70 => self.cmd_seek(),
            _ => {
                if self.trace {
                    hdc_log!(self.trace_file, "HDC: Unknown command 0x{:02X}", self.cur_cmd);
                }
                self.set_error(ERR_ABORTED);
            }
        }
    }

    /// RESTORE (0x1r): Recalibrate — move heads to cylinder 0.
    /// Per spec: steps toward track 0, sets SEEK_DONE on completion.
    /// Bits 3:0 encode stepping rate (ignored in emulation).
    fn cmd_restore(&mut self) {
        self.cylinder_low = 0;
        self.cylinder_high = 0;
        self.status &= !STS_BUSY;
        self.status |= STS_SEEK_DONE;
        if self.trace {
            hdc_log!(self.trace_file, "HDC: RESTORE complete");
        }
    }

    /// SEEK (0x7r): Position heads at cylinder specified in task file.
    /// Per spec: validates cylinder/head (not sector), sets SEEK_DONE.
    /// Bits 3:0 encode stepping rate (ignored in emulation).
    fn cmd_seek(&mut self) {
        // Per spec: SEEK validates cylinder/head only, not sector
        let cyl = self.get_cyl();
        let head = self.get_head();
        if cyl >= CYLINDERS || head >= HEADS {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: SEEK failed — C={} H={} out of range", cyl, head);
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }
        self.status &= !STS_BUSY;
        self.status |= STS_SEEK_DONE;
        if self.trace {
            hdc_log!(self.trace_file, "HDC: SEEK complete — C={}, H={}", cyl, head);
        }
    }

    /// TEST (0x90): Internal diagnostics. Immediate pass.
    /// Per spec: runs self-test, reports result in error register
    /// without setting the ERROR status bit. Sets INTRQ on completion.
    fn cmd_test(&mut self) {
        self.error = DIAG_WD2797; // Same as reset: no floppy chip
        // Per spec: ERROR status bit is NOT set for diagnostic codes
        self.status &= !STS_BUSY;
        self.intrq = true;
        if self.trace {
            hdc_log!(self.trace_file, "HDC: TEST complete, diag=0x{:02X}", self.error);
        }
    }

    /// READ SECTOR (0x2x): Read sector(s) from disk to host via PIO.
    /// Bit 3 (D): 0=PIO, 1=DMA (we only support PIO)
    /// Bit 2 (M): multi-sector
    /// Bit 1 (L): long (include 4 ECC bytes)
    fn cmd_read_sector(&mut self) {
        let off = self.get_off();
        let xfer_size = self.get_sector_size();
        if off + xfer_size > DISK_SIZE {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: READ SECTOR failed — address out of range");
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // Per WD spec: ID NOT FOUND means the sector header was not found
        // on the track — i.e., the track hasn't been formatted. Check the
        // per-track formatted state rather than the sector data content.
        if !self.is_track_formatted() {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: READ SECTOR failed — track not formatted (C={}, H={}, S={}, SDH=0x{:02X})",
                    self.get_cyl(), self.get_head(), self.sector_number, self.sdh);
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // Copy sector data into transfer buffer using the full SDH sector size.
        self.data_buf[..xfer_size].copy_from_slice(&self.disk_data[off..off + xfer_size]);
        self.data_length = xfer_size;

        // LONG mode: append 4 fake ECC bytes
        if self.cur_cmd & CMD_LONG != 0 {
            self.data_buf[xfer_size] = 0;
            self.data_buf[xfer_size + 1] = 0;
            self.data_buf[xfer_size + 2] = 0;
            self.data_buf[xfer_size + 3] = 0;
            self.data_length += 4;
        }

        self.data_ix = 0;
        // Per spec PIO READ: BUSY cleared, DRQ set when buffer ready
        self.status &= !STS_BUSY;
        self.get_data();

        if self.trace {
            hdc_log!(self.trace_file, "HDC: READ SECTOR — offset 0x{:X}, {} bytes", off, self.data_length);
        }
        if self.trace {
            let preview_len = 16.min(xfer_size);
            let preview: String = (0..preview_len)
                .map(|i| format!("{:02x}", self.data_buf[i]))
                .collect::<Vec<_>>().join(" ");
            hdc_log!(self.trace_file, "HDC: READ data[0..{}]: {}", preview_len, preview);
        }
    }

    /// WRITE SECTOR (0x3x): Write sector(s) from host to disk via PIO.
    /// Bit 2 (M): multi-sector
    /// Bit 1 (L): long (expect 4 extra ECC bytes)
    fn cmd_write_sector(&mut self) {
        self.wr_off = self.get_off();
        let sector_size = self.get_sector_size();
        if self.wr_off + sector_size > DISK_SIZE {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: WRITE SECTOR failed — address out of range");
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // Per WD spec: writing to an unformatted track fails with ID NOT FOUND
        if !self.is_track_formatted() {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: WRITE SECTOR failed — track not formatted (C={}, H={}, S={})",
                    self.get_cyl(), self.get_head(), self.sector_number);
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // Transfer size comes from SDH bits 6:5 (256 or 512 for Kaypro)
        self.data_length = self.get_sector_size();
        if self.cur_cmd & CMD_LONG != 0 {
            self.data_length += 4;
        }
        self.data_ix = 0;
        // Per spec PIO WRITE: DRQ set to request data from host, BUSY remains
        self.status |= STS_DRQ;

        if self.trace {
            hdc_log!(self.trace_file, "HDC: WRITE SECTOR — offset 0x{:X}, expecting {} bytes",
                self.wr_off, self.data_length);
        }
    }

    /// Extract sector size from SDH register bits 6:5.
    /// Per WD1002-05 spec: 00=256, 01=512, 10=1024, 11=128.
    fn get_sector_size(&self) -> usize {
        match (self.sdh >> 5) & 0x03 {
            0 => 256,
            1 => 512,
            2 => 1024,
            3 => 128,
            _ => unreachable!(),
        }
    }

    /// FORMAT TRACK (0x50): Format a track using interleave table from host.
    /// Per spec: host writes 2 bytes per sector (bad block flag + logical
    /// sector number). The Sector Count register specifies sectors per track.
    /// The total transfer size matches the SDH sector size encoding.
    fn cmd_format_track(&mut self) {
        // Validate cylinder/head (sector number is not relevant for FORMAT)
        let cyl = self.get_cyl();
        let head = self.get_head();
        if cyl >= CYLINDERS || head >= HEADS {
            if self.trace {
                hdc_log!(self.trace_file, "HDC: FORMAT failed — C={} H={} out of range", cyl, head);
            }
            self.set_error(ERR_ID_NOT_FOUND);
            return;
        }

        // The host sends an interleave table whose size equals the
        // sector size encoded in the SDH register (bits 6:5).
        // HDFMT uses SDH=0xA0 (512-byte, 2 OTIRs) or SDH=0x80
        // (256-byte, 1 OTIR). We must match the expected length.
        self.data_length = self.get_sector_size();
        self.data_ix = 0;
        // Per spec: DRQ set to receive interleave table, BUSY remains
        self.status |= STS_DRQ;

        if self.trace {
            hdc_log!(self.trace_file, "HDC: FORMAT TRACK — C={}, H={}, SDH=0x{:02X}, sectors={}, expecting {} bytes",
                cyl, head, self.sdh, self.sector_count, self.data_length);
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
            self.advance_sector();
            if self.sector_count > 0 {
                // Continue with next sector — re-enter READ with BUSY
                self.status |= STS_BUSY;
                self.cmd_read_sector();
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
                // WRITE SECTOR — commit buffer to disk image.
                // Only write actual sector data (not ECC bytes in LONG mode).
                let sector_size = self.get_sector_size();
                let write_len = sector_size;
                let off = self.wr_off;
                if off + write_len <= DISK_SIZE {
                    self.disk_data[off..off + write_len]
                        .copy_from_slice(&self.data_buf[..write_len]);
                    if self.trace {
                        hdc_log!(self.trace_file, "HDC: WRITE SECTOR committed {} bytes at offset 0x{:X}",
                            write_len, off);
                        let preview_len = 16.min(write_len);
                        let preview: String = (0..preview_len)
                            .map(|i| format!("{:02x}", self.data_buf[i]))
                            .collect::<Vec<_>>().join(" ");
                        hdc_log!(self.trace_file, "HDC: WRITE data[0..{}]: {}", preview_len, preview);
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
                    self.advance_sector();
                    if self.sector_count > 0 {
                        // Re-latch offset for next sector and request more data
                        self.wr_off = self.get_off();
                        self.status |= STS_DRQ;
                        return;
                    }
                }
                self.set_done();
            }
            0x50 => {
                // FORMAT TRACK — interleave table received.
                // Parse bad block flags from the interleave data (2 bytes
                // per sector: bad_block_flag, logical_sector_number).
                // Mark the track as formatted so subsequent READ/WRITE
                // commands succeed.
                let track_idx = self.get_track_index();
                if track_idx < self.track_formatted.len() {
                    self.track_formatted[track_idx] = true;
                }
                // Fill formatted track data with 0xE5 (standard CP/M blank fill).
                // This persists the "formatted" state to the image file so that
                // detect_formatted_tracks correctly identifies it on reload.
                let off = track_idx * TRACK_SIZE;
                if off + TRACK_SIZE <= self.disk_data.len() {
                    self.disk_data[off..off + TRACK_SIZE].fill(0xE5);
                    if let Some(ref mut f) = self.file {
                        if f.seek(SeekFrom::Start(off as u64)).is_ok() {
                            let _ = f.write_all(&self.disk_data[off..off + TRACK_SIZE]);
                        }
                    }
                }
                if self.trace {
                    let formatted_count = self.track_formatted.iter().filter(|&&f| f).count();
                    hdc_log!(self.trace_file, "HDC: FORMAT TRACK complete — C={}, H={}, SDH=0x{:02X}, track_idx={}, total_formatted={}",
                        self.get_cyl(), self.get_head(), self.sdh, track_idx, formatted_count);
                }
                self.set_done();
            }
            _ => {
                self.set_done();
            }
        }
    }
}
