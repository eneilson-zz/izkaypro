use std::fs::{self, File, OpenOptions};
use std::io::Read;
use super::media::{self, *};

// Fallback embedded disk images (used when external files can't be loaded)
static FALLBACK_DISK_DSDD: &[u8] = include_bytes!("../disks/cpm22g-rom292a.img");
static FALLBACK_BLANK_DSDD: &[u8] = include_bytes!("../disks/cpm22-kaypro4-blank.img");

pub enum Drive {
    A = 0,
    B = 1,
}

pub struct FloppyController {
    pub motor_on: bool,
    pub drive: u8,
    side_2: bool,
    track: u8,           // Track register (software-accessible)
    pub head_position: u8,   // Physical head position (moved by STEP commands)
    step_direction: i8,  // Last step direction: 1 = in (towards higher tracks), -1 = out
    sector: u8,
    pub single_density: bool,
    data: u8,
    status: u8,

    media: [Media ;2],

    pub read_index: usize,
    pub read_last: usize,

    data_buffer: Vec<u8>,

    // WRITE TRACK state
    pub write_track_active: bool,
    write_track_buffer: Vec<u8>,
    write_track_remaining: usize,
    write_track_drive: u8,    // Drive latched at WRITE TRACK start
    write_track_side: bool,   // Side latched at WRITE TRACK start
    write_track_head: u8,     // Head position latched at WRITE TRACK start
    multi_sector: bool,

    // Index pulse simulation - counter for toggling index bit
    status_read_count: u32,

    // Counts consecutive status reads without data access while BUSY.
    // When the program stops reading/writing data and just polls status,
    // the transfer has been abandoned and BUSY should clear.
    status_polls_without_data: u8,

    // READ ADDRESS busy countdown: when >0, BUSY remains set.
    // Decremented on each status read; when it reaches 0, BUSY clears
    // and the completion NMI fires. This allows both polling-based ROMs
    // (KayPLUS) and NMI-driven ROMs (81-292a) to work correctly.
    read_address_countdown: u8,

    pub raise_nmi: bool,
    pub trace: bool,
    pub trace_rw: bool,

    // Debug: last FDC command for crash diagnosis
    pub last_command: u8,
    pub last_command_count: u64,
}

#[derive(Copy, Clone)]
#[repr(u8)]
#[allow(dead_code)]
pub enum FDCStatus {
    NotReady = 0x80,
    WriteProtected = 0x40,
    _WriteFault = 0x20,
    SeekErrorOrRecordNotFound = 0x10,
    _CRCError = 0x08,
    LostDataOrTrack0 = 0x04,
    DataRequest = 0x02,
    Busy = 0x01,
    NoError = 0x00,
}

impl FloppyController {
    /// Create a new floppy controller with specified disk images and format.
    /// `side1_sector_base`: sector ID base for side 1 headers (10 = standard Kaypro, 0 = KayPLUS).
    pub fn new(
        disk_a_path: &str,
        disk_b_path: &str,
        default_format: MediaFormat,
        side1_sector_base: u8,
        trace: bool,
        trace_rw: bool,
    ) -> FloppyController {
        let (disk_a_content, disk_a_name, disk_a_file, disk_a_wp) = Self::load_disk_or_fallback(
            disk_a_path,
            FALLBACK_DISK_DSDD,
            "Fallback Boot Disk (DSDD)",
        );
        
        let (disk_b_content, disk_b_name, disk_b_file, disk_b_wp) = Self::load_disk_or_fallback(
            disk_b_path,
            FALLBACK_BLANK_DSDD,
            "Fallback Blank Disk (DSDD)",
        );
        
        let format_a = match media::detect_media_format(disk_a_content.len()) {
            MediaFormat::Unformatted => default_format,
            f => f,
        };
        let format_b = match media::detect_media_format(disk_b_content.len()) {
            MediaFormat::Unformatted => default_format,
            f => f,
        };

        FloppyController {
            motor_on: false,
            drive: 0,
            side_2: false,
            track: 0,
            head_position: 0,
            step_direction: 1, // Default to stepping in
            sector: 0,
            single_density: false,
            data: 0,
            status: 0,
            media: [
                Media {
                    file: disk_a_file,
                    name: disk_a_name,
                    content: disk_a_content,
                    format: format_a,
                    write_protected: disk_a_wp,
                    side1_sector_base,
                    learned_n: None,
                    learned_sector_base: None,
                    track_geometry: std::collections::HashMap::new(),
                    write_min: usize::MAX,
                    write_max: 0,
                },
                Media {
                    file: disk_b_file,
                    name: disk_b_name,
                    content: disk_b_content,
                    format: format_b,
                    write_protected: disk_b_wp,
                    side1_sector_base,
                    learned_n: None,
                    learned_sector_base: None,
                    track_geometry: std::collections::HashMap::new(),
                    write_min: usize::MAX,
                    write_max: 0,
                },
            ],

            read_index: 0,
            read_last: 0,

            data_buffer: Vec::new(),

            write_track_active: false,
            write_track_buffer: Vec::new(),
            write_track_remaining: 0,
            write_track_drive: 0,
            write_track_side: false,
            write_track_head: 0,
            multi_sector: false,

            status_read_count: 0,
            status_polls_without_data: 0,
            read_address_countdown: 0,

            raise_nmi: false,
            trace,
            trace_rw,
            last_command: 0,
            last_command_count: 0,
        }
    }
    
