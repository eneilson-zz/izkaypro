use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom, Result, Error, ErrorKind};

/*
Notes on the DSDD disks as seen by different components:

Physical disk:
    There are two sides with 40 tracks each.
    Each track has 10 sectors, each with 512 bytes.
    The sectors on side 1 are numbered from 0 to 9,
    and on side 2 from 10 to 19.

Floppy controller:
    The controller doesn't know the disk side.
    The head can move from tack 0 to 39.
    When looking for a sector, the sector id of
    the media has to match.

BIOS and ROM entrypoints:
    There are no sides.
    Tracks are numbered from 0 to 79. Even tracks
    are on side 1, odd tracks are on side 2.
    Logical sectors are numbered from 0 to 39, each with 128 bytes

File images:
    They have the same order as per the BIOS entrypoints
    The file has 2*40*10*4 logical ectors, each with 128 bytes.
    First the 40 sectors of the first track of side 1,
    then the 40 sectors of the first track of side 2,
    then the 40 sectors of the second track of side 1,
    then the 40 sectors of the second track of side 2,
    and so on.
*/

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum MediaFormat {
    Unformatted,
    SsSd,     // Single-sided, single-density
    SsDd,     // Single-sided, double-density
    DsDd,     // Double-sided, double-density
}

const SECTOR_SIZE_DD: usize = 512;
const SECTOR_SIZE_SD: usize = 256;

pub fn detect_media_format(len: usize) -> MediaFormat {
    if len == 102400 {
        MediaFormat::SsSd
    } else if (204800..=205824).contains(&len) {
        // Some valid disk images are a bit bigger, I don't know why
        MediaFormat::SsDd
    } else if (409600..=411648).contains(&len) {
        MediaFormat::DsDd
    } else {
        MediaFormat::Unformatted
    }
}

pub struct Media {
    pub file: Option<File>,
    pub name: String,
    pub content: Vec<u8>,
    pub format: MediaFormat,
    /// Sector ID base for side 1 headers, reflecting how the disk was physically formatted.
    /// Standard Kaypro disks: 10 (sector IDs 10-19 on side 1)
    /// KayPLUS-formatted disks: 0 (sector IDs 0-9 on both sides)
    pub side1_sector_base: u8,
    /// Sector size code learned from WRITE TRACK (IDAM N field).
    /// When Some(n), overrides the default sector_size/sectors_per_side.
    /// Allows formats like 5×1024 SSDD to coexist with 10×512 SSDD.
    pub learned_n: Option<u8>,
    /// Sector ID base learned from WRITE TRACK (minimum sector ID seen).
    /// When Some(b), overrides the default sector_id_base().
    pub learned_sector_base: Option<u8>,

    pub write_min: usize,
    pub write_max: usize,
}

impl Media {
    pub fn double_sided(&self) -> bool {
        self.format == MediaFormat::DsDd
    }

    pub fn tracks(&self) -> u8 {
        match self.format {
            MediaFormat::SsSd => 40,
            MediaFormat::SsDd => 40,
            MediaFormat::DsDd => 40,
            MediaFormat::Unformatted => 0,
        }
    }

    pub fn sectors_per_side(&self) -> u8 {
        if let Some(n) = self.learned_n {
            let sector_size = 128usize << (n as usize);
            let bytes_per_side = self.content.len() / self.tracks() as usize
                / if self.double_sided() { 2 } else { 1 };
            (bytes_per_side / sector_size) as u8
        } else {
            match self.format {
                MediaFormat::SsSd => 10,
                MediaFormat::SsDd => 10,
                MediaFormat::DsDd => 10,
                MediaFormat::Unformatted => 0,
            }
        }
    }

    pub fn sectors(&self) -> u8 {
        if self.double_sided() {
            self.sectors_per_side() * 2
        } else {
            self.sectors_per_side()
        }
    }

    pub fn sector_size(&self) -> usize {
        if let Some(n) = self.learned_n {
            128usize << (n as usize)
        } else {
            match self.format {
                MediaFormat::SsSd => SECTOR_SIZE_SD,
                _ => SECTOR_SIZE_DD,
            }
        }
    }

    pub fn sector_id_base(&self) -> u8 {
        if let Some(base) = self.learned_sector_base {
            base
        } else {
            match self.format {
                MediaFormat::SsSd => 1,
                _ => 0,
            }
        }
    }


    pub fn load_disk(&mut self, filename: &str) -> Result<()>{
        self.flush_disk();

        // Try opening writable, then read only
        let (mut file, readonly) = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(filename)
            {
                Ok(file) => (file, false),
                _ => {
                    // Try opening read-only
                    match OpenOptions::new()
                        .read(true)
                        .open(filename)
                        {
                            Ok(file) => (file, true),
                            Err(err) => {
                                return Err(err);
                            }
                        }
                }
            };

        // Load content
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        // Store the file descriptor for writable files
        let file = if readonly {
            None
        } else {
            Some(file)
        };

        let format = detect_media_format(content.len());
        if format == MediaFormat::Unformatted {
            return Err(Error::new(ErrorKind::Other, format!("Unrecognized disk image format (len {})", content.len())));
        }

        self.file = file;
        self.name = filename.to_owned();
        self.content = content;
        self.format = format;
        self.learned_n = None;
        self.learned_sector_base = None;

        Ok(())
    }

