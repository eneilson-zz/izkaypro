use super::media::*;

// ============================================================================
// KAYPRO DISK IMAGE SELECTION
// ============================================================================
// Uncomment ONE of the following sections to select the boot disk.
// IMPORTANT: Also update the ROM selection in kaypro_machine.rs to match!

// --- Kaypro 4/84 with ROM 81-292a (DSDD) ---
// Uses: roms/81-292a.rom in kaypro_machine.rs
//static DISK_CPM22: &[u8] = include_bytes!("../disks/k484-cpm22f-boot.img");
//static DISK_BLANK: &[u8] = include_bytes!("../disks/cpm22-kaypro4-blank.img");
//const DEFAULT_FORMAT: MediaFormat = MediaFormat::DsDd;
//const BOOT_DISK_NAME: &str = "CP/M 2.2F Kaypro 4/83 (DSDD)";
//const BLANK_DISK_NAME: &str = "Blank DSDD disk";

// --- Kaypro 4/83 with ROM 81-232 (DSDD) ---
// Uses: roms/81-232.rom in kaypro_machine.rs
static DISK_CPM22: &[u8] = include_bytes!("../disks/k484-cpm22f-boot.img");
static DISK_DIAGS: &[u8] = include_bytes!("../disks/KayproDiagnostics.img");
const DEFAULT_FORMAT: MediaFormat = MediaFormat::DsDd;
const BOOT_DISK_NAME: &str = "CP/M 2.2F Kaypro 4/83 (DSDD)";
const DIAGS_DISK_NAME: &str = "Emulator Diagnostics (DSDD)";

// --- Kaypro 4-84 with Turbo ROM 3.4 (DSDD) ---
// Uses: roms/trom34.rom in kaypro_machine.rs
// static DISK_CPM22: &[u8] = include_bytes!("../disks/k484_turborom_63k_boot.img");
// static DISK_BLANK: &[u8] = include_bytes!("../disks/cpm22-kaypro4-blank.img");
// const DEFAULT_FORMAT: MediaFormat = MediaFormat::DsDd;
// const BOOT_DISK_NAME: &str = "CP/M 2.2 Turbo ROM 3.4 (DSDD)";
// const BLANK_DISK_NAME: &str = "Blank DSDD disk";

// --- Kaypro II with ROM 81-149 (SSDD) ---
// Uses: roms/81-149c.rom in kaypro_machine.rs
// static DISK_CPM22: &[u8] = include_bytes!("../disks/cpm22-rom149.img");
// static DISK_BLANK: &[u8] = include_bytes!("../disks/blank.img");
// const DEFAULT_FORMAT: MediaFormat = MediaFormat::SsDd;
// const BOOT_DISK_NAME: &str = "CP/M 2.2 Kaypro II (SSDD)";
// const BLANK_DISK_NAME: &str = "Blank SSDD disk";

// ============================================================================

pub enum Drive {
    A = 0,
    B = 1,
}

pub struct FloppyController {
    pub motor_on: bool,
    pub drive: u8,
    side_2: bool,
    track: u8,
    sector: u8,
    pub single_density: bool,
    data: u8,
    status: u8,

    media: [Media ;2],

    read_index: usize,
    read_last: usize,

    data_buffer: Vec<u8>,

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
    pub fn new(trace: bool, trace_rw: bool) -> FloppyController {
        FloppyController {
            motor_on: false,
            drive: 0,
            side_2: false,
            track: 0,
            sector: 0,
            single_density: false,
            data: 0,
            status: 0,
            media: [
                Media {
                    file: None,
                    name: BOOT_DISK_NAME.to_owned(),
                    content: DISK_CPM22.to_vec(),
                    format: DEFAULT_FORMAT,
                    write_min: usize::MAX,
                    write_max: 0,
                },
                Media {
                    file: None,
                    name: DIAGS_DISK_NAME.to_owned(),
                    content: DISK_DIAGS.to_vec(),
                    format: DEFAULT_FORMAT,
                    write_min: usize::MAX,
                    write_max: 0,
                },
            ],

            read_index: 0,
            read_last: 0,

            data_buffer: Vec::new(),

            raise_nmi: false,
            trace,
            trace_rw,
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
                self.status = FDCStatus::NoError as u8;
            } else {
                self.status = FDCStatus::SeekErrorOrRecordNotFound as u8;
            }
            self.raise_nmi = true;
        } else if (command & 0xe0) == 0x80 {
            // READ SECTOR command, type II
            // 100mFEFx
            if command & 0x10 != 0 {
                panic!("Multiple sector reads not supported")
            }
            if self.trace || self.trace_rw {
                println!("FDC: Read sector (Si:{}, Tr:{}, Se:{})", self.side_2, self.track, self.sector);
            }

            let side_2 = self.side_2;
            let track = self.track;
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
                println!("FDC: Write sector (Si:{}, Tr:{}, Se:{})", self.side_2, self.track, self.sector);
            }

            let side_2 = self.side_2;
            let track = self.track;
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
            let track = self.track;
            let sector = self.sector;

            let (valid, sector_id) = self.media_selected().read_address(side_2, track, sector);
            if valid {
                if self.trace {
                    println!("FDC: Read address ({},{},{}) -> sector_id={}", side_2, track, sector, sector_id);
                }
                // Note: Real WD1793 does NOT modify sector register during READ ADDRESS
                self.status = FDCStatus::NoError as u8;
                self.data_buffer.clear();
                self.data_buffer.push(self.track);
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

        self.status
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
