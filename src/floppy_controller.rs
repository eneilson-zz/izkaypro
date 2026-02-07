use std::fs::{self, File, OpenOptions};
use std::io::Read;
use super::media::*;

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
        let (disk_a_content, disk_a_name, disk_a_file) = Self::load_disk_or_fallback(
            disk_a_path,
            FALLBACK_DISK_DSDD,
            "Fallback Boot Disk (DSDD)",
        );
        
        let (disk_b_content, disk_b_name, disk_b_file) = Self::load_disk_or_fallback(
            disk_b_path,
            FALLBACK_BLANK_DSDD,
            "Fallback Blank Disk (DSDD)",
        );
        
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
                    format: default_format,
                    side1_sector_base,
                    write_min: usize::MAX,
                    write_max: 0,
                },
                Media {
                    file: disk_b_file,
                    name: disk_b_name,
                    content: disk_b_content,
                    format: default_format,
                    side1_sector_base,
                    write_min: usize::MAX,
                    write_max: 0,
                },
            ],

            read_index: 0,
            read_last: 0,

            data_buffer: Vec::new(),

            write_track_active: false,
            write_track_buffer: Vec::new(),
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
    /// Returns (content, name, file_handle). File handle is Some for writable files.
    fn load_disk_or_fallback(path: &str, fallback: &'static [u8], fallback_name: &str) -> (Vec<u8>, String, Option<File>) {
        match OpenOptions::new().read(true).write(true).open(path) {
            Ok(mut file) => {
                let mut content = Vec::new();
                match file.read_to_end(&mut content) {
                    Ok(_) => (content, path.to_string(), Some(file)),
                    Err(e) => {
                        eprintln!("Warning: Could not read disk image '{}': {}, using fallback", path, e);
                        (fallback.to_vec(), fallback_name.to_string(), None)
                    }
                }
            }
            Err(_) => {
                match fs::read(path) {
                    Ok(content) => {
                        eprintln!("Note: Disk image '{}' opened read-only (write-protected)", path);
                        (content, path.to_string(), None)
                    }
                    Err(_) => {
                        eprintln!("Warning: Could not load disk image '{}', using fallback", path);
                        (fallback.to_vec(), fallback_name.to_string(), None)
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
        self.media_selected().flush_disk();
        self.drive = drive;
    }

    // Diagnostic accessors for crash tracing
    pub fn get_status_raw(&self) -> u8 { self.status }
    pub fn write_track_buf_len(&self) -> usize { self.write_track_buffer.len() }
    pub fn data_buffer_len(&self) -> usize { self.data_buffer.len() }
    pub fn side_2(&self) -> bool { self.side_2 }

    pub fn put_command(&mut self, command: u8) {
        self.last_command = command;
        self.last_command_count += 1;
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
            if self.trace || self.trace_rw {
                println!("FDC: Read sector (Si:{}, Tr:{}, Se:{}, Head:{}, multi:{})", self.side_2, self.track, self.sector, self.head_position, self.multi_sector);
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
                self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
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
                self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
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
                self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
                self.read_address_countdown = 0;
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
                println!("FDC: Write track (Si:{}, Tr:{}, Head:{})", self.side_2, self.head_position, self.head_position);
            }
            self.write_track_active = true;
            self.write_track_buffer.clear();
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
                self.status = FDCStatus::NoError as u8;
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
            } else {
                self.status = FDCStatus::NoError as u8;
                self.status_polls_without_data = 0;
                status = self.status;
            }
        } else {
            // Type I status (or idle): bit 1 = Index pulse
            if self.motor_on && (self.status_read_count % 100) < 5 {
                status |= 0x02;
            }
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
            if self.write_track_buffer.len() >= 12000 {
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
            // Sector data exhausted but BUSY still set: the program expects
            // more bytes than our 512-byte sectors contain (e.g. non-Kaypro
            // formats with 128/256/1024-byte sectors). Feed dummy bytes and
            // keep raising NMI so the HALT/INI loop doesn't hang. BUSY will
            // be cleared when the program polls status via get_status().
            self.data = 0x00;
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
        let side_2 = self.side_2;
        let track = self.head_position;
        let buf = &self.write_track_buffer;
        let mut sectors_written = 0;

        // Parse the WD1793 WRITE TRACK data stream:
        // IDAM sequence: F5 F5 F5 FE C H R N F7
        // DAM sequence:  F5 F5 F5 FB/F8 <data> F7
        // F5 = sync A1 (missing clock), F7 = CRC generation (not literal)
        let mut i = 0;
        while i + 3 < buf.len() {
            // Look for IDAM: F5 F5 F5 FE
            if buf[i] == 0xF5 && buf[i+1] == 0xF5 && buf[i+2] == 0xF5 && buf[i+3] == 0xFE {
                i += 4; // Skip sync + IDAM mark
                if i + 4 >= buf.len() { break; }

                let _id_track = buf[i];
                let _id_head = buf[i+1];
                let id_sector = buf[i+2];
                let id_n = buf[i+3];
                let sector_size = 128usize << (id_n as usize);
                i += 4; // Skip CHRN

                // Skip CRC marker (F7)
                if i < buf.len() && buf[i] == 0xF7 { i += 1; }

                // Scan forward for DAM: F5 F5 F5 FB or F5 F5 F5 F8
                while i + 3 < buf.len() {
                    if buf[i] == 0xF5 && buf[i+1] == 0xF5 && buf[i+2] == 0xF5
                        && (buf[i+3] == 0xFB || buf[i+3] == 0xF8) {
                        i += 4; // Skip sync + DAM
                        break;
                    }
                    i += 1;
                }

                // Read sector data
                if i + sector_size > buf.len() { break; }

                let (valid, index, _last) = self.media[self.drive as usize]
                    .sector_index(side_2, track, id_sector);
                if valid {
                    for j in 0..sector_size {
                        self.media[self.drive as usize].write_byte(index + j, buf[i + j]);
                    }
                    sectors_written += 1;
                }
                i += sector_size;

                // Skip CRC marker (F7)
                if i < buf.len() && buf[i] == 0xF7 { i += 1; }
            } else {
                i += 1;
            }
        }

        if self.trace || self.trace_rw {
            println!("FDC: Write track complete (Si:{}, Tr:{}, {} sectors formatted)",
                side_2, track, sectors_written);
        }

        self.media[self.drive as usize].flush_disk();
        self.write_track_active = false;
        self.status = FDCStatus::NoError as u8;
        self.raise_nmi = true;
    }
}
