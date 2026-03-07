use std::collections::HashMap;
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

/// Physical disk geometry, sufficient for TurboROM format detection and sector addressing.
/// Mirrors the information encoded in a TurboROM DPB format_byte / algorithm table entry.
#[derive(Clone, Copy, Debug)]
pub struct DiskGeometry {
    /// WD1793 N field: sector size = 128 << n  (0=128B, 1=256B, 2=512B, 3=1024B)
    pub n: u8,
    /// Sectors per track per side
    pub sectors_per_track: u8,
    /// First sector ID on side 0 (used in READ ADDRESS and sector_index)
    pub sector_id_base: u8,
    /// First sector ID on side 1 (double-sided disks only)
    pub side1_sector_id_base: u8,
    /// Physical tracks per side
    pub tracks: u8,
    pub double_sided: bool,
    /// Expected FDC density mode for this geometry:
    /// `true` = single density (FM), `false` = double density (MFM)
    pub single_density: bool,
    /// Human-readable name for display
    pub label: &'static str,
}

// ── TurboROM-supported format presets ────────────────────────────────────────

/// Kaypro SSSD / Osborne SSSD: 40T × 10S × 256B, IDs 1-10
pub const GEOM_KAYPRO_SSSD: DiskGeometry = DiskGeometry {
    n: 1, sectors_per_track: 10, sector_id_base: 1, side1_sector_id_base: 0,
    tracks: 40, double_sided: false, single_density: true, label: "Kaypro/Osborne SSSD",
};

/// Kaypro II SSDD: 40T × 10S × 512B, IDs 0-9
pub const GEOM_KAYPRO_SSDD: DiskGeometry = DiskGeometry {
    n: 2, sectors_per_track: 10, sector_id_base: 0, side1_sector_id_base: 0,
    tracks: 40, double_sided: false, single_density: false, label: "Kaypro SSDD",
};

/// Kaypro 4 DSDD (standard): 40T × 2S × 10S × 512B, side0 IDs 0-9, side1 IDs 10-19
pub const GEOM_KAYPRO_DSDD: DiskGeometry = DiskGeometry {
    n: 2, sectors_per_track: 10, sector_id_base: 0, side1_sector_id_base: 10,
    tracks: 40, double_sided: true, single_density: false, label: "Kaypro DSDD",
};

/// KayPLUS DSDD: same layout as Kaypro DSDD but both sides use IDs 0-9
pub const GEOM_KAYPLUS_DSDD: DiskGeometry = DiskGeometry {
    n: 2, sectors_per_track: 10, sector_id_base: 0, side1_sector_id_base: 0,
    tracks: 40, double_sided: true, single_density: false, label: "KayPLUS DSDD",
};

/// Advent/Osborne SSDD: 40T × 5S × 1024B, IDs 1-5
pub const GEOM_ADVENT_SSDD: DiskGeometry = DiskGeometry {
    n: 3, sectors_per_track: 5, sector_id_base: 1, side1_sector_id_base: 0,
    tracks: 40, double_sided: false, single_density: false, label: "Advent/Osborne SSDD",
};

/// Advent DSDD 48TPI: 40T × 2S × 5S × 1024B, side0 IDs 1-5, side1 IDs 11-15
#[allow(dead_code)]
pub const GEOM_ADVENT_DSDD48: DiskGeometry = DiskGeometry {
    n: 3, sectors_per_track: 5, sector_id_base: 1, side1_sector_id_base: 11,
    tracks: 40, double_sided: true, single_density: false, label: "Advent DSDD 48TPI",
};

/// Advent DSDD 96TPI: 80T × 2S × 5S × 1024B, side0 IDs 1-5, side1 IDs 21-25
pub const GEOM_ADVENT_DSDD96: DiskGeometry = DiskGeometry {
    n: 3, sectors_per_track: 5, sector_id_base: 1, side1_sector_id_base: 21,
    tracks: 80, double_sided: true, single_density: false, label: "Advent DSDD 96TPI",
};