    /// Load a disk image from file, falling back to embedded data if not found.
    /// Returns (content, name, file_handle, write_protected).
    fn load_disk_or_fallback(path: &str, fallback: &'static [u8], fallback_name: &str) -> (Vec<u8>, String, Option<File>, bool) {
        match OpenOptions::new().read(true).write(true).open(path) {
            Ok(mut file) => {
                let mut content = Vec::new();
                match file.read_to_end(&mut content) {
                    Ok(_) => (content, path.to_string(), Some(file), false),
                    Err(e) => {
                        eprintln!("Warning: Could not read disk image '{}': {}, using fallback", path, e);
                        (fallback.to_vec(), fallback_name.to_string(), None, false)
                    }
                }
            }
            Err(_) => {
                match fs::read(path) {
                    Ok(content) => {
                        eprintln!("Note: Disk image '{}' opened read-only (write-protected)", path);
                        (content, path.to_string(), None, true)
                    }
                    Err(_) => {
                        eprintln!("Warning: Could not load disk image '{}', using fallback", path);
                        (fallback.to_vec(), fallback_name.to_string(), None, false)
                    }
                }
            }
        }
    }

    pub fn media_a(&self) -> &Media {
        &self.media[Drive::A as usize]
    }

    pub fn media_b(&self) -> &Media {
        &self.media[Drive::B as usize]
    }

    pub fn media_a_mut(&mut self) -> &mut Media {
        &mut self.media[Drive::A as usize]
    }

    pub fn media_b_mut(&mut self) -> &mut Media {
        &mut self.media[Drive::B as usize]
    }

    pub fn media_selected(&mut self) -> &mut Media {
        &mut self.media[self.drive as usize]
    }

    pub fn set_motor(&mut self, motor_on: bool) {
        self.media_selected().flush_disk();
        self.motor_on = motor_on;
    }

    pub fn set_single_density(&mut self, single_density: bool) {
        self.single_density = single_density;
    }

    pub fn set_side(&mut self, side_2: bool) {
        self.side_2 = side_2;
    }

    pub fn set_drive(&mut self, drive: u8) {
        if drive != self.drive {
            if self.write_track_active {
                if self.trace {
                    println!("FDC: Drive select {} -> {} IGNORED (write track active on drive {})",
                        self.drive, drive, self.write_track_drive);
                }
                return;
            }
            self.media_selected().flush_disk();
            if self.trace {
                println!("FDC: Drive select {} -> {}", self.drive, drive);
            }
            self.drive = drive;
        }
    }

