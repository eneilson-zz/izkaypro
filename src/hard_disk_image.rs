use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const CYLINDERS: u64 = 306;
pub const HEADS: u64 = 4;
pub const SECTORS_PER_TRACK: u64 = 17;
pub const SECTOR_SIZE: u64 = 512;
pub const HEADER_SIZE: u64 = 128;
pub const DATA_SIZE: u64 = CYLINDERS * HEADS * SECTORS_PER_TRACK * SECTOR_SIZE;
pub const IMAGE_SIZE: u64 = DATA_SIZE + HEADER_SIZE;

const HEADER_TEXT: &str = "306c4h512z17p1l\n";
const FORMAT_FLAG_PREFIX: &str = "fmt=";

/// Ensure the HD image exists. If missing, create a blank image with a
/// WD1002-compatible trailing geometry header.
pub fn ensure_exists<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)?;

    file.set_len(IMAGE_SIZE)?;
    initialize_unformatted_markers(&mut file)?;
    write_header(&mut file, false)?;
    file.flush()?;
    Ok(())
}

fn initialize_unformatted_markers(file: &mut std::fs::File) -> std::io::Result<()> {
    // Important for Kaypro10 ROM behavior:
    // A fully-zero defect map sector has a valid 16-bit checksum (0x0000),
    // which can make a brand-new blank image look "formatted/present" to BIOS.
    // Real unformatted media should fail this validation so ROM falls back
    // to floppy boot until HDFMT writes a valid map.
    //
    // Poison both probe locations used by ROM:
    // - cyl 0, head 0, sector 16
    // - cyl 0, head 1, sector 16
    let mut invalid_map = [0u8; SECTOR_SIZE as usize];
    invalid_map[0] = 0xFF; // non-zero payload, leave checksum bytes zero => invalid

    let sec16_head0 = 16u64 * SECTOR_SIZE;
    let sec16_head1 = (SECTORS_PER_TRACK + 16u64) * SECTOR_SIZE;

    file.seek(SeekFrom::Start(sec16_head0))?;
    file.write_all(&invalid_map)?;
    file.seek(SeekFrom::Start(sec16_head1))?;
    file.write_all(&invalid_map)?;
    Ok(())
}

fn checksum16(bytes: &[u8]) -> u16 {
    let mut acc: u16 = 0;
    for &b in bytes {
        acc = acc.wrapping_add(b as u16);
    }
    acc
}

fn has_valid_k10_boot_sector(buf: &[u8]) -> bool {
    if buf.len() < 128 {
        return false;
    }
    // A blank sector (all 0x00/0xFF) trivially "passes" checksum but must
    // never be considered a valid Kaypro10 parameter/boot sector.
    let all_zero = buf.iter().all(|&b| b == 0x00);
    let all_ff = buf.iter().all(|&b| b == 0xFF);
    if all_zero || all_ff {
        return false;
    }
    let sum = checksum16(&buf[..126]);
    let stored = u16::from_le_bytes([buf[126], buf[127]]);
    if sum != stored {
        return false;
    }
    // Basic sanity on sector-count field used by ROM boot logic.
    // Bytes 6..7 are little-endian count of additional 128-byte sectors.
    let count = u16::from_le_bytes([buf[6], buf[7]]);
    count > 0 && count < 512
}