/// Micro Cornucopia 96TPI: 80T × 2S × 10S × 512B, side0 IDs 0-9, side1 IDs 20-29
pub const GEOM_MICROCORNUCOPIA: DiskGeometry = DiskGeometry {
    n: 2, sectors_per_track: 10, sector_id_base: 0, side1_sector_id_base: 20,
    tracks: 80, double_sided: true, single_density: false, label: "Micro Cornucopia 96TPI",
};

/// Xerox 820-1 SSSD: 40T × 18S × 128B FM, IDs 1-18
pub const GEOM_XEROX_820: DiskGeometry = DiskGeometry {
    n: 0, sectors_per_track: 18, sector_id_base: 1, side1_sector_id_base: 0,
    tracks: 40, double_sided: false, single_density: true, label: "Xerox 820-1 SSSD",
};

/// Epson QX-10 DSDD 48TPI: 40T × 2S × 16S × 256B
/// Side 1 uses the same sector IDs as side 0 (1-16); side selection is
/// carried by the head/side signal, not by an offset ID range.
pub const GEOM_EPSON_QX10: DiskGeometry = DiskGeometry {
    n: 1, sectors_per_track: 16, sector_id_base: 1, side1_sector_id_base: 1,
    tracks: 40, double_sided: true, single_density: false, label: "Epson QX-10 DSDD",
};

// ─────────────────────────────────────────────────────────────────────────────

const SECTOR_SIZE_DD: usize = 512;
const SECTOR_SIZE_SD: usize = 256;

/// Count active CP/M directory entries (status 0x00-0x0F) at a byte offset.
/// Scans up to 64 entries (2048 bytes).  Stops on any non-0x00..=0x1F, non-0xE5 byte.
fn count_valid_cpm_entries(data: &[u8], dir_offset: usize) -> usize {
    let limit = (dir_offset + 32 * 64).min(data.len());
    let mut count = 0;
    let mut offset = dir_offset;
    while offset + 32 <= limit {
        match data[offset] {
            0x00..=0x0F => count += 1,
            0xE5        => {}           // empty slot — keep scanning
            _           => break,       // not a directory region
        }
        offset += 32;
    }
    count
}

/// Detect disk geometry from raw image bytes.
///
/// `side1_sector_base` — the machine-configured sector ID base for side 1
/// (10 for standard Kaypro, 0 for KayPLUS).  Used to resolve the 409,600 B
/// ambiguity between Kaypro DSDD (side1 IDs 10-19) and KayPLUS DSDD (0-9).
///
/// Returns `None` if the image size is not recognised.
pub fn auto_detect_geometry(data: &[u8], side1_sector_base: u8) -> Option<DiskGeometry> {
    match data.len() {
        // 40T × 10S × 256B = 102,400 B  →  Kaypro/Osborne SSSD
        // Allow small trailing slack (same rationale as SSDD range handling).
        102_400..=103_424 => Some(GEOM_KAYPRO_SSSD),

        // 92,160 B  →  Xerox 820-1 SSSD (40T × 18S × 128B, unique size)
        // Allow small trailing slack for images with extra tail padding.
        92_160..=93_184 => Some(GEOM_XEROX_820),

        // 204,800 B  →  Kaypro SSDD (10S×512B)  OR  Advent/Osborne SSDD (5S×1024B)
        // Disambiguate via CP/M directory probe at each format's reserved-track offset.
        //   Kaypro SSDD:  2 reserved tracks × 10S × 512B = 10,240 B
        //   Advent SSDD:  3 reserved tracks ×  5S ×1024B = 15,360 B
        204_800..=205_824 => {
            let kaypro_score = count_valid_cpm_entries(data, 2 * 10 * 512);
            let advent_score = count_valid_cpm_entries(data,  3 *  5 * 1024);
            if advent_score > kaypro_score {
                Some(GEOM_ADVENT_SSDD)
            } else {
                Some(GEOM_KAYPRO_SSDD)
            }
        }

        // 327,680 B  →  Epson QX-10 DSDD 48TPI (40T × 2S × 16S × 256B, unique size)
        // NOTE: geometry (sector IDs, reserved tracks) needs verification with real images.
        327_680 => Some(GEOM_EPSON_QX10),

        // 409,600 B  →  Kaypro/KayPLUS DSDD (10S×512B)  OR  Advent DSDD 48TPI (5S×1024B)
        // Cannot be distinguished by content — both have identical directory offsets.
        // Default to Kaypro/KayPLUS DSDD; use side1_sector_base to pick the right variant.
        // Advent DSDD 48 images created inside the emulator already have per-track geometry
        // from WRITE TRACK and will not be affected by this default.
        409_600..=411_648 => {
            if side1_sector_base == 0 {
                Some(GEOM_KAYPLUS_DSDD)
            } else {
                Some(GEOM_KAYPRO_DSDD)
            }
        }

        // 819,200 B  →  Advent DSDD 96TPI (80T × 2S × 5S × 1024B)
        //           OR  Micro Cornucopia (80T × 2S × 10S × 512B)
        // Probe: Advent DSDD 96 has 3 reserved tracks per side (side-interleaved image),
        // directory at offset 3×2×5×1024 = 30,720 B.
        // Micro Cornucopia has 2 reserved tracks, directory at 2×2×10×512 = 20,480 B.
        819_200 => {
            let advent_score = count_valid_cpm_entries(data, 3 * 2 * 5 * 1024);
            let micro_score  = count_valid_cpm_entries(data, 2 * 2 * 10 * 512);
            if micro_score > advent_score {
                Some(GEOM_MICROCORNUCOPIA)
            } else {
                Some(GEOM_ADVENT_DSDD96)
            }
        }

        _ => None,
    }
}