    pub fn put_command(&mut self, command: u8) {
        self.last_command = command;
        self.last_command_count += 1;

        if self.write_track_active && (command & 0xf0) != 0xd0 {
            if self.trace || self.trace_rw {
                println!("FDC: New command 0x{:02x} while write_track_active, finishing write track (buf len={})",
                    command, self.write_track_buffer.len());
            }
            self.finish_write_track();
        }

        self.media_selected().flush_disk();

        if (command & 0xf0) == 0x00 {
            // RESTORE command, type I
            // 0000_hVrr
            if self.trace {
                println!("FDC: Restore");
            }
            self.read_index = 0;
            self.read_last = 0;
            self.track = 0x00;
            self.head_position = 0; // Physical head returns to track 0
            self.status = self.type_i_status(FDCStatus::LostDataOrTrack0 as u8);
            self.raise_nmi = true;

        } else if (command & 0xf0) == 0x10 {
            // SEEK command, type I
            // 0001_hVrr
            let track = self.data;
            if self.trace {
                println!("FDC: Seek track {}", track);
            }
            if self.media_selected().is_valid_track(track) {
                self.track = track;
                self.head_position = track; // Physical head moves to target
                self.status = self.type_i_status(FDCStatus::NoError as u8);
            } else {
                self.status = self.type_i_status(FDCStatus::SeekErrorOrRecordNotFound as u8);
            }
            self.raise_nmi = true;
        } else if (command & 0xe0) == 0x20 {
            // STEP command, type I
            // 001T_hVrr (T=1 updates track register)
            // Moves in same direction as last STEP IN or STEP OUT
            let update_track = (command & 0x10) != 0;
            if self.step_direction > 0 {
                // Step in (towards higher tracks)
                if self.head_position < 39 {
                    self.head_position += 1;
                }
            } else {
                // Step out (towards track 0)
                if self.head_position > 0 {
                    self.head_position -= 1;
                }
            }
            if update_track {
                self.track = self.head_position;
            }
            if self.trace {
                println!("FDC: Step (dir={}, update={}) head={}", 
                    if self.step_direction > 0 { "in" } else { "out" }, 
                    update_track, self.head_position);
            }
            if self.head_position == 0 {
                self.status = self.type_i_status(FDCStatus::LostDataOrTrack0 as u8);
            } else {
                self.status = self.type_i_status(FDCStatus::NoError as u8);
            }
            self.raise_nmi = true;
        } else if (command & 0xe0) == 0x40 {
            // STEP IN command, type I
            // 010T_hVrr (T=1 updates track register)
            let update_track = (command & 0x10) != 0;
            self.step_direction = 1; // Remember direction for STEP command
            // Always move physical head
            if self.head_position < 39 {
                self.head_position += 1;
            }
            if update_track {
                self.track = self.head_position;
            }
            if self.trace {
                println!("FDC: Step in (update={}) head={}", update_track, self.head_position);
            }
            self.status = self.type_i_status(FDCStatus::NoError as u8);
            self.raise_nmi = true;
        } else if (command & 0xe0) == 0x60 {
            // STEP OUT command, type I
            // 011T_hVrr (T=1 updates track register)
            let update_track = (command & 0x10) != 0;
            self.step_direction = -1; // Remember direction for STEP command
            // Always move physical head
            if self.head_position > 0 {
                self.head_position -= 1;
            }
            if update_track {
                self.track = self.head_position;
            }
            if self.trace {
                println!("FDC: Step out (update={}) head={}", update_track, self.head_position);
            }
            if self.head_position == 0 {
                self.status = self.type_i_status(FDCStatus::LostDataOrTrack0 as u8);
            } else {
                self.status = self.type_i_status(FDCStatus::NoError as u8);
            }
            self.raise_nmi = true;
        } else if (command & 0xe0) == 0x80 {
            // READ SECTOR command, type II
            // 100mSEC0
            self.multi_sector = (command & 0x10) != 0;
            let side_compare = (command & 0x02) != 0;
            let side_flag = if (command & 0x08) != 0 { 1u8 } else { 0u8 };
            if self.trace || self.trace_rw {
                println!("FDC: Read sector (cmd:0x{:02x}, Si:{}, Tr:{}, Se:{}, Head:{}, multi:{}, C:{}, S:{})",
                    command, self.side_2, self.track, self.sector, self.head_position,
                    self.multi_sector, side_compare, side_flag);
            }

            let side_2 = self.side_2;
            let track = self.head_position; // Use physical head position for disk access
            let sector = self.sector;
            let (valid, index, last) =  self.media_selected().sector_index(side_2, track, sector);
            if valid {
                self.read_index = index;
                self.read_last = last;
                self.status = FDCStatus::Busy as u8;
                if self.trace || self.trace_rw {
                    println!("FDC: Read sector setup: index={}, last={}, transfer_size={}", index, last, last - index);
                }
            } else {
                // Real WD1793: the FDC searches for the sector ID as the disk
                // rotates. After ~5 revolutions without finding it, BUSY clears
                // and RNF is set. We set BUSY initially so the program's NMI
                // handler has time to set up before the completion NMI fires.
                // BUSY will clear on the next status poll (no data to transfer).
                self.status = FDCStatus::Busy as u8 | FDCStatus::SeekErrorOrRecordNotFound as u8;
                if self.trace || self.trace_rw {
                    println!("FDC: Read sector FAILED: sector {} not found", sector);
                }
            }
            self.raise_nmi = true;

            } else if (command & 0xe0) == 0xa0 {
            // WRITE SECTOR command, type II
            // 101mSECa
            self.multi_sector = (command & 0x10) != 0;
            // a0 (bit 0): 0=normal data mark (FB), 1=deleted data mark (F8)
            // We accept both but don't distinguish in sector images
            if self.trace || self.trace_rw {
                println!("FDC: Write sector (Si:{}, Tr:{}, Se:{}, Head:{}, multi:{})", self.side_2, self.track, self.sector, self.head_position, self.multi_sector);
            }

            if self.media_selected().is_write_protected() {
                self.status = FDCStatus::WriteProtected as u8;
                self.raise_nmi = true;
                return;
            }

            let side_2 = self.side_2;
            let track = self.head_position; // Use physical head position for disk access
            let sector = self.sector;
            let (valid, index, last) =  self.media_selected().sector_index(side_2, track, sector);
            if valid {
                self.read_index = index;
                self.read_last = last;
                self.status = FDCStatus::Busy as u8;
            } else {
                self.status = FDCStatus::Busy as u8 | FDCStatus::SeekErrorOrRecordNotFound as u8;
            }
            self.raise_nmi = true;

        } else if (command & 0xf0) == 0xc0 {
            // READ ADDRESS command, type III
            // 1100_0E00
            let side_2 = self.side_2;
            let track = self.head_position; // Use physical head position
            let sector = self.sector;

            let (valid, base_sector_id) = self.media_selected().read_address(side_2, track, sector);
            if valid {
                let rotation_pos = (self.status_read_count / 10) as u8 % 10;
                let sector_id = base_sector_id + rotation_pos;
                
                if self.trace {
                    println!("FDC: Read address ({},{},{}) -> sector_id={}", side_2, track, sector, sector_id);
                }
                // WD1793 datasheet: "The track address of the ID field is written
                // into the sector register so that a comparison can be made by the user."
                self.sector = self.head_position;
                self.data_buffer.clear();
                self.data_buffer.push(self.head_position);
                self.data_buffer.push(0);
                self.data_buffer.push(sector_id);
                self.data_buffer.push(2);
                self.data_buffer.push(0xde);
                self.data_buffer.push(0xad);
                // Set BUSY - the real WD1793 stays busy while scanning for the
                // next sector ID. BUSY clears after the ID field is read.
                // We use a countdown decremented on status reads so that
                // polling ROMs (KayPLUS) see BUSY for a realistic duration.
                self.status = FDCStatus::Busy as u8;
                self.read_address_countdown = 10;
                self.raise_nmi = true;
            } else {
                if self.trace {
                    println!("FDC: Read address ({},{},{}) = Error", side_2, track, sector);
                }
                // Real WD1793: stays BUSY while scanning for sector headers
                // (~5 revolutions), then clears BUSY and sets RNF. Use the
                // same countdown as the success path so the BIOS sees the
                // BUSY→not-BUSY transition it expects before checking error bits.
                self.status = FDCStatus::Busy as u8 | FDCStatus::SeekErrorOrRecordNotFound as u8;
                self.read_address_countdown = 10;
                self.raise_nmi = true;
            }
        } else if (command & 0xf0) == 0xd0 {
            // FORCE INTERRUPT command, type IV
            // 1101_IIII
            let interrupts = command & 0x0f;
            if self.trace {
                println!("FDC: Force interrupt {:04b}", interrupts);
            }

            if self.write_track_active {
                if self.trace || self.trace_rw {
                    println!("FDC: Force interrupt while write_track_active, buf len={}", self.write_track_buffer.len());
                }
                self.finish_write_track();
            }

            // Terminate current command
            self.read_index = 0;
            self.read_last = 0;
            self.data_buffer.clear();
            self.multi_sector = false;
            self.read_address_countdown = 0;
            self.status &= !(FDCStatus::Busy as u8);

            // I3: Immediate interrupt
            // I0-I2: Conditional interrupts (not-ready, ready-to-not-ready, index pulse)
            // We generate interrupt for I3 or any non-zero I value
            if interrupts != 0 {
                self.raise_nmi = true;
            }
        } else if (command & 0xf0) == 0xe0 {
            // READ TRACK command, type III
            // 1110_0E00
            if self.trace {
                println!("FDC: Read track (not implemented, returning empty)");
            }
            self.status = FDCStatus::NoError as u8;
            self.raise_nmi = true;
        } else if (command & 0xf0) == 0xf0 {
            // WRITE TRACK command, type III (format track)
            // 1111_0E00
            if self.media_selected().is_write_protected() {
                self.status = FDCStatus::WriteProtected as u8;
                self.raise_nmi = true;
                return;
            }
            if self.trace || self.trace_rw {
                println!("FDC: Write track (Drive:{}, Si:{}, Tr:{}, Head:{}, SD:{})", self.drive, self.side_2, self.head_position, self.head_position, self.single_density);
            }
            self.write_track_active = true;
            self.write_track_drive = self.drive;
            self.write_track_side = self.side_2;
            self.write_track_head = self.head_position;
            self.write_track_buffer.clear();
            self.write_track_remaining = if self.single_density { 3125 } else { 0 };
            self.status = FDCStatus::Busy as u8;
            self.raise_nmi = true;
        } else {
            if self.trace {
                println!("FDC: ${:02x} command not implemented", command);
            }
        }
    }