const K10_HD_SECTOR_MAP: [u8; 16] = [1, 6, 11, 16, 4, 9, 14, 2, 7, 12, 17, 5, 10, 15, 3, 8];
const K10_LOGICALS_PER_TRACK: usize = 64;
const K10_SLOT1_TRACK0_BASE: u64 = SECTORS_PER_TRACK * SECTOR_SIZE; // cyl0/head1
const K10_PARAM_SECTOR_INDEX: u64 = 16; // BIOS validates sector 16 (0-based sec=0x10)
const K10_BOOT_SECTOR_INDEX: u64 = 0; // BIOS parameter/boot checksum sector
const K10_PROTECTED_BOOT_OFFSETS: [u64; 2] = [
    K10_BOOT_SECTOR_INDEX * SECTOR_SIZE,
    K10_SLOT1_TRACK0_BASE + (K10_BOOT_SECTOR_INDEX * SECTOR_SIZE),
];
const K10_PROTECTED_PARAM_OFFSETS: [u64; 2] = [
    K10_PARAM_SECTOR_INDEX * SECTOR_SIZE,
    K10_SLOT1_TRACK0_BASE + (K10_PARAM_SECTOR_INDEX * SECTOR_SIZE),
];
const K10_PROTECTED_ALL_OFFSETS: [u64; 4] = [
    K10_PROTECTED_BOOT_OFFSETS[0],
    K10_PROTECTED_BOOT_OFFSETS[1],
    K10_PROTECTED_PARAM_OFFSETS[0],
    K10_PROTECTED_PARAM_OFFSETS[1],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerWriteOutcome {
    Applied,
    AppliedProtectedSector,
    PreservedProtectedSector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerWriteSource {
    WriteData,
    FormatTrack,
}

fn k10_logical_offset(track_base: u64, logical_sector: usize) -> Option<u64> {
    if logical_sector >= K10_LOGICALS_PER_TRACK {
        return None;
    }
    let phys_index = logical_sector / 4;
    let quarter = logical_sector % 4;
    let sec_id = K10_HD_SECTOR_MAP.get(phys_index).copied()? as u64; // 1-based
    Some(track_base + (sec_id - 1) * SECTOR_SIZE + (quarter as u64) * 128)
}

fn read_k10_logical_boot(hd: &mut HardDiskImage, track_base: u64, out: &mut [u8; 128]) -> std::io::Result<()> {
    let off = k10_logical_offset(track_base, 0).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid k10 logical sector mapping")
    })?;
    hd.read_at(off, out)
}

fn build_putsysu_boot_payload(floppy: &[u8]) -> std::io::Result<Vec<u8>> {
    // Build the 14 x 512-byte bootstrap window using physical floppy sectors:
    // - sectors 0..9  (track0/side0)
    // - sectors 14..17 (track0/side1 sector IDs 14..17 with base-10 numbering)
    // This matches the Kaypro10 ROM fallback fetch pattern observed in traces.
    const FLOPPY_SECTOR_SIZE: usize = 512;
    const BOOT_SECTORS: usize = 14;
    let mut payload = vec![0u8; BOOT_SECTORS * FLOPPY_SECTOR_SIZE];
    let mut out_ix = 0usize;
    for sec in 0..10usize {
        let src_off = sec * FLOPPY_SECTOR_SIZE;
        let dst_off = out_ix * FLOPPY_SECTOR_SIZE;
        if src_off + FLOPPY_SECTOR_SIZE > floppy.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "floppy image too small for boot sector copy",
            ));
        }
        payload[dst_off..dst_off + FLOPPY_SECTOR_SIZE]
            .copy_from_slice(&floppy[src_off..src_off + FLOPPY_SECTOR_SIZE]);
        out_ix += 1;
    }
    for sec in [14usize, 15usize, 18usize, 19usize] {
        let src_off = sec * FLOPPY_SECTOR_SIZE;
        let dst_off = out_ix * FLOPPY_SECTOR_SIZE;
        if src_off + FLOPPY_SECTOR_SIZE > floppy.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "floppy image too small for side1 boot sector copy",
            ));
        }
        payload[dst_off..dst_off + FLOPPY_SECTOR_SIZE]
            .copy_from_slice(&floppy[src_off..src_off + FLOPPY_SECTOR_SIZE]);
        out_ix += 1;
    }
    Ok(payload)
}

fn laydown_putsysu_boot_window(hd: &mut HardDiskImage, payload: &[u8]) -> std::io::Result<()> {
    if payload.len() < 14 * SECTOR_SIZE as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "PUTSYSU payload too small",
        ));
    }

    // Seed a full 14-sector system window on both heads. PUTSYSU writes
    // additional boot/system tracks beyond the ROM's first fetch window.
    for sec in 0..14usize {
        let off = (sec as u64) * SECTOR_SIZE;
        let src_off = sec * SECTOR_SIZE as usize;
        hd.write_at(off, &payload[src_off..src_off + SECTOR_SIZE as usize])?;
    }

    for sec in 0..14usize {
        let off = K10_SLOT1_TRACK0_BASE + (sec as u64) * SECTOR_SIZE;
        let src_off = sec * SECTOR_SIZE as usize;
        hd.write_at(off, &payload[src_off..src_off + SECTOR_SIZE as usize])?;
    }

    // Preserve the empirically validated ROM fetch window mapping:
    // head 0 sec0..9 then head 1 sec4..7 must return payload sectors 10..13.
    for i in 0..4usize {
        let off = K10_SLOT1_TRACK0_BASE + ((4 + i) as u64) * SECTOR_SIZE;
        let src_off = (10 + i) * SECTOR_SIZE as usize;
        hd.write_at(off, &payload[src_off..src_off + SECTOR_SIZE as usize])?;
    }

    Ok(())
}