/// Legacy size-only format detection used for initial MediaFormat classification.
/// Returns the closest `MediaFormat` bucket; fine-grained geometry comes from
/// `auto_detect_geometry()`.
pub fn detect_media_format(len: usize) -> MediaFormat {
    match len {
        102_400..=103_424        => MediaFormat::SsSd,
        92_160..=93_184          => MediaFormat::SsSd,   // Xerox 820-1
        204_800..=205_824        => MediaFormat::SsDd,
        327_680                  => MediaFormat::DsDd,   // Epson QX-10
        409_600..=411_648        => MediaFormat::DsDd,
        819_200                  => MediaFormat::DsDd,   // Advent 96 / Micro Cornucopia
        _                        => MediaFormat::Unformatted,
    }
}

#[derive(Clone, Copy)]
pub struct TrackGeometry {
    pub n: u8,
    pub sector_count: u8,
    pub sector_base: u8,
}

pub struct Media {
    pub file: Option<File>,
    pub name: String,
    pub content: Vec<u8>,
    pub format: MediaFormat,
    /// Detected or configured disk geometry.  When Some, overrides format-based
    /// geometry calculations for tracks(), double_sided(), and read_address() N.
    /// Pre-populated at load time by apply_geometry(); updated by finish_write_track().
    pub geometry: Option<DiskGeometry>,
    /// Whether the disk is write-protected. Separate from file handle presence:
    /// file: None + write_protected: false = in-memory writable (test/fallback media)
    /// file: None + write_protected: true = read-only disk image
    /// file: Some + write_protected: false = persistent writable disk
    pub write_protected: bool,
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
    /// Per-track geometry learned from WRITE TRACK or pre-populated from DiskGeometry.
    /// Indexed by (track, side).  On a real WD1793, each track has its own IDAM
    /// headers defining N/sector layout.  Takes priority in sector_index() and
    /// read_address().
    pub track_geometry: HashMap<(u8, bool), TrackGeometry>,

    pub write_min: usize,
    pub write_max: usize,
}

impl Media {
    pub fn double_sided(&self) -> bool {
        if let Some(ref geom) = self.geometry {
            geom.double_sided
        } else {
            self.format == MediaFormat::DsDd
        }
    }