    pub fn get_status(&mut self) -> u8 {
        self.status_read_count = self.status_read_count.wrapping_add(1);

        // READ ADDRESS busy countdown: the WD1793 stays BUSY while scanning
        // for the next sector ID field. We decrement on each status poll;
        // when it reaches 0, BUSY clears and the completion NMI fires.
        if self.read_address_countdown > 0 {
            self.read_address_countdown -= 1;
            if self.read_address_countdown == 0 {
                // Clear BUSY but preserve error bits (e.g., RNF from
                // READ ADDRESS on a non-existent side).
                self.status &= !(FDCStatus::Busy as u8);
            }
        }

        let mut status = self.status;
        if self.status & FDCStatus::Busy as u8 != 0 {
            // Type II/III status: bit 1 = DRQ (data ready for CPU to read/write)
            if self.read_index < self.read_last || !self.data_buffer.is_empty() || self.write_track_active {
                self.status_polls_without_data += 1;
                if self.status_polls_without_data < 10 {
                    status |= FDCStatus::DataRequest as u8;
                } else {
                    self.read_index = 0;
                    self.read_last = 0;
                    self.data_buffer.clear();
                    self.multi_sector = false;
                    self.status = FDCStatus::NoError as u8;
                    self.status_polls_without_data = 0;
                    status = self.status;
                }
            } else if self.read_address_countdown > 0 {
                status |= FDCStatus::DataRequest as u8;
            } else {
                self.status &= !(FDCStatus::Busy as u8);
                self.status_polls_without_data = 0;
                status = self.status;
            }
        } else {
            // Type I status (or idle): bit 1 = Index pulse
            if self.motor_on && (self.status_read_count % 100) < 5 {
                status |= 0x02;
            }
        }
        if (self.trace || self.trace_rw) && status & 0x1c != 0 {
            println!("FDC: Status error bits: {:02x} (busy:{}, drq:{}, lost:{}, crc:{}, rnf:{})",
                status,
                status & 0x01 != 0,
                status & 0x02 != 0,
                status & 0x04 != 0,
                status & 0x08 != 0,
                status & 0x10 != 0);
        }
        status
    }

