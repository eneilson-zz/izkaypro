use crate::hard_disk_image::{
    ControllerWriteOutcome, ControllerWriteSource, HardDiskImage, HEADS, SECTOR_SIZE, SECTORS_PER_TRACK,
};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

const REG_DATA: usize = 0;
const REG_ERROR: usize = 1;
const REG_PRECOMP: usize = 1;
const REG_SEC_COUNT: usize = 2;
const REG_SECTOR: usize = 3;
const REG_CYL_LO: usize = 4;
const REG_CYL_HI: usize = 5;
const REG_SDH: usize = 6;
const REG_STATUS: usize = 7;
const REG_CMD: usize = 7;

const STS_BUSY: u8 = 0x80;
const STS_READY: u8 = 0x40;
const STS_WRITE_FAULT: u8 = 0x20;
const STS_SEEK_DONE: u8 = 0x10;
const STS_DRQ: u8 = 0x08;
const STS_CORR: u8 = 0x04;
const STS_ERROR: u8 = 0x01;

const ERR_CRC: u8 = 0x20;
const ERR_ID_NOT_FOUND: u8 = 0x10;
const ERR_ABORTED: u8 = 0x04;
const ERR_DAM_NOT_FOUND: u8 = 0x01;

const DIAG_WD2797_ERR: u8 = 0x01;

const CMD_RESTORE: u8 = 0x10;
const CMD_TEST: u8 = 0x90;
const CMD_READ: u8 = 0x20;
const CMD_WRITE: u8 = 0x30;
const CMD_FORMAT_TRACK: u8 = 0x50;
const CMD_SEEK: u8 = 0x70;
const CMD_MULTI: u8 = 0x04;
const CMD_LONG: u8 = 0x02;