    pub fn flush_disk(&mut self) {
        if self.write_max < self.write_min {
            // nothing to write
            return;
        }

        if let Some(ref mut file) = self.file {
            file.seek(SeekFrom::Start(self.write_min as u64)).unwrap();
            file.write_all(&self.content[self.write_min..=self.write_max]).unwrap();
        }

        self.write_max = 0;
        self.write_min = usize::MAX;
    }

    pub fn is_valid_track(&self, track: u8) -> bool {
        track < self.tracks()
    }

    pub fn upgrade_to_double_sided(&mut self) {
        if self.double_sided() {
            return;
        }
        let tracks = self.tracks() as usize;
        if tracks == 0 {
            return;
        }
        let sector_size = self.sector_size();
        let spt = self.sectors_per_side() as usize;
        let bytes_per_side_track = spt * sector_size;
        let new_len = tracks * 2 * bytes_per_side_track;
        let mut new_content = vec![0xE5u8; new_len];
        for t in 0..tracks {
            let src_offset = t * bytes_per_side_track;
            let dst_offset = t * 2 * bytes_per_side_track;
            let copy_len = bytes_per_side_track.min(self.content.len().saturating_sub(src_offset));
            if copy_len > 0 {
                new_content[dst_offset..dst_offset + copy_len]
                    .copy_from_slice(&self.content[src_offset..src_offset + copy_len]);
            }
        }
        self.content = new_content;
        self.format = MediaFormat::DsDd;
        self.write_min = 0;
        self.write_max = new_len - 1;
    }

    pub fn read_address(&self, side_2: bool, track: u8, _sector: u8) -> (bool, u8) {
        if track >= self.tracks() || (side_2 && !self.double_sided()) {
            return (false, 0);
        }

        // READ ADDRESS returns the base sector ID for the current side.
        // SSSD: sectors 1-10 on side 0 (1-based)
        // DSDD standard Kaypro: side 0 = 0-9, side 1 = 10-19
        // DSDD KayPLUS: side 0 = 0-9, side 1 = 0-9
        let base = if self.format == MediaFormat::SsSd {
            self.sector_id_base()
        } else if side_2 {
            self.side1_sector_base
        } else {
            0
        };
        (true, base)
    }

    pub fn sector_index(&self, side_2: bool, track: u8, sector: u8) -> (bool, usize, usize) {
        if side_2 && !self.double_sided() {
            return (false, 0, 0);
        }
        if track >= self.tracks() {
            return (false, 0, 0);
        }

        // Map the FDC sector ID to a 0-based image slot index.
        // SSSD uses 1-based sector IDs (1-10), subtract the base to get 0-based.
        // DSDD uses 0-based on side 0 (0-9) and either 10-19 or 0-9 on side 1.
        let base = self.sector_id_base();
        let adjusted = if sector >= base { sector - base } else { return (false, 0, 0); };

        // For double-sided: remap side 1 sectors into the second half of the track
        let mapped_sector = if side_2 && adjusted < self.sectors_per_side() {
            adjusted + self.sectors_per_side()
        } else {
            adjusted
        };

        if !side_2 && mapped_sector >= self.sectors_per_side() {
            return (false, 0, 0);
        }
        if side_2 && mapped_sector >= self.sectors() {
            return (false, 0, 0);
        }

        let sector_size = self.sector_size();
        let index = (track as usize * self.sectors() as usize + mapped_sector as usize) * sector_size;
        let last = index + sector_size;
        if last > self.content.len() {
            return (false, 0, 0);
        }
        (true, index, last)
    }

    pub fn read_byte(&self, index: usize) -> u8 {
        self.content[index]
    }

    pub fn write_byte(&mut self, index: usize, value: u8) {
        self.content[index] = value;
        if index < self.write_min {
            self.write_min = index;
        }
        if index > self.write_max {
            self.write_max = index;
        }
    }

    pub fn is_write_protected(&self) -> bool {
        self.file.is_none()
    }

    pub fn info(&self) -> String {
        self.name.clone() + " (" +
            match self.file {
                Some(_) => "persistent",
                _ => "transient"
            } + " " +
            match self.format {
                MediaFormat::Unformatted => " (unformatted)",
                MediaFormat::SsSd => " (SSSD)",
                MediaFormat::SsDd => " (SSDD)",
                MediaFormat::DsDd => " (DSDD)",
            } + ")"
    }
}