    pub fn put_track(&mut self, value: u8) {
        self.track = value;
        if self.trace {
            println!("FDC: Set track {}", value);
        }
    }

    pub fn get_track(&self) -> u8 {
        self.track
    }

    pub fn put_sector(&mut self, value: u8) {
        self.sector = value;
        if self.trace {
            println!("FDC: Set sector {}", value);
        }
    }

    pub fn get_sector(&self) -> u8 {
        self.sector
    }

    pub fn put_data(&mut self, value: u8) {
        self.data = value;
        self.status_polls_without_data = 0;

        if self.write_track_active {
            self.write_track_buffer.push(value);
            self.raise_nmi = true;
            if self.write_track_remaining > 0 {
                self.write_track_remaining -= 1;
                if self.write_track_remaining == 0 {
                    if self.trace || self.trace_rw {
                        println!("FDC: WriteTrack finishing (countdown reached 0, buf len={})", self.write_track_buffer.len());
                    }
                    self.finish_write_track();
                }
            } else if self.write_track_buffer.len() >= 12000 {
                if self.trace || self.trace_rw {
                    println!("FDC: WriteTrack finishing (safety limit 12000, buf len={})", self.write_track_buffer.len());
                }
                self.finish_write_track();
            }
            return;
        }

        if self.read_index < self.read_last {
            // Store byte
            let index = self.read_index;
            let data = self.data;
            self.media_selected().write_byte(index, data);
            self.read_index += 1;
            self.raise_nmi = true;
            if self.read_index == self.read_last {
                // We are done writing this sector
                self.media_selected().flush_disk();
                if self.trace {
                    println!("FDC: Set data completed ${:02x} {}-{}-{}", self.data, self.read_index, self.read_last, self.sector);
                }
                if self.multi_sector {
                    // Auto-increment sector and continue to next
                    self.sector += 1;
                    let side_2 = self.side_2;
                    let track = self.head_position;
                    let sector = self.sector;
                    let (valid, index, last) = self.media[self.drive as usize]
                        .sector_index(side_2, track, sector);
                    if valid {
                        self.read_index = index;
                        self.read_last = last;
                        if self.trace {
                            println!("FDC: Multi-sector write continuing with sector {}", self.sector);
                        }
                    } else {
                        // No more valid sectors - complete
                        self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
                        self.read_index = 0;
                        self.read_last = 0;
                        self.multi_sector = false;
                    }
                } else {
                    // Don't clear BUSY yet — same as get_data(): the program
                    // may write more bytes than our 512-byte sector holds.
                    // BUSY will be cleared when get_status() is polled.
                    self.read_index = 0;
                    self.read_last = 0;
                }
            }
        } else if self.status & FDCStatus::Busy as u8 != 0
            && self.read_index == 0 && self.read_last == 0
            && !self.write_track_active
        {
            // Sector data exhausted on write: accept and discard overflow
            // bytes, keep raising NMI so the HALT/OUTI loop doesn't hang.
            self.raise_nmi = true;
        }
    }