    pub fn tracks(&self) -> u8 {
        if let Some(ref geom) = self.geometry {
            geom.tracks
        } else {
            match self.format {
                MediaFormat::SsSd => 40,
                MediaFormat::SsDd => 40,
                MediaFormat::DsDd => 40,
                MediaFormat::Unformatted => 0,
            }
        }
    }

    /// Apply a known geometry to this media object.
    ///
    /// Sets `geometry`, `learned_n`, and `learned_sector_base` so that
    /// `sector_size()`, `sectors_per_side()`, `sector_id_base()`, `tracks()`, and
    /// `double_sided()` all reflect the correct geometry immediately.
    ///
    /// For double-sided foreign formats whose side-1 sector ID base cannot be
    /// expressed by `side1_sector_base` alone (e.g. Advent DSDD 96 base=21,
    /// Micro Cornucopia base=20), `track_geometry` is pre-populated for both
    /// sides so that `sector_index()` and `read_address()` use the right values.
    ///
    /// Standard Kaypro / KayPLUS DSDD side-1 tracks are intentionally NOT
    /// pre-populated: the fallback path in `sector_index()` naturally handles
    /// both KayPLUS-style (0–9) and Kaypro-style (10–19) side-1 IDs, which
    /// the KayPLUS ROM relies on during boot.
    pub fn apply_geometry(&mut self, geom: DiskGeometry) {
        self.geometry = Some(geom);
        self.learned_n = Some(geom.n);
        self.learned_sector_base = Some(geom.sector_id_base);

        // Pre-populate track_geometry only for double-sided formats where the
        // geometry's side-1 base differs from the machine-configured side1_sector_base.
        // Single-sided and standard Kaypro/KayPLUS DS formats work correctly via
        // the fallback path in sector_index() using learned_n and learned_sector_base.
        if geom.double_sided && geom.side1_sector_id_base != self.side1_sector_base {
            for track in 0..geom.tracks {
                self.track_geometry.insert((track, false), TrackGeometry {
                    n: geom.n,
                    sector_count: geom.sectors_per_track,
                    sector_base: geom.sector_id_base,
                });
                self.track_geometry.insert((track, true), TrackGeometry {
                    n: geom.n,
                    sector_count: geom.sectors_per_track,
                    sector_base: geom.side1_sector_id_base,
                });
            }
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

    pub fn track_stride_per_side(&self) -> usize {
        self.sectors_per_side() as usize * self.sector_size()
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

        let geometry = auto_detect_geometry(&content, self.side1_sector_base);
        if geometry.is_none() {
            return Err(Error::new(ErrorKind::Other,
                format!("Unrecognized disk image format (size {} bytes)", content.len())));
        }

        let format = detect_media_format(content.len());

        self.write_protected = readonly;
        self.file = file;
        self.name = filename.to_owned();
        self.content = content;
        self.format = format;
        self.geometry = None;
        self.learned_n = None;
        self.learned_sector_base = None;
        self.track_geometry.clear();

        // Apply detected geometry: populates learned_n, learned_sector_base,
        // and pre-fills track_geometry for all tracks so that READ ADDRESS
        // returns the correct N and sector base immediately on disk insertion.
        self.apply_geometry(geometry.unwrap());

        Ok(())
    }

    pub fn flush_disk(&mut self) {
        if self.write_max < self.write_min {
            // nothing to write
            return;
        }

        if let Some(ref mut file) = self.file {
            if let Err(e) = file.seek(SeekFrom::Start(self.write_min as u64)) {
                eprintln!("Warning: Failed to seek disk '{}': {}", self.name, e);
                return;
            }
            if let Err(e) = file.write_all(&self.content[self.write_min..=self.write_max]) {
                eprintln!("Warning: Failed to write disk '{}': {}", self.name, e);
                return;
            }
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
        let stride = self.track_stride_per_side();
        if stride == 0 { return; }
        let new_len = tracks * 2 * stride;
        let mut new_content = vec![0xE5u8; new_len];
        for t in 0..tracks {
            let src_offset = t * stride;
            let dst_offset = t * 2 * stride;
            let copy_len = stride.min(self.content.len().saturating_sub(src_offset));
            if copy_len > 0 {
                new_content[dst_offset..dst_offset + copy_len]
                    .copy_from_slice(&self.content[src_offset..src_offset + copy_len]);
            }
        }
        self.content = new_content;
        self.format = MediaFormat::DsDd;
        if let Some(ref mut geom) = self.geometry {
            geom.double_sided = true;
        }
        self.write_min = 0;
        self.write_max = new_len - 1;
    }

    /// Returns `(valid, base_sector_id, n_code)`.
    ///
    /// `n_code` is the WD1793 N field (sector size code: 0=128B, 1=256B, 2=512B, 3=1024B).
    /// TurboROM reads this from READ ADDRESS to select the matching DPB entry.
    /// Previously this was hardcoded to 2; now it reflects the actual disk geometry.
    pub fn read_address(&self, side_2: bool, track: u8, _sector: u8) -> (bool, u8, u8) {
        if track >= self.tracks() || (side_2 && !self.double_sided()) {
            return (false, 0, 2);
        }

        // Per-track geometry (from WRITE TRACK or pre-populated by apply_geometry)
        // is the authoritative source: it gives both the correct sector base and N.
        if let Some(geom) = self.track_geometry.get(&(track, side_2)) {
            return (true, geom.sector_base, geom.n);
        }

        // Fallback: track_geometry not populated (standard Kaypro/KayPLUS formats).
        // Use sector_id_base() for side 0 — this respects learned_sector_base set by
        // apply_geometry(), so Advent SSDD (base=1) returns the right value here.
        // Side 1 uses the machine-configured side1_sector_base.
        let base = if side_2 {
            self.side1_sector_base
        } else {
            self.sector_id_base()
        };
        let n = self.learned_n.unwrap_or(2);
        (true, base, n)
    }

    pub fn density_matches(&self, controller_single_density: bool) -> bool {
        if let Some(ref geom) = self.geometry {
            geom.single_density == controller_single_density
        } else {
            true
        }
    }

    pub fn sector_index(&self, side_2: bool, track: u8, sector: u8) -> (bool, usize, usize) {
        if side_2 && !self.double_sided() {
            return (false, 0, 0);
        }
        if track >= self.tracks() {
            return (false, 0, 0);
        }

        // Look up per-track geometry (from WRITE TRACK), fall back to globals
        if let Some(geom) = self.track_geometry.get(&(track, side_2)) {
            let n = geom.n;
            let spt = geom.sector_count;
            let base = geom.sector_base;
            let sector_size = 128usize << (n as usize);

            let adjusted = if sector >= base { sector - base } else { return (false, 0, 0); };
            if adjusted >= spt { return (false, 0, 0); }

            let stride = self.track_stride_per_side();
            if stride == 0 { return (false, 0, 0); }
            let sides = if self.double_sided() { 2usize } else { 1usize };
            let side_idx = if side_2 { 1usize } else { 0usize };
            let track_offset = stride * (track as usize * sides + side_idx);

            let index = track_offset + adjusted as usize * sector_size;
            let last = index + sector_size;
            if last > self.content.len() {
                return (false, 0, 0);
            }
            return (true, index, last);
        }

        // Fallback: use the original flat layout mapping.
        // The image layout interleaves sides: for each track, side 0 sectors
        // come first, then side 1 sectors. Sector IDs map directly to slots.
        let base = self.sector_id_base();
        let adjusted = if sector >= base { sector - base } else { return (false, 0, 0); };

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
        self.write_protected
    }

    pub fn info(&self) -> String {
        let persistence = match self.file {
            Some(_) => "persistent",
            _       => "transient",
        };
        let fmt = if let Some(ref geom) = self.geometry {
            geom.label
        } else {
            match self.format {
                MediaFormat::Unformatted => "unformatted",
                MediaFormat::SsSd       => "SSSD",
                MediaFormat::SsDd       => "SSDD",
                MediaFormat::DsDd       => "DSDD",
            }
        };
        format!("{} ({}, {})", self.name, persistence, fmt)
    }
}