fn patch_k10_boot_header(hd: &mut HardDiskImage, track_base: u64) -> std::io::Result<()> {
    let off = k10_logical_offset(track_base, 0).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid k10 boot header mapping")
    })?;
    let mut boot = [0u8; 128];
    hd.read_at(off, &mut boot)?;
    // Convert the floppy bootstrap header to the Kaypro10 HD bootstrap
    // variant used by ROM disk-loader paths.
    boot[0] = 0x18;
    boot[1] = 0xFE;
    boot[2] = 0x00;
    boot[3] = 0xDE;
    boot[4] = 0x00;
    boot[5] = 0xF4;
    boot[6] = 0x34;
    boot[7] = 0x00;
    let sum = checksum16(&boot[..126]);
    boot[126..128].copy_from_slice(&sum.to_le_bytes());
    hd.write_at(off, &boot)?;
    Ok(())
}

/// Seed a Kaypro 10 hard disk image with a bootable CP/M system taken from a
/// known-good Kaypro 10 floppy image.
///
/// This writes:
/// - system bootstrap blocks (from floppy logical sectors) at HD offset 0
/// - mirrored boot parameter sector at alternate slot offset (17 * 512)
/// - a valid empty defect-map sector at HD sector 16 (16 * 512)
pub fn seed_kaypro10_from_floppy<P: AsRef<Path>, Q: AsRef<Path>>(
    hd_path: P,
    floppy_path: Q,
) -> std::io::Result<()> {
    ensure_exists(&hd_path)?;

    let mut hd = HardDiskImage::open(hd_path)?;
    let mut slot0_boot = [0u8; 128];
    let mut slot1_boot = [0u8; 128];
    read_k10_logical_boot(&mut hd, 0, &mut slot0_boot)?;
    read_k10_logical_boot(&mut hd, K10_SLOT1_TRACK0_BASE, &mut slot1_boot)?;
    let slot0_boot_ok = has_valid_k10_boot_sector(&slot0_boot);
    let slot1_boot_ok = has_valid_k10_boot_sector(&slot1_boot);

    let floppy = fs::read(floppy_path)?;
    if floppy.len() < 128 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "floppy image too small",
        ));
    }

    if !slot0_boot_ok || !slot1_boot_ok {
        let payload = build_putsysu_boot_payload(&floppy)?;
        laydown_putsysu_boot_window(&mut hd, &payload)?;
        // Keep boot-header checksums valid at both probe offsets.
        patch_k10_boot_header(&mut hd, 0)?;
        patch_k10_boot_header(&mut hd, K10_SLOT1_TRACK0_BASE)?;
    }

    // Write a valid defect-map sector at sector 17 for both slot probe
    // locations (head 0 and head 1 of cylinder 0).
    // Keep checksum-valid content and copy the 128-byte boot header at the
    // front so subsequent BIOS buffer reuse paths still see a coherent
    // parameter header signature.
    let mut defect = [0u8; 512];
    let mut boot_header = [0u8; 128];
    hd.read_at(0, &mut boot_header)?;
    defect[..128].copy_from_slice(&boot_header);
    let sum = checksum16(&defect[..510]);
    defect[510..512].copy_from_slice(&sum.to_le_bytes());
    for track_base in [0, K10_SLOT1_TRACK0_BASE] {
        hd.write_at(track_base + (16u64 * 512u64), &defect)?;
    }

    // Mark the first two heads/tracks formatted so controller checks pass.
    hd.set_track_formatted(0, 0, true)?;
    hd.set_track_formatted(0, 1, true)?;
    hd.set_track_formatted(1, 0, true)?;
    hd.set_track_formatted(1, 1, true)?;

    Ok(())
}