    pub fn get_data(&mut self) -> u8 {
        self.status_polls_without_data = 0;
        if !self.data_buffer.is_empty() {
            self.data = self.data_buffer[0];
            self.data_buffer.remove(0);
            self.raise_nmi = true;
        } else if self.read_index < self.read_last {
            // Prepare next byte
            let index = self.read_index;
            self.data = self.media_selected().read_byte(index);
            self.read_index += 1;
            self.raise_nmi = true;
            if self.read_index == self.read_last {
                // We are done reading this sector's actual data.
                if self.trace {
                    println!("FDC: Get data completed ${:02x} {}-{}-{}", self.data, self.read_index, self.read_last, self.sector);
                }
                if self.trace || self.trace_rw {
                    let track = self.head_position;
                    let side = self.side_2;
                    let sector_size = if let Some(geom) = self.media[self.drive as usize].track_geometry.get(&(track, side)) {
                        128usize << (geom.n as usize)
                    } else {
                        self.media[self.drive as usize].sector_size()
                    };
                    let start = self.read_last.saturating_sub(sector_size);
                    let b0 = self.media[self.drive as usize].read_byte(start);
                    let b1 = self.media[self.drive as usize].read_byte(start + 1);
                    let b2 = self.media[self.drive as usize].read_byte(start + 2);
                    let b3 = self.media[self.drive as usize].read_byte(start + 3);
                    println!("FDC: Verify read sector {}: first 4 bytes = {:02x} {:02x} {:02x} {:02x}",
                        self.sector, b0, b1, b2, b3);
                }
                if self.multi_sector {
                    // Auto-increment sector and continue to next
                    self.sector += 1;
                    let side_2 = self.side_2;
                    let track = self.head_position;
                    let sector = self.sector;
                    let (valid, index, last) = self.media[self.drive as usize]
                        .sector_index(side_2, track, sector);
                    if valid {
                        self.read_index = index;
                        self.read_last = last;
                        if self.trace {
                            println!("FDC: Multi-sector read continuing with sector {}", self.sector);
                        }
                    } else {
                        // No more valid sectors - complete
                        self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
                        self.read_index = 0;
                        self.read_last = 0;
                        self.multi_sector = false;
                    }
                } else {
                    // Don't clear BUSY yet — the program may expect more bytes
                    // than our fixed 512-byte sector size (e.g. non-Kaypro formats
                    // with 1024-byte sectors). Keep BUSY set so get_data() can
                    // continue feeding dummy bytes. BUSY will be cleared when
                    // get_status() detects no more data to transfer.
                    self.read_index = 0;
                    self.read_last = 0;
                }
            }
        } else if self.status & FDCStatus::Busy as u8 != 0
            && self.read_index == 0 && self.read_last == 0
            && self.data_buffer.is_empty()
            && !self.write_track_active
        {
            // Sector data exhausted but BUSY still set. Keep raising NMI
            // so the HALT/INI loop doesn't hang (the program needs a
            // completion NMI to exit the transfer). Retain the last data
            // byte in self.data (don't overwrite with 0x00) so that any
            // extra INI reads get the same value as the final real byte,
            // avoiding verify corruption.
            self.raise_nmi = true;
        }

        self.data
    }

