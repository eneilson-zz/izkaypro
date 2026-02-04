use std::fs;
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
    head_position: u8,   // Physical head position (moved by STEP commands)
    step_direction: i8,  // Last step direction: 1 = in (towards higher tracks), -1 = out
    sector: u8,
    pub single_density: bool,
    data: u8,
    status: u8,

    media: [Media ;2],

    read_index: usize,
    read_last: usize,

    data_buffer: Vec<u8>,

    // Index pulse simulation - counter for toggling index bit
    status_read_count: u32,

    pub raise_nmi: bool,
    pub trace: bool,
    pub trace_rw: bool
}

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum FDCStatus {
    _NotReady = 0x80,
    _WriteProtected = 0x40,
    _WriteFault = 0x20,
    SeekErrorOrRecordNotFound = 0x10,
    _CRCError = 0x08,
    LostDataOrTrack0 = 0x04,
    _DataRequest = 0x02,
    Busy = 0x01,
    NoError = 0x00,
}

impl FloppyController {
    /// Create a new floppy controller with specified disk images and format
    pub fn new(
        disk_a_path: &str,
        disk_b_path: &str,
        default_format: MediaFormat,
        trace: bool,
        trace_rw: bool,
    ) -> FloppyController {
        // Load disk A from file, fall back to embedded if not found
        let (disk_a_content, disk_a_name) = Self::load_disk_or_fallback(
            disk_a_path,
            FALLBACK_DISK_DSDD,
            "Fallback Boot Disk (DSDD)",
        );
        
        // Load disk B from file, fall back to embedded if not found
        let (disk_b_content, disk_b_name) = Self::load_disk_or_fallback(
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
                    file: None,
                    name: disk_a_name,
                    content: disk_a_content,
                    format: default_format,
                    write_min: usize::MAX,
                    write_max: 0,
                },
                Media {
                    file: None,
                    name: disk_b_name,
                    content: disk_b_content,
                    format: default_format,
                    write_min: usize::MAX,
                    write_max: 0,
                },
            ],

            read_index: 0,
            read_last: 0,

            data_buffer: Vec::new(),

            status_read_count: 0,