pub fn is_kaypro10_bootable<P: AsRef<Path>>(hd_path: P) -> std::io::Result<bool> {
    ensure_exists(&hd_path)?;
    let mut hd = HardDiskImage::open(hd_path)?;

    let mut slot0_boot = [0u8; 128];
    let mut slot1_boot = [0u8; 128];
    read_k10_logical_boot(&mut hd, 0, &mut slot0_boot)?;
    read_k10_logical_boot(&mut hd, K10_SLOT1_TRACK0_BASE, &mut slot1_boot)?;

    let mut sec16_0 = [0u8; 512];
    let mut sec16_1 = [0u8; 512];
    hd.read_at(16u64 * 512u64, &mut sec16_0)?;
    hd.read_at(K10_SLOT1_TRACK0_BASE + 16u64 * 512u64, &mut sec16_1)?;

    let slot0_ok = has_valid_k10_boot_sector(&slot0_boot) && has_valid_k10_defect_sector(&sec16_0);
    let slot1_ok = has_valid_k10_boot_sector(&slot1_boot) && has_valid_k10_defect_sector(&sec16_1);
    Ok(slot0_ok || slot1_ok)
}

fn has_valid_k10_defect_sector(buf: &[u8]) -> bool {
    if buf.len() < 512 {
        return false;
    }
    let sum = checksum16(&buf[..510]);
    let stored = u16::from_le_bytes([buf[510], buf[511]]);
    sum == stored
}

pub struct HardDiskImage {
    image_path: PathBuf,
    file: std::fs::File,
    formatted_tracks: Vec<bool>,
}

impl HardDiskImage {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let image_path = path.as_ref().to_path_buf();
        ensure_exists(&image_path)?;
        let mut file = OpenOptions::new().read(true).write(true).open(&image_path)?;
        let header = read_header(&mut file)?;
        let formatted_any = if let Some(flag) = parse_format_flag(&header) {
            flag
        } else {
            detect_formatted_data(&mut file)?
        };
        write_header(&mut file, formatted_any)?;

        let mut me = Self {
            image_path,
            file,
            formatted_tracks: vec![formatted_any; (CYLINDERS * HEADS) as usize],
        };
        me.load_track_map()?;
        Ok(me)
    }

    pub fn is_formatted(&self) -> bool {
        self.formatted_tracks.iter().any(|&t| t)
    }

    pub fn set_track_formatted(&mut self, cyl: u16, head: u8, formatted: bool) -> std::io::Result<()> {
        if let Some(i) = track_index(cyl as u64, head as u64) {
            self.formatted_tracks[i] = formatted;
            self.persist_track_map()?;
        }
        Ok(())
    }

    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        if offset.checked_add(buf.len() as u64).is_none() || offset + buf.len() as u64 > DATA_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("read range out of bounds: off={} len={}", offset, buf.len()),
            ));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(buf)?;
        Ok(())
    }

    pub fn write_at(&mut self, offset: u64, buf: &[u8]) -> std::io::Result<()> {
        if offset.checked_add(buf.len() as u64).is_none() || offset + buf.len() as u64 > DATA_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("write range out of bounds: off={} len={}", offset, buf.len()),
            ));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(buf)?;
        Ok(())
    }

    /// Write path used by WD controller emulation.
    ///
    /// Kaypro10 firmware treats C0/H0/S16 (and mirrored C0/H1/S16) as parameter
    /// sectors used for Winchester presence validation. HDFMT format/fill passes
    /// may blast those sectors with fill-pattern data; preserve existing content
    /// only for obvious destructive fill writes so legitimate parameter/defect
    /// sector layouts are not rejected.
    pub fn write_controller_sector(
        &mut self,
        offset: u64,
        buf: &[u8],
        source: ControllerWriteSource,
    ) -> std::io::Result<ControllerWriteOutcome> {
        // Protected-sector policy (Kaypro10):
        // - FORMAT TRACK must never overwrite protected sectors.
        // - Programmed WRITE data path may update protected sectors; firmware
        //   uses 128/256/512-byte writes while constructing valid sectors.
        for protected_off in K10_PROTECTED_ALL_OFFSETS {
            if ranges_overlap(offset, buf.len(), protected_off, SECTOR_SIZE as usize) {
                if source == ControllerWriteSource::FormatTrack {
                    return Ok(ControllerWriteOutcome::PreservedProtectedSector);
                }
                self.write_at(offset, buf)?;
                return Ok(ControllerWriteOutcome::AppliedProtectedSector);
            }
        }

        self.write_at(offset, buf)?;
        Ok(ControllerWriteOutcome::Applied)
    }

    fn track_map_path(&self) -> PathBuf {
        let mut p = self.image_path.clone();
        p.set_extension(format!(
            "{}fmtmap",
            self.image_path
                .extension()
                .map(|e| format!("{}.", e.to_string_lossy()))
                .unwrap_or_default()
        ));
        p
    }

    fn load_track_map(&mut self) -> std::io::Result<()> {
        let path = self.track_map_path();
        if !path.exists() {
            return Ok(());
        }
        let mut f = OpenOptions::new().read(true).open(path)?;
        let mut magic = [0u8; 8];
        f.read_exact(&mut magic)?;
        if &magic != b"K10FMTM1" {
            return Ok(());
        }
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        for (i, v) in self.formatted_tracks.iter_mut().enumerate() {
            let b = i / 8;
            let m = 1u8 << (i % 8);
            *v = b < bytes.len() && (bytes[b] & m) != 0;
        }
        Ok(())
    }

    fn persist_track_map(&mut self) -> std::io::Result<()> {
        let n = self.formatted_tracks.len();
        let mut bytes = vec![0u8; n.div_ceil(8)];
        for (i, v) in self.formatted_tracks.iter().enumerate() {
            if *v {
                bytes[i / 8] |= 1u8 << (i % 8);
            }
        }
        let path = self.track_map_path();
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        f.write_all(b"K10FMTM1")?;
        f.write_all(&bytes)?;
        f.flush()?;
        let any = self.is_formatted();
        write_header(&mut self.file, any)?;
        self.file.flush()?;
        Ok(())
    }
}