// Complete controller self-test after this many host register polls.
// Using poll-based completion avoids dependence on host wall-clock speed.
const RESET_DIAG_POLLS: u32 = 1024;
const ACTIVE_DRIVE_SELECT: u8 = 0x01; // Kaypro 10 wiring uses DriveSel2.
const K10_128_LOGICAL_TO_PHYS: [u8; 16] = [1, 6, 11, 16, 4, 9, 14, 2, 7, 12, 17, 5, 10, 15, 3, 8];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TransferPhase {
    Idle,
    ReadData,
    WriteData,
    FormatData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingCompletion {
    SeekOk,
    SeekErr(u8),
}

#[derive(Clone, Copy, Debug)]
pub struct WdDebugSnapshot {
    pub cmd: u8,
    pub status: u8,
    pub sec_count: u8,
    pub sector: u8,
    pub cyl: u16,
    pub sdh: u8,
    pub xfer_size: usize,
    pub data_ix: usize,
    pub phase: u8,
    pub pending_offset: u64,
    pub logical_spt: u8,
    pub last_load_offset: u64,
    pub last_load_sum128: u16,
    pub last_load_sum_full: u16,
}

pub struct Wd1002Controller {
    regs: [u8; 8],
    cur_cmd: u8,
    precomp: u8,
    image: HardDiskImage,
    data_buf: Vec<u8>,
    data_len: usize,
    data_ix: usize,
    phase: TransferPhase,
    pending_offset: u64,
    intrq: bool,
    reset_gate_high: bool,
    xfer_size: usize,
    remaining_sectors: u16,
    trace: bool,
    trace_log: Option<BufWriter<File>>,
    idle_data_reads: u64,
    diag_polls_remaining: u32,
    complete_polls_remaining: u8,
    pending_completion: Option<PendingCompletion>,
    last_load_offset: u64,
    last_load_sum128: u16,
    last_load_sum_full: u16,
}

impl Wd1002Controller {
    pub fn new(path: &str, trace: bool, trace_log_path: Option<&str>) -> std::io::Result<Self> {
        let image = HardDiskImage::open(path)?;
        let trace_log = if trace {
            let log_path = trace_log_path.unwrap_or("logs/wd1002.log");
            if let Some(parent) = Path::new(log_path).parent() {
                if !parent.as_os_str().is_empty() {
                    create_dir_all(parent)?;
                }
            }
            let file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(log_path)?;
            Some(BufWriter::new(file))
        } else {
            None
        };

        let mut me = Self {
            regs: [0; 8],
            cur_cmd: 0,
            precomp: 0,
            image,
            data_buf: vec![0; SECTOR_SIZE as usize + 4],
            data_len: 0,
            data_ix: 0,
            phase: TransferPhase::Idle,
            pending_offset: 0,
            intrq: false,
            reset_gate_high: false,
            xfer_size: SECTOR_SIZE as usize,
            remaining_sectors: 1,
            trace,
            trace_log,
            idle_data_reads: 0,
            diag_polls_remaining: 0,
            complete_polls_remaining: 0,
            pending_completion: None,
            last_load_offset: 0,
            last_load_sum128: 0,
            last_load_sum_full: 0,
        };
        me.reset();
        Ok(me)
    }

    pub fn reset(&mut self) {
        self.regs = [0; 8];
        self.cur_cmd = 0;
        self.precomp = 0;
        self.data_ix = 0;
        self.data_len = 0;
        self.phase = TransferPhase::Idle;
        self.pending_offset = 0;
        self.intrq = false;
        self.xfer_size = SECTOR_SIZE as usize;
        self.remaining_sectors = 1;

        self.regs[REG_STATUS] = STS_BUSY;
        self.regs[REG_ERROR] = 0;
        self.diag_polls_remaining = RESET_DIAG_POLLS;
        self.complete_polls_remaining = 0;
        self.pending_completion = None;
        self.last_load_offset = 0;
        self.last_load_sum128 = 0;
        self.last_load_sum_full = 0;
        self.log("reset");
    }

    pub fn on_system_port_write(&mut self, bits: u8) {
        // Kaypro 10 firmware pulses bit 1 on port 0x14 to reset WD path.
        let high = bits & 0x02 != 0;
        if high && !self.reset_gate_high {
            self.reset();
        }
        self.reset_gate_high = high;
    }

    pub fn port_in(&mut self, port: u8) -> u8 {
        self.tick();
        let r = (port & 0x07) as usize;
        let val = match r {
            REG_DATA => self.read_data_port(),
            REG_STATUS => {
                self.flush_idle_data_reads();
                self.intrq = false;
                self.regs[r]
            }
            _ => {
                self.flush_idle_data_reads();
                self.regs[r]
            }
        };

        if r == REG_DATA && self.phase != TransferPhase::ReadData {
            self.idle_data_reads = self.idle_data_reads.saturating_add(1);
        } else {
            self.log_line(format!("WD1002 IN  0x{:02X} => 0x{:02X}", port & 0x07, val));
        }
        val
    }

    pub fn port_out(&mut self, port: u8, value: u8) {
        self.tick();
        self.flush_idle_data_reads();
        let r = (port & 0x07) as usize;
        self.log_line(format!("WD1002 OUT 0x{:02X} <= 0x{:02X}", port & 0x07, value));

        match r {
            REG_PRECOMP => {
                self.precomp = value;
                self.regs[REG_PRECOMP] = value;
            }
            REG_CMD => {
                if self.regs[REG_STATUS] & STS_BUSY != 0 {
                    self.fail(ERR_ABORTED, "cmd while busy");
                    return;
                }
                self.cur_cmd = value;
                self.regs[REG_STATUS] &= !STS_ERROR;
                self.regs[REG_STATUS] &= !STS_CORR;
                self.regs[REG_ERROR] = 0;
                self.intrq = false;
                self.log_cmd("process");
                self.process_cmd();
            }
            REG_DATA => self.write_data_port(value),
            REG_SDH => {
                self.regs[REG_SDH] = value;
                if self.drive_ready() {
                    self.regs[REG_STATUS] |= STS_READY;
                } else {
                    self.regs[REG_STATUS] &= !STS_READY;
                }
            }
            _ => self.regs[r] = value,
        }
    }

    pub fn take_intrq(&mut self) -> bool {
        let pending = self.intrq;
        if pending {
            self.intrq = false;
        }
        pending
    }

    /// Advance internal controller timing by one CPU-instruction quantum.
    /// This allows deferred completions (e.g. SEEK) to complete even when
    /// firmware is not actively polling WD registers.
    pub fn step(&mut self) {
        self.tick();
    }

    fn process_cmd(&mut self) {
        self.flush_idle_data_reads();
        self.phase = TransferPhase::Idle;
        self.data_ix = 0;
        self.data_len = 0;

        if self.diag_polls_remaining > 0 {
            self.fail(ERR_ABORTED, "cmd during diagnostics");
            return;
        }

        if !self.selected_winchester() {
            self.fail(ERR_ABORTED, "bad lun");
            return;
        }

        if !self.drive_ready() {
            self.fail(ERR_ABORTED, "drive not ready");
            return;
        }

        if self.regs[REG_STATUS] & STS_WRITE_FAULT != 0 {
            self.fail(ERR_ABORTED, "write fault");
            return;
        }

        self.xfer_size = self.get_sector_size();
        if self.xfer_size > self.data_buf.len() {
            self.data_buf.resize(self.xfer_size + 4, 0);
        }

        match self.cur_cmd & 0xF0 {
            CMD_TEST => {
                self.regs[REG_STATUS] |= STS_BUSY;
                self.regs[REG_ERROR] = 0;
                self.complete_ok("test done");
            }
            CMD_RESTORE => {
                self.regs[REG_STATUS] |= STS_BUSY;
                self.regs[REG_STATUS] |= STS_SEEK_DONE;
                self.regs[REG_CYL_LO] = 0;
                self.regs[REG_CYL_HI] = 0;
                self.complete_ok("restore done");
            }
            CMD_SEEK => {
                self.regs[REG_STATUS] |= STS_BUSY;
                self.complete_polls_remaining = 2;
                self.pending_completion = Some(if self.is_valid_ch() {
                    PendingCompletion::SeekOk
                } else {
                    PendingCompletion::SeekErr(ERR_ID_NOT_FOUND)
                });
                self.log("seek pending");
            }
            CMD_READ => {
                let Some(offset) = self.compute_offset() else {
                    self.fail(ERR_ID_NOT_FOUND, "read id-not-found");
                    return;
                };

                self.regs[REG_STATUS] |= STS_BUSY;
                if self.load_sector(offset).is_err() {
                    self.fail(ERR_CRC, "read load-sector failed");
                    return;
                }

                self.pending_offset = offset;
                self.remaining_sectors = self.init_sector_count();
                self.data_ix = 0;
                self.data_len = self.xfer_size;
                if self.cur_cmd & CMD_LONG != 0 {
                    self.data_buf[self.xfer_size..self.xfer_size + 4].fill(0);
                    self.data_len += 4;
                }
                self.phase = TransferPhase::ReadData;
                self.regs[REG_STATUS] &= !STS_BUSY;
                self.regs[REG_STATUS] |= STS_DRQ;
                // INTRQ is asserted on command completion, not at initial DRQ.
                self.log("read ready");
            }
            CMD_WRITE => {
                let Some(offset) = self.compute_offset() else {
                    self.fail(ERR_ID_NOT_FOUND, "write id-not-found");
                    return;
                };

                self.pending_offset = offset;
                self.remaining_sectors = self.init_sector_count();
                self.data_ix = 0;
                self.data_len = self.xfer_size + if self.cur_cmd & CMD_LONG != 0 { 4 } else { 0 };
                self.phase = TransferPhase::WriteData;
                self.regs[REG_STATUS] |= STS_BUSY | STS_DRQ;
                self.log("write ready");
            }
            CMD_FORMAT_TRACK => {
                if self.compute_track_base_offset().is_none() {
                    self.fail(ERR_ID_NOT_FOUND, "format id-not-found");
                    return;
                }

                self.data_ix = 0;
                self.data_len = self.xfer_size;
                self.phase = TransferPhase::FormatData;
                self.regs[REG_STATUS] |= STS_BUSY | STS_DRQ;
                self.log("format ready");
            }
            _ => self.fail(ERR_ABORTED, "unsupported command"),
        }
    }

    fn read_data_port(&mut self) -> u8 {
        if self.phase != TransferPhase::ReadData || self.data_ix >= self.data_len {
            return self.regs[REG_DATA];
        }

        let v = self.data_buf[self.data_ix];
        self.regs[REG_DATA] = v;
        self.data_ix += 1;

        if self.data_ix < self.data_len {
            self.regs[REG_STATUS] |= STS_DRQ;
            return v;
        }

        if self.cur_cmd & CMD_MULTI != 0 {
            match self.advance_multi_read() {
                Ok(true) => return v,
                Ok(false) => {}
                Err(code) => {
                    self.fail(code, "read multi advance failed");
                    return v;
                }
            }
        }

        self.complete_ok("read done");
        v
    }

    fn write_data_port(&mut self, value: u8) {
        if self.phase == TransferPhase::Idle {
            self.regs[REG_DATA] = value;
            return;
        }

        if self.data_ix >= self.data_len {
            self.complete_ok("write extra byte");
            return;
        }

        self.data_buf[self.data_ix] = value;
        self.data_ix += 1;
        if self.data_ix < self.data_len {
            self.regs[REG_STATUS] |= STS_DRQ;
            return;
        }

        match self.phase {
            TransferPhase::WriteData => {
                let outcome = match self
                    .image
                    .write_controller_sector(
                        self.pending_offset,
                        &self.data_buf[..self.xfer_size],
                        ControllerWriteSource::WriteData,
                    )
                {
                    Ok(outcome) => outcome,
                    Err(_) => {
                        self.fail(ERR_DAM_NOT_FOUND, "write data failed");
                        return;
                    }
                };
                if outcome == ControllerWriteOutcome::PreservedProtectedSector {
                    self.log_line(format!(
                        "WD1002 protected-sector preserve off={} len={}",
                        self.pending_offset, self.xfer_size
                    ));
                }
                if outcome == ControllerWriteOutcome::AppliedProtectedSector {
                    self.log_line(format!(
                        "WD1002 protected-sector accept off={} len={}",
                        self.pending_offset, self.xfer_size
                    ));
                }

                if outcome == ControllerWriteOutcome::Applied
                    || outcome == ControllerWriteOutcome::AppliedProtectedSector
                {
                    self.log("write committed");
                }

                if self.cur_cmd & CMD_MULTI != 0 {
                    match self.advance_multi_write() {
                        Ok(true) => {
                            self.log("write multi advance");
                            return;
                        }
                        Ok(false) => {}
                        Err(code) => {
                            self.fail(code, "write multi advance failed");
                            return;
                        }
                    }
                }

                self.complete_ok("write done");
            }
            TransferPhase::FormatData => {
                if let Err(code) = self.finish_format_track() {
                    self.fail(code, "format failed");
                    return;
                }
                self.complete_ok("format done");
            }
            _ => self.complete_ok("transfer done"),
        }
    }

    fn advance_multi_read(&mut self) -> Result<bool, u8> {
        if self.remaining_sectors > 0 {
            self.remaining_sectors -= 1;
        }
        self.regs[REG_SEC_COUNT] = self.regs[REG_SEC_COUNT].wrapping_sub(1);

        if self.remaining_sectors == 0 {
            return Ok(false);
        }

        self.increment_chs();
        let Some(offset) = self.compute_offset() else {
            return Err(ERR_ID_NOT_FOUND);
        };

        self.load_sector(offset).map_err(|_| ERR_CRC)?;
        self.pending_offset = offset;
        self.data_ix = 0;
        self.data_len = self.xfer_size;
        if self.cur_cmd & CMD_LONG != 0 {
            self.data_buf[self.xfer_size..self.xfer_size + 4].fill(0);
            self.data_len += 4;
        }
        self.regs[REG_STATUS] &= !STS_BUSY;
        self.regs[REG_STATUS] |= STS_DRQ;
        // INTRQ is asserted when the multi-sector command completes.
        Ok(true)
    }

    fn advance_multi_write(&mut self) -> Result<bool, u8> {
        if self.remaining_sectors > 0 {
            self.remaining_sectors -= 1;
        }
        self.regs[REG_SEC_COUNT] = self.regs[REG_SEC_COUNT].wrapping_sub(1);

        if self.remaining_sectors == 0 {
            return Ok(false);
        }

        self.increment_chs();
        let Some(offset) = self.compute_offset() else {
            return Err(ERR_ID_NOT_FOUND);
        };

        self.pending_offset = offset;
        self.data_ix = 0;
        self.data_len = self.xfer_size + if self.cur_cmd & CMD_LONG != 0 { 4 } else { 0 };
        self.regs[REG_STATUS] |= STS_BUSY | STS_DRQ;
        Ok(true)
    }

    fn finish_format_track(&mut self) -> Result<(), u8> {
        let Some(track_base) = self.compute_track_base_offset() else {
            return Err(ERR_ID_NOT_FOUND);
        };

        let logical_spt = self.get_sectors_per_track() as usize;
        let mut fmt_count = self.regs[REG_SEC_COUNT] as usize;
        if fmt_count == 0 {
            fmt_count = logical_spt;
        }
        let sectors_to_format = fmt_count.min(logical_spt);
        let fill = vec![0u8; self.xfer_size];

        // Interleave table bytes in the sector buffer provide the sector IDs
        // to format. Support both 0-based and 1-based host IDs.
        for i in 0..sectors_to_format {
            let id = self.data_buf.get(i).copied().unwrap_or(0) as usize;
            let sec = if id < logical_spt {
                id
            } else if id >= 1 && id <= logical_spt {
                id - 1
            } else {
                return Err(ERR_ID_NOT_FOUND);
            };
            let off = track_base + (sec as u64) * (self.xfer_size as u64);
            match self
                .image
                .write_controller_sector(off, &fill, ControllerWriteSource::FormatTrack)
            {
                Ok(ControllerWriteOutcome::Applied) => {}
                Ok(ControllerWriteOutcome::AppliedProtectedSector) => {
                    self.log_line(format!(
                        "WD1002 protected-sector accept off={} len={} during format-track",
                        off, self.xfer_size
                    ));
                }
                Ok(ControllerWriteOutcome::PreservedProtectedSector) => {
                    self.log_line(format!(
                        "WD1002 protected-sector preserve off={} len={} during format-track",
                        off, self.xfer_size
                    ));
                }
                Err(_) => return Err(ERR_DAM_NOT_FOUND),
            }
        }

        let cyl = ((self.regs[REG_CYL_HI] as u16) << 8) | self.regs[REG_CYL_LO] as u16;
        let head = self.get_head();
        if self.image.set_track_formatted(cyl, head, true).is_err() {
            return Err(ERR_DAM_NOT_FOUND);
        }

        self.regs[REG_SEC_COUNT] = 0;
        Ok(())
    }

    fn load_sector(&mut self, offset: u64) -> std::io::Result<()> {
        self.image.read_at(offset, &mut self.data_buf[..self.xfer_size])?;
        self.last_load_offset = offset;
        self.last_load_sum_full = self.data_buf[..self.xfer_size]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
        let n128 = usize::min(128, self.xfer_size);
        self.last_load_sum128 = self.data_buf[..n128]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
        if self.trace && self.xfer_size >= 8 {
            let tail_a = self.data_buf[self.xfer_size - 2];
            let tail_b = self.data_buf[self.xfer_size - 1];
            self.log_line(format!(
                "WD1002 load off={} len={} csum128={:04X} csum={:04X} b[0..8]={:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} tail={:02X} {:02X}",
                offset,
                self.xfer_size,
                self.last_load_sum128,
                self.last_load_sum_full,
                self.data_buf[0],
                self.data_buf[1],
                self.data_buf[2],
                self.data_buf[3],
                self.data_buf[4],
                self.data_buf[5],
                self.data_buf[6],
                self.data_buf[7],
                tail_a,
                tail_b
            ));
        }
        Ok(())
    }

    fn is_valid_ch(&self) -> bool {
        let cyl = ((self.regs[REG_CYL_HI] as u16) << 8) | self.regs[REG_CYL_LO] as u16;
        let head = self.get_head() as u64;
        (cyl as u64) < crate::hard_disk_image::CYLINDERS && head < HEADS
    }

    fn compute_offset(&self) -> Option<u64> {
        let cyl = ((self.regs[REG_CYL_HI] as u16) << 8) | self.regs[REG_CYL_LO] as u16;
        let head = self.get_head() as u64;
        if (cyl as u64) >= crate::hard_disk_image::CYLINDERS || head >= HEADS {
            return None;
        }

        let sec = self.regs[REG_SECTOR] as u64;
        let logical_spt = self.get_sectors_per_track() as u64;
        // Kaypro10 BIOS/HDFMT program sectors as 0-based logical sectors.
        if sec >= logical_spt {
            return None;
        }

        let track_base = ((cyl as u64) * HEADS + head) * SECTORS_PER_TRACK * SECTOR_SIZE;
        let offset = if self.xfer_size == 128 {
            // Kaypro10 128-byte mode addresses 64 logical sectors using an
            // interleaved 16-sector physical map; each physical sector
            // contains 4 logical 128-byte records.
            let logical = sec as usize;
            let phys_ix = logical / 4;
            let quarter = logical % 4;
            let sec_id = *K10_128_LOGICAL_TO_PHYS.get(phys_ix)? as u64; // 1-based
            track_base + (sec_id - 1) * SECTOR_SIZE + (quarter as u64) * 128
        } else {
            track_base + sec * (self.xfer_size as u64)
        };
        if offset + (self.xfer_size as u64) > track_base + SECTORS_PER_TRACK * SECTOR_SIZE {
            return None;
        }
        Some(offset)
    }

    fn compute_track_base_offset(&self) -> Option<u64> {
        let cyl = ((self.regs[REG_CYL_HI] as u16) << 8) | self.regs[REG_CYL_LO] as u16;
        let head = self.get_head() as u64;
        if (cyl as u64) >= crate::hard_disk_image::CYLINDERS || head >= HEADS {
            return None;
        }
        Some(((cyl as u64) * HEADS + head) * SECTORS_PER_TRACK * SECTOR_SIZE)
    }

    fn get_head(&self) -> u8 {
        self.regs[REG_SDH] & 0x07
    }

    fn get_lun(&self) -> u8 {
        (self.regs[REG_SDH] & 0x18) >> 3
    }

    fn selected_winchester(&self) -> bool {
        // SDH bits 4..3: 00/01/10 = hard disk select, 11 = floppy select.
        self.get_lun() != 0x03
    }

    fn selected_drive(&self) -> u8 {
        self.get_lun()
    }

    fn drive_ready(&self) -> bool {
        self.selected_winchester() && self.selected_drive() == ACTIVE_DRIVE_SELECT
    }

    fn get_sector_size(&self) -> usize {
        match (self.regs[REG_SDH] >> 5) & 0x03 {
            0 => 256,
            1 => 512,
            2 => 1024,
            _ => 128,
        }
    }

    fn get_sectors_per_track(&self) -> u8 {
        match self.get_sector_size() {
            256 => 32,
            512 => 17,
            1024 => 8,
            128 => 64,
            _ => 17,
        }
    }

    fn init_sector_count(&self) -> u16 {
        if self.cur_cmd & CMD_MULTI == 0 {
            1
        } else if self.regs[REG_SEC_COUNT] == 0 {
            256
        } else {
            self.regs[REG_SEC_COUNT] as u16
        }
    }

    fn increment_chs(&mut self) {
        let spt = self.get_sectors_per_track();
        let next_sector = self.regs[REG_SECTOR].wrapping_add(1);
        if next_sector < spt {
            self.regs[REG_SECTOR] = next_sector;
            return;
        }

        self.regs[REG_SECTOR] = 0;
        let mut head = self.get_head().wrapping_add(1);
        if (head as u64) >= HEADS {
            head = 0;
            let cyl = (((self.regs[REG_CYL_HI] as u16) << 8) | self.regs[REG_CYL_LO] as u16).wrapping_add(1);
            self.regs[REG_CYL_LO] = (cyl & 0xFF) as u8;
            self.regs[REG_CYL_HI] = ((cyl >> 8) & 0xFF) as u8;
        }
        self.regs[REG_SDH] = (self.regs[REG_SDH] & !0x07) | (head & 0x07);
    }

    fn complete_ok(&mut self, msg: &str) {
        self.phase = TransferPhase::Idle;
        self.data_ix = 0;
        self.data_len = 0;
        self.regs[REG_STATUS] &= !STS_DRQ;
        self.regs[REG_STATUS] &= !STS_BUSY;
        self.regs[REG_STATUS] &= !STS_WRITE_FAULT;
        self.regs[REG_STATUS] &= !STS_CORR;
        self.regs[REG_STATUS] |= STS_SEEK_DONE;
        if self.drive_ready() {
            self.regs[REG_STATUS] |= STS_READY;
        } else {
            self.regs[REG_STATUS] &= !STS_READY;
        }
        // Kaypro10 host path expects command-completion NMI for transfer
        // commands; SEEK completion is polled via status.
        self.intrq = (self.cur_cmd & 0xF0) != CMD_SEEK;
        self.log(msg);
    }

    fn fail(&mut self, code: u8, msg: &str) {
        self.phase = TransferPhase::Idle;
        self.data_ix = 0;
        self.data_len = 0;
        self.regs[REG_ERROR] = code;
        self.regs[REG_STATUS] |= STS_ERROR;
        self.regs[REG_STATUS] &= !STS_DRQ;
        self.regs[REG_STATUS] &= !STS_BUSY;
        self.regs[REG_STATUS] &= !STS_WRITE_FAULT;
        self.regs[REG_STATUS] &= !STS_CORR;
        self.regs[REG_STATUS] |= STS_SEEK_DONE;
        if self.drive_ready() {
            self.regs[REG_STATUS] |= STS_READY;
        } else {
            self.regs[REG_STATUS] &= !STS_READY;
        }
        self.intrq = true;
        self.log(msg);
    }

    fn tick(&mut self) {
        if self.diag_polls_remaining > 0 {
            self.diag_polls_remaining -= 1;
            if self.diag_polls_remaining == 0 {
                self.regs[REG_STATUS] &= !STS_BUSY;
                self.regs[REG_STATUS] |= STS_SEEK_DONE;
                if self.drive_ready() {
                    self.regs[REG_STATUS] |= STS_READY;
                } else {
                    self.regs[REG_STATUS] &= !STS_READY;
                }
                // WD1002-05/HDO power-up diagnostic code. Kaypro10 tooling
                // expects WD2797 error for HDO.
                self.regs[REG_ERROR] = DIAG_WD2797_ERR;
                self.log("diagnostics complete");
            }
        }
        if self.complete_polls_remaining > 0 {
            self.complete_polls_remaining -= 1;
            if self.complete_polls_remaining == 0 {
                if let Some(done) = self.pending_completion.take() {
                    match done {
                        PendingCompletion::SeekOk => {
                            self.regs[REG_STATUS] |= STS_SEEK_DONE;
                            self.complete_ok("seek done");
                        }
                        PendingCompletion::SeekErr(code) => {
                            self.fail(code, "seek id-not-found");
                        }
                    }
                }
            }
        }
    }

    fn log(&mut self, msg: &str) {
        if !self.trace {
            return;
        }
        self.log_line(format!(
            "WD1002 {} cmd={:02X} sts={:02X} err={:02X} cnt={:02X} sec={:02X} cyl={:02X}{:02X} sdh={:02X} phase={:?} xfer={} rem={} off={}",
            msg,
            self.cur_cmd,
            self.regs[REG_STATUS],
            self.regs[REG_ERROR],
            self.regs[REG_SEC_COUNT],
            self.regs[REG_SECTOR],
            self.regs[REG_CYL_HI],
            self.regs[REG_CYL_LO],
            self.regs[REG_SDH],
            self.phase,
            self.xfer_size,
            self.remaining_sectors,
            self.pending_offset
        ));
    }

    fn log_cmd(&mut self, prefix: &str) {
        if !self.trace {
            return;
        }
        self.log_line(format!(
            "WD1002 {} cmd={:02X} cnt={:02X} sec={:02X} cyl={:02X}{:02X} sdh={:02X}",
            prefix,
            self.cur_cmd,
            self.regs[REG_SEC_COUNT],
            self.regs[REG_SECTOR],
            self.regs[REG_CYL_HI],
            self.regs[REG_CYL_LO],
            self.regs[REG_SDH]
        ));
    }

    fn log_line(&mut self, line: String) {
        if !self.trace {
            return;
        }
        if let Some(log) = self.trace_log.as_mut() {
            let _ = writeln!(log, "{}", line);
            let _ = log.flush();
        }
    }

    fn flush_idle_data_reads(&mut self) {
        if !self.trace || self.idle_data_reads == 0 {
            return;
        }
        let count = self.idle_data_reads;
        self.idle_data_reads = 0;
        self.log_line(format!(
            "WD1002 IN  0x00 idle-repeat x{} (sts={:02X} err={:02X} cnt={:02X} sec={:02X} cyl={:02X}{:02X} sdh={:02X})",
            count,
            self.regs[REG_STATUS],
            self.regs[REG_ERROR],
            self.regs[REG_SEC_COUNT],
            self.regs[REG_SECTOR],
            self.regs[REG_CYL_HI],
            self.regs[REG_CYL_LO],
            self.regs[REG_SDH]
        ));
    }

    pub fn debug_snapshot(&self) -> (u8, u8, u8, u8, u8, u8, usize, usize, u8) {
        let phase = match self.phase {
            TransferPhase::Idle => 0,
            TransferPhase::ReadData => 1,
            TransferPhase::WriteData => 2,
            TransferPhase::FormatData => 3,
        };
        (
            self.cur_cmd,
            self.regs[REG_STATUS],
            self.regs[REG_SEC_COUNT],
            self.regs[REG_SECTOR],
            self.regs[REG_CYL_LO],
            self.regs[REG_SDH],
            self.xfer_size,
            self.data_ix,
            phase,
        )
    }

    pub fn debug_snapshot_ext(&self) -> WdDebugSnapshot {
        let phase = match self.phase {
            TransferPhase::Idle => 0,
            TransferPhase::ReadData => 1,
            TransferPhase::WriteData => 2,
            TransferPhase::FormatData => 3,
        };
        WdDebugSnapshot {
            cmd: self.cur_cmd,
            status: self.regs[REG_STATUS],
            sec_count: self.regs[REG_SEC_COUNT],
            sector: self.regs[REG_SECTOR],
            cyl: ((self.regs[REG_CYL_HI] as u16) << 8) | self.regs[REG_CYL_LO] as u16,
            sdh: self.regs[REG_SDH],
            xfer_size: self.xfer_size,
            data_ix: self.data_ix,
            phase,
            pending_offset: self.pending_offset,
            logical_spt: self.get_sectors_per_track(),
            last_load_offset: self.last_load_offset,
            last_load_sum128: self.last_load_sum128,
            last_load_sum_full: self.last_load_sum_full,
        }
    }
}