    fn type_i_status(&self, base_status: u8) -> u8 {
        let mut status = base_status;
        if self.media[self.drive as usize].is_write_protected() {
            status |= FDCStatus::WriteProtected as u8;
        }
        if self.motor_on {
            status |= 0x20; // S5: Head Loaded
        }
        status
    }

    fn finish_write_track(&mut self) {
        let drive = self.write_track_drive as usize;
        let side_2 = self.write_track_side;
        let track = self.write_track_head;

        if side_2 && !self.media[drive].double_sided() {
            if self.trace || self.trace_rw {
                println!("FDC: WriteTrack side 1 on single-sided disk — upgrading to DSDD");
            }
            self.media[drive].upgrade_to_double_sided();
        }

        let buf = &self.write_track_buffer;
        let mut sectors_written = 0;
        let mut track_learned_n: Option<u8> = None;
        let mut min_sector_id: u8 = 255;
        let mut sector_count: u8 = 0;

        // Sync byte depends on density:
        //   MFM (double density): F5 F5 F5 = A1 sync with missing clock
        //   FM  (single density): 00 00 00 = zero-fill gap
        let sync = if self.single_density { 0x00u8 } else { 0xF5u8 };

        // First pass: learn geometry from IDAMs before writing
        // (so sector_index uses correct per-track values)
        {
            let mut i = 0;
            while i + 3 < buf.len() {
                if buf[i] == sync && buf[i+1] == sync && buf[i+2] == sync && buf[i+3] == 0xFE {
                    i += 4;
                    if i + 4 >= buf.len() { break; }
                    let id_sector = buf[i+2];
                    let id_n = buf[i+3];
                    if id_sector < min_sector_id {
                        min_sector_id = id_sector;
                    }
                    if track_learned_n.is_none() {
                        track_learned_n = Some(id_n);
                    }
                    sector_count += 1;
                    i += 4;
                    // Skip past CRC marker, DAM, data, CRC marker
                    let stream_size = 128usize << (id_n as usize);
                    if i < buf.len() && buf[i] == 0xF7 { i += 1; }
                    while i + 3 < buf.len() {
                        if buf[i] == sync && buf[i+1] == sync && buf[i+2] == sync
                            && (buf[i+3] == 0xFB || buf[i+3] == 0xF8) {
                            i += 4;
                            break;
                        }
                        i += 1;
                    }
                    i += stream_size;
                    if i < buf.len() && buf[i] == 0xF7 { i += 1; }
                } else {
                    i += 1;
                }
            }
        }

        // Record per-track geometry
        if let Some(n) = track_learned_n {
            let geom = super::media::TrackGeometry {
                n,
                sector_count,
                sector_base: min_sector_id,
            };
            self.media[drive].track_geometry.insert((track, side_2), geom);

            // Set global learned_n as fallback (use the first non-trivial N seen)
            if self.media[drive].learned_n.is_none() {
                self.media[drive].learned_n = Some(n);
            }
            if self.media[drive].learned_sector_base.is_none() {
                self.media[drive].learned_sector_base = Some(min_sector_id);
            } else if Some(min_sector_id) < self.media[drive].learned_sector_base {
                self.media[drive].learned_sector_base = Some(min_sector_id);
            }
        }

        // Second pass: write sector data to image
        let mut i = 0;
        while i + 3 < buf.len() {
            if buf[i] == sync && buf[i+1] == sync && buf[i+2] == sync && buf[i+3] == 0xFE {
                i += 4;
                if i + 4 >= buf.len() { break; }

                let _id_track = buf[i];
                let _id_head = buf[i+1];
                let id_sector = buf[i+2];
                let id_n = buf[i+3];
                let stream_size = 128usize << (id_n as usize);
                i += 4;

                if i < buf.len() && buf[i] == 0xF7 { i += 1; }

                // Scan forward for DAM
                while i + 3 < buf.len() {
                    if buf[i] == sync && buf[i+1] == sync && buf[i+2] == sync
                        && (buf[i+3] == 0xFB || buf[i+3] == 0xF8) {
                        i += 4;
                        break;
                    }
                    i += 1;
                }

                let write_len = stream_size;
                if i + stream_size > buf.len() { break; }

                let (valid, index, _last) = self.media[drive]
                    .sector_index(side_2, track, id_sector);
                if valid {
                    if self.trace || self.trace_rw {
                        println!("FDC: WriteTrack sector {} at buf[{}], media index {}, len {}, id_n={}, stream_size={}",
                            id_sector, i, index, write_len, id_n, stream_size);
                    }
                    for j in 0..write_len {
                        self.media[drive].write_byte(index + j, buf[i + j]);
                    }
                    sectors_written += 1;
                } else if self.trace || self.trace_rw {
                    println!("FDC: WriteTrack sector {} at buf[{}] INVALID (sector_index returned false)",
                        id_sector, i);
                }
                i += stream_size;

                if i < buf.len() && buf[i] == 0xF7 { i += 1; }
            } else {
                i += 1;
            }
        }

        if self.trace || self.trace_rw {
            println!("FDC: Write track complete (Si:{}, Tr:{}, {} sectors, SD:{}, buf:{})",
                side_2, track, sectors_written, self.single_density, buf.len());
        }

        if track_learned_n.is_some() && min_sector_id != 255 {
            let media = &self.media[drive];
            if self.trace || self.trace_rw {
                println!("FDC: Track {}/{} geometry: N={}, sectors={}, sector_base={}",
                    track, if side_2 { "S1" } else { "S0" },
                    track_learned_n.unwrap(), sector_count, min_sector_id);
            }
            // Also log global fallback
            if self.trace || self.trace_rw {
                println!("FDC: Global fallback geometry: N={}, sector_size={}, sectors_per_side={}, sector_base={}",
                    media.learned_n.unwrap_or(255), media.sector_size(),
                    media.sectors_per_side(), media.sector_id_base());
            }
        }

        self.media[drive].flush_disk();
        self.write_track_active = false;
        self.status = FDCStatus::NoError as u8;
        self.raise_nmi = true;
    }
}