fn write_header(file: &mut std::fs::File, formatted: bool) -> std::io::Result<()> {
    let mut hdr = [0u8; HEADER_SIZE as usize];
    let text = format!("{}{}{}\n", HEADER_TEXT, FORMAT_FLAG_PREFIX, if formatted { 1 } else { 0 });
    let bytes = text.as_bytes();
    let n = bytes.len().min(hdr.len());
    hdr[..n].copy_from_slice(&bytes[..n]);
    file.seek(SeekFrom::Start(DATA_SIZE))?;
    file.write_all(&hdr)?;
    Ok(())
}

fn read_header(file: &mut std::fs::File) -> std::io::Result<[u8; HEADER_SIZE as usize]> {
    let mut hdr = [0u8; HEADER_SIZE as usize];
    file.seek(SeekFrom::Start(DATA_SIZE))?;
    file.read_exact(&mut hdr)?;
    Ok(hdr)
}

fn parse_format_flag(hdr: &[u8; HEADER_SIZE as usize]) -> Option<bool> {
    let end = hdr.iter().position(|&b| b == 0).unwrap_or(hdr.len());
    let text = std::str::from_utf8(&hdr[..end]).ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix(FORMAT_FLAG_PREFIX) {
            return match value.trim() {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            };
        }
    }
    None
}

fn detect_formatted_data(file: &mut std::fs::File) -> std::io::Result<bool> {
    const CHUNK: usize = 8192;
    let mut buf = [0u8; CHUNK];
    file.seek(SeekFrom::Start(0))?;
    let mut remaining = DATA_SIZE;
    while remaining > 0 {
        let n = usize::min(CHUNK, remaining as usize);
        file.read_exact(&mut buf[..n])?;
        if buf[..n].iter().any(|&b| b != 0) {
            return Ok(true);
        }
        remaining -= n as u64;
    }
    Ok(false)
}

fn track_index(cyl: u64, head: u64) -> Option<usize> {
    if cyl >= CYLINDERS || head >= HEADS {
        return None;
    }
    Some((cyl * HEADS + head) as usize)
}

fn ranges_overlap(a_off: u64, a_len: usize, b_off: u64, b_len: usize) -> bool {
    let a_end = a_off.saturating_add(a_len as u64);
    let b_end = b_off.saturating_add(b_len as u64);
    a_off < b_end && b_off < a_end
}