            raise_nmi: false,
            trace,
            trace_rw,
        }
    }
    
    /// Load a disk image from file, falling back to embedded data if not found
    fn load_disk_or_fallback(path: &str, fallback: &'static [u8], fallback_name: &str) -> (Vec<u8>, String) {
        match fs::read(path) {
            Ok(content) => (content, path.to_string()),
            Err(_) => {
                eprintln!("Warning: Could not load disk image '{}', using fallback", path);
                (fallback.to_vec(), fallback_name.to_string())
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

    pub fn put_command(&mut self, command: u8) {
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
            self.status = FDCStatus::LostDataOrTrack0 as u8;
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
                self.status = FDCStatus::NoError as u8;
            } else {
                self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
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
                self.status = FDCStatus::LostDataOrTrack0 as u8;
            } else {
                self.status = FDCStatus::NoError as u8;
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
            self.status = FDCStatus::NoError as u8;
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
                self.status = FDCStatus::LostDataOrTrack0 as u8;
            } else {
                self.status = FDCStatus::NoError as u8;
            }
            self.raise_nmi = true;
        } else if (command & 0xe0) == 0x80 {
            // READ SECTOR command, type II
            // 100mFEFx
            if command & 0x10 != 0 {
                panic!("Multiple sector reads not supported")
            }
            if self.trace || self.trace_rw {
                println!("FDC: Read sector (Si:{}, Tr:{}, Se:{}, Head:{})", self.side_2, self.track, self.sector, self.head_position);
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
            // 101mFEFa
            if command & 0x10 != 0 {
                panic!("Multiple sector writes not supported")
            }
            if command & 0x01 != 0 {
                panic!("Delete data mark not supported")
            }
            if self.trace || self.trace_rw {
                println!("FDC: Write sector (Si:{}, Tr:{}, Se:{}, Head:{})", self.side_2, self.track, self.sector, self.head_position);
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

            let (valid, _base_sector_id) = self.media_selected().read_address(side_2, track, sector);
            if valid {
                // Simulate disk rotation - return different sector IDs each time
                // This is needed for Turbo ROM which polls READ ADDRESS to find sectors
                // Use status_read_count to rotate through sectors 0-9 (side 0) or 10-19 (side 1)
                let rotation_pos = (self.status_read_count / 10) as u8 % 10;
                let sector_id = if side_2 {
                    10 + rotation_pos  // Sectors 10-19 on side 1
                } else {
                    rotation_pos       // Sectors 0-9 on side 0
                };
                
                if self.trace {
                    println!("FDC: Read address ({},{},{}) -> sector_id={}", side_2, track, sector, sector_id);
                }
                // Note: Real WD1793 does NOT modify sector register during READ ADDRESS
                self.status = FDCStatus::NoError as u8;
                self.data_buffer.clear();
                self.data_buffer.push(self.head_position); // Physical track from sector header
                self.data_buffer.push(0); // Kaypro 4-84: head byte is always 0 in sector ID
                self.data_buffer.push(sector_id);
                self.data_buffer.push(2); // For sector size 512
                self.data_buffer.push(0xde); // CRC 1
                self.data_buffer.push(0xad); // CRC 2
            } else {
                if self.trace {
                    println!("FDC: Read address ({},{},{}) = Error", side_2, track, sector);
                }
                self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
                self.data_buffer.push(0);
                self.data_buffer.push(0);
                self.data_buffer.push(0);
                self.data_buffer.push(0);
                self.data_buffer.push(0);
                self.data_buffer.push(0);
                self.data_buffer.push(0);
            }
            self.raise_nmi = true;
        } else if (command & 0xf0) == 0xd0 {
            // FORCE INTERRUPT command, type IV
            // 1101_IIII
            let interrupts = command & 0x0f;
            if self.trace {
                println!("FDC: Force interrupt {}", interrupts);
            }

            if interrupts == 0 {
                // The current command is terminated and busy is reset.
                self.read_index = 0;
                self.read_last = 0;
                self.data_buffer.clear();
                self.status &= !(FDCStatus::Busy as u8);
            } else {
                panic!("FDC: Interrupt forced with non zero I");
            }
        } else {
            if self.trace {
                println!("FDC: ${:02x} command not implemented", command);
            }
            panic!();
        }
    }

    pub fn get_status(&mut self) -> u8 {
        // Consume data if queued
        self.get_data();

        // Simulate index pulse (bit 1) when motor is on
        // At 300 RPM, one rotation = 200ms, index pulse duration ~2-4ms
        // We simulate it by toggling every N status reads (timing not critical)
        self.status_read_count = self.status_read_count.wrapping_add(1);
        let index_pulse = if self.motor_on {
            // Pulse is active for a short period (~5% of rotation cycle)
            (self.status_read_count % 100) < 5
        } else {
            false
        };

        let mut status = self.status;
        if index_pulse {
            status |= 0x02; // Set Index bit (bit 1)
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

        if self.read_index < self.read_last {
            // Store byte
            let index = self.read_index;
            let data = self.data;
            self.media_selected().write_byte(index, data);
            self.read_index += 1;
            self.raise_nmi = true;
            if self.read_index == self.read_last {
                // We are done writing
                self.media_selected().flush_disk();
                if self.trace {
                    println!("FDC: Set data completed ${:02x} {}-{}-{}", self.data, self.read_index, self.read_last, self.sector);
                }
                self.status = FDCStatus::NoError as u8;
                self.read_index = 0;
                self.read_last = 0;
            }
        }

        //if self.trace {
        //    println!("FDC: Set data ${:02x}", value);
        //}
    }

    pub fn get_data(&mut self) -> u8 {
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
                // We are done reading
                if self.trace {
                    println!("FDC: Get data completed ${:02x} {}-{}-{}", self.data, self.read_index, self.read_last, self.sector);
                }
                self.status = FDCStatus::NoError as u8;
                self.read_index = 0;
                self.read_last = 0;
                // Note: Real WD1793 does NOT auto-increment sector register for single-sector reads
            }
        }

        //if self.trace {
        //    println!("FDC: Get data ${:02x} {}-{}-{}", self.data, self.read_index, self.read_last, self.sector);
        //}
        self.data
    }
}
