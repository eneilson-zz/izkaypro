#[cfg(test)]
mod tests {
    use crate::floppy_controller::FloppyController;
    use crate::media::MediaFormat;

    fn build_format_stream(density_sd: bool, track: u8, head: u8, n: u8, sectors: &[u8], fill: u8) -> Vec<u8> {
        let sector_size = 128usize << (n as usize);
        let mut stream = Vec::new();

        if density_sd {
            for _ in 0..16 { stream.push(0xFF); }

            for &sec_id in sectors {
                for _ in 0..3 { stream.push(0x00); }
                stream.push(0xFE);
                stream.push(track);
                stream.push(head);
                stream.push(sec_id);
                stream.push(n);
                stream.push(0xF7);
                for _ in 0..11 { stream.push(0xFF); }
                for _ in 0..3 { stream.push(0x00); }
                stream.push(0xFB);
                for _ in 0..sector_size { stream.push(fill); }
                stream.push(0xF7);
                for _ in 0..10 { stream.push(0xFF); }
            }
            while stream.len() < 3125 {
                stream.push(0xFF);
            }
        } else {
            for _ in 0..80 { stream.push(0x4E); }

            for &sec_id in sectors {
                for _ in 0..12 { stream.push(0x00); }
                for _ in 0..3 { stream.push(0xF5); }
                stream.push(0xFE);
                stream.push(track);
                stream.push(head);
                stream.push(sec_id);
                stream.push(n);
                stream.push(0xF7);
                for _ in 0..22 { stream.push(0x4E); }
                for _ in 0..12 { stream.push(0x00); }
                for _ in 0..3 { stream.push(0xF5); }
                stream.push(0xFB);
                for _ in 0..sector_size { stream.push(fill); }
                stream.push(0xF7);
                for _ in 0..24 { stream.push(0x4E); }
            }
            while stream.len() < 12000 {
                stream.push(0x4E);
            }
        }

        stream
    }

    fn apply_skew(sector_count: u8, sector_base: u8, skew: u8) -> Vec<u8> {
        if skew == 0 {
            return (sector_base..sector_base + sector_count).collect();
        }
        let mut result = vec![0u8; sector_count as usize];
        let mut pos = 0usize;
        for i in 0..sector_count {
            result[pos] = sector_base + i;
            pos = (pos + skew as usize) % sector_count as usize;
        }
        result
    }

    fn format_and_verify(
        name: &str,
        image_size: usize,
        format: MediaFormat,
        side1_sector_base: u8,
        tracks_config: &[(bool, u8, u8, u8, u8)], // (single_density, n, spt, sector_base, skew)
        num_tracks: u8,
        sides: u8,
    ) -> usize {
        let blank = vec![0xE5u8; image_size];

        let mut fdc = FloppyController::new(
            "__nonexistent_test_a__",
            "__nonexistent_test_b__",
            format,
            side1_sector_base,
            false,
            false,
        );

        fdc.media_b_mut().content = blank;
        fdc.media_b_mut().format = format;
        fdc.media_b_mut().learned_n = None;
        fdc.media_b_mut().learned_sector_base = None;
        fdc.media_b_mut().track_geometry.clear();
        fdc.media_b_mut().write_protected = false;

        fdc.set_drive(1);
        fdc.set_motor(true);

        let mut failed_sectors = 0;

        for phys_track in 0..num_tracks {
            for side in 0..sides {
                let side_2 = side == 1;
                fdc.set_side(side_2);

                let config_idx = if tracks_config.len() == 1 {
                    0
                } else {
                    if phys_track == 0 { 0 } else { 1 }
                };
                let (single_density, n, spt, sector_base, skew) = tracks_config[config_idx];

                fdc.set_single_density(single_density);

                let sectors = apply_skew(spt, sector_base, skew);
                let stream = build_format_stream(single_density, phys_track, side as u8, n, &sectors, 0xE5);

                fdc.put_track(phys_track);
                fdc.head_position = phys_track;

                fdc.put_command(0xF0);

                for &byte in &stream {
                    fdc.put_data(byte);
                    if !fdc.write_track_active {
                        break;
                    }
                }

                if fdc.write_track_active {
                    fdc.put_command(0xD0);
                }
            }
        }

        for phys_track in 0..num_tracks {
            for side in 0..sides {
                let side_2 = side == 1;
                fdc.set_side(side_2);

                let config_idx = if tracks_config.len() == 1 {
                    0
                } else {
                    if phys_track == 0 { 0 } else { 1 }
                };
                let (single_density, n, spt, sector_base, _skew) = tracks_config[config_idx];
                let sector_size = 128usize << (n as usize);

                fdc.set_single_density(single_density);
                fdc.head_position = phys_track;
                fdc.put_track(phys_track);

                for sec_idx in 0..spt {
                    let sec_id = sector_base + sec_idx;
                    fdc.put_sector(sec_id);
                    fdc.put_command(0x80);

                    let mut sector_data = Vec::new();
                    for _ in 0..sector_size {
                        let byte = fdc.get_data();
                        sector_data.push(byte);
                    }

                    for _ in 0..20 {
                        let status = fdc.get_status();
                        if status & 0x01 == 0 { break; }
                    }

                    let all_ok = sector_data.iter().all(|&b| b == 0xE5);
                    if !all_ok {
                        let bad_count = sector_data.iter().filter(|&&b| b != 0xE5).count();
                        let first_bad = sector_data.iter().position(|&b| b != 0xE5).unwrap_or(0);
                        eprintln!("  FAIL: {} track {} side {} sector {}: {}/{} bytes wrong (first bad at [{}]={:02x}, bytes[510..514]: {:02x} {:02x} {:02x} {:02x})",
                            name, phys_track, side, sec_id, bad_count, sector_size, first_bad,
                            sector_data.get(first_bad).copied().unwrap_or(0),
                            sector_data.get(510).copied().unwrap_or(0),
                            sector_data.get(511).copied().unwrap_or(0),
                            sector_data.get(512).copied().unwrap_or(0),
                            sector_data.get(513).copied().unwrap_or(0));
                        failed_sectors += 1;
                    }
                }
            }
        }

        if failed_sectors == 0 {
            eprintln!("  PASS: {} - all sectors verified", name);
        } else {
            eprintln!("  FAIL: {} - {} sectors failed verification", name, failed_sectors);
        }

        failed_sectors
    }

    #[test]
    fn test_osborne_sssd() {
        let failures = format_and_verify(
            "Osborne SSSD",
            102400,
            MediaFormat::SsSd,
            0,
            &[(true, 1, 10, 1, 1)],
            40,
            1,
        );
        assert_eq!(failures, 0, "Osborne SSSD: {} sectors failed", failures);
    }

    #[test]
    fn test_osborne_ssdd() {
        let failures = format_and_verify(
            "Osborne SSDD",
            204800,
            MediaFormat::SsDd,
            0,
            &[(true, 0, 18, 1, 1)],
            40,
            1,
        );
        assert_eq!(failures, 0, "Osborne SSDD: {} sectors failed", failures);
    }

    #[test]
    fn test_xerox820_sssd() {
        let failures = format_and_verify(
            "Xerox 820 SSSD",
            204800,
            MediaFormat::SsDd,
            0,
            &[(false, 2, 10, 0, 0)],
            40,
            1,
        );
        assert_eq!(failures, 0, "Xerox 820 SSSD: {} sectors failed", failures);
    }

    #[test]
    fn test_xerox820_dssd() {
        let failures = format_and_verify(
            "Xerox 820 DSSD",
            204800,
            MediaFormat::SsDd,
            0,
            &[(false, 3, 5, 1, 3)],
            40,
            1,
        );
        assert_eq!(failures, 0, "Xerox 820 DSSD: {} sectors failed", failures);
    }

    #[test]
    fn test_xerox820ii_ssdd() {
        let failures = format_and_verify(
            "Xerox 820-II SSDD",
            204800,
            MediaFormat::SsDd,
            0,
            &[
                (true, 0, 18, 1, 1),
                (false, 1, 17, 1, 3),
            ],
            40,
            1,
        );
        assert_eq!(failures, 0, "Xerox 820-II SSDD: {} sectors failed", failures);
    }

    #[test]
    fn test_xerox820ii_dsdd() {
        let failures = format_and_verify(
            "Xerox 820-II DSDD",
            409600,
            MediaFormat::DsDd,
            0,
            &[
                (true, 0, 18, 1, 1),
                (false, 1, 17, 1, 3),
            ],
            40,
            2,
        );
        assert_eq!(failures, 0, "Xerox 820-II DSDD: {} sectors failed", failures);
    }

    #[test]
    fn test_kaypro_ssdd() {
        let failures = format_and_verify(
            "Kaypro SSDD",
            409600,
            MediaFormat::DsDd,
            0,
            &[(false, 3, 5, 1, 3)],
            40,
            1,
        );
        assert_eq!(failures, 0, "Kaypro SSDD: {} sectors failed", failures);
    }

    #[test]
    fn test_kaypro_dsdd() {
        let failures = format_and_verify(
            "Kaypro DSDD",
            409600,
            MediaFormat::DsDd,
            0,
            &[(false, 2, 10, 0, 0)],
            40,
            2,
        );
        assert_eq!(failures, 0, "Kaypro DSDD: {} sectors failed", failures);
    }

    #[test]
    fn test_advent_ssdd() {
        let failures = format_and_verify(
            "Advent 1k SSDD",
            409600,
            MediaFormat::DsDd,
            0,
            &[(false, 3, 5, 1, 3)],
            40,
            1,
        );
        assert_eq!(failures, 0, "Advent 1k SSDD: {} sectors failed", failures);
    }

    #[test]
    fn test_advent_dsdd() {
        let failures = format_and_verify(
            "Advent 1k DSDD",
            409600,
            MediaFormat::DsDd,
            0,
            &[(false, 3, 5, 1, 3)],
            40,
            2,
        );
        assert_eq!(failures, 0, "Advent 1k DSDD: {} sectors failed", failures);
    }

    fn build_format_stream_unique(density_sd: bool, track: u8, head: u8, n: u8, sectors: &[u8]) -> Vec<u8> {
        let sector_size = 128usize << (n as usize);
        let mut stream = Vec::new();

        if density_sd {
            for _ in 0..16 { stream.push(0xFF); }
            for &sec_id in sectors {
                let fill = sec_id.wrapping_add(track).wrapping_add(head * 0x40);
                for _ in 0..3 { stream.push(0x00); }
                stream.push(0xFE);
                stream.push(track);
                stream.push(head);
                stream.push(sec_id);
                stream.push(n);
                stream.push(0xF7);
                for _ in 0..11 { stream.push(0xFF); }
                for _ in 0..3 { stream.push(0x00); }
                stream.push(0xFB);
                for _ in 0..sector_size { stream.push(fill); }
                stream.push(0xF7);
                for _ in 0..10 { stream.push(0xFF); }
            }
            while stream.len() < 3125 { stream.push(0xFF); }
        } else {
            for _ in 0..80 { stream.push(0x4E); }
            for &sec_id in sectors {
                let fill = sec_id.wrapping_add(track).wrapping_add(head * 0x40);
                for _ in 0..12 { stream.push(0x00); }
                for _ in 0..3 { stream.push(0xF5); }
                stream.push(0xFE);
                stream.push(track);
                stream.push(head);
                stream.push(sec_id);
                stream.push(n);
                stream.push(0xF7);
                for _ in 0..22 { stream.push(0x4E); }
                for _ in 0..12 { stream.push(0x00); }
                for _ in 0..3 { stream.push(0xF5); }
                stream.push(0xFB);
                for _ in 0..sector_size { stream.push(fill); }
                stream.push(0xF7);
                for _ in 0..24 { stream.push(0x4E); }
            }
            while stream.len() < 12000 { stream.push(0x4E); }
        }
        stream
    }

    #[test]
    fn test_xerox820ii_dsdd_mixed_density() {
        let image_size = 409600;
        let format = MediaFormat::DsDd;
        let num_tracks: u8 = 40;
        let sides: u8 = 2;

        let blank = vec![0x00u8; image_size];

        let mut fdc = FloppyController::new(
            "__nonexistent_test_a__",
            "__nonexistent_test_b__",
            format,
            0,
            true,
            true,
        );

        fdc.media_b_mut().content = blank;
        fdc.media_b_mut().format = format;
        fdc.media_b_mut().learned_n = None;
        fdc.media_b_mut().learned_sector_base = None;
        fdc.media_b_mut().track_geometry.clear();
        fdc.media_b_mut().write_protected = false;

        fdc.set_drive(1);
        fdc.set_motor(true);

        // Format phase: Track 0 Side 0 = SD (N=0, 18 spt, base 1),
        // all other track/sides = DD (N=1, 17 spt, base 1)
        // This matches the real Xerox 820-II DSDD format.
        for phys_track in 0..num_tracks {
            for side in 0..sides {
                let side_2 = side == 1;
                fdc.set_side(side_2);

                let (single_density, n, spt, sector_base) = if phys_track == 0 && !side_2 {
                    (true, 0u8, 18u8, 1u8)
                } else {
                    (false, 1u8, 17u8, 1u8)
                };

                fdc.set_single_density(single_density);

                let sectors: Vec<u8> = (sector_base..sector_base + spt).collect();
                let stream = build_format_stream_unique(single_density, phys_track, side, n, &sectors);

                fdc.put_track(phys_track);
                fdc.head_position = phys_track;
                fdc.put_command(0xF0);

                for &byte in &stream {
                    fdc.put_data(byte);
                    if !fdc.write_track_active { break; }
                }
                if fdc.write_track_active {
                    fdc.put_command(0xD0);
                }
            }
        }

        // Verify phase
        let mut failed_sectors = 0;
        for phys_track in 0..num_tracks {
            for side in 0..sides {
                let side_2 = side == 1;
                fdc.set_side(side_2);

                let (single_density, n, spt, sector_base) = if phys_track == 0 && !side_2 {
                    (true, 0u8, 18u8, 1u8)
                } else {
                    (false, 1u8, 17u8, 1u8)
                };
                let sector_size = 128usize << (n as usize);

                fdc.set_single_density(single_density);
                fdc.head_position = phys_track;
                fdc.put_track(phys_track);

                for sec_idx in 0..spt {
                    let sec_id = sector_base + sec_idx;
                    let expected_fill = sec_id.wrapping_add(phys_track).wrapping_add(side * 0x40);

                    fdc.put_sector(sec_id);
                    fdc.put_command(0x80);

                    let mut sector_data = Vec::new();
                    for _ in 0..sector_size {
                        sector_data.push(fdc.get_data());
                    }

                    for _ in 0..20 {
                        if fdc.get_status() & 0x01 == 0 { break; }
                    }

                    let all_ok = sector_data.iter().all(|&b| b == expected_fill);
                    if !all_ok {
                        let bad_count = sector_data.iter().filter(|&&b| b != expected_fill).count();
                        eprintln!("  FAIL: track {} side {} sector {}: expected fill 0x{:02x}, {}/{} bytes wrong, first bytes: {:02x} {:02x} {:02x} {:02x}",
                            phys_track, side, sec_id, expected_fill, bad_count, sector_size,
                            sector_data.get(0).copied().unwrap_or(0),
                            sector_data.get(1).copied().unwrap_or(0),
                            sector_data.get(2).copied().unwrap_or(0),
                            sector_data.get(3).copied().unwrap_or(0));
                        failed_sectors += 1;
                    }
                }
            }
        }

        if failed_sectors == 0 {
            eprintln!("  PASS: Xerox 820-II DSDD mixed-density - all sectors verified");
        } else {
            eprintln!("  FAIL: Xerox 820-II DSDD mixed-density - {} sectors failed", failed_sectors);
        }
        assert_eq!(failed_sectors, 0, "Xerox 820-II DSDD mixed-density: {} sectors failed", failed_sectors);
    }

    /// Reproduce the crash when a DSDD machine (Kaypro 4-84) reads an SSDD disk.
    /// The BIOS does: Restore, Read Address side 0, Read Address side 1.
    /// Side 1 fails (SSDD has no side 1). The FDC must behave like a real WD1793:
    /// stay BUSY while scanning for sector headers, then clear BUSY with RNF set.
    #[test]
    fn test_ssdd_disk_in_dsdd_machine() {
        let ssdd_image = vec![0xE5u8; 204800]; // SSDD = 204800 bytes

        let mut fdc = FloppyController::new(
            "__nonexistent_test_a__",
            "__nonexistent_test_b__",
            MediaFormat::DsDd, // DSDD machine
            10,                // Standard Kaypro side1_sector_base
            true,
            true,
        );

        // Load SSDD image in drive B
        fdc.media_b_mut().content = ssdd_image;
        fdc.media_b_mut().format = MediaFormat::SsDd;
        fdc.media_b_mut().learned_n = None;
        fdc.media_b_mut().learned_sector_base = None;
        fdc.media_b_mut().track_geometry.clear();
        fdc.media_b_mut().write_protected = false;

        fdc.set_drive(1);
        fdc.set_motor(true);

        // Step 1: Force Interrupt (BIOS init)
        fdc.put_command(0xD0);

        // Step 2: Restore (seek to track 0)
        fdc.put_command(0x00);
        assert_eq!(fdc.head_position, 0, "Head should be at track 0 after Restore");
        assert!(fdc.raise_nmi, "Restore should raise NMI");
        fdc.raise_nmi = false;

        // Step 3: Read status (BIOS checks track 0)
        let status = fdc.get_status();
        eprintln!("  Status after Restore: 0x{:02x}", status);
        assert!(status & 0x04 != 0, "Track 0 bit should be set");

        // Step 4: Read Address on side 0 — should succeed
        fdc.set_side(false);
        fdc.put_command(0xC0);
        assert!(fdc.raise_nmi, "Read Address side 0 should raise NMI");
        fdc.raise_nmi = false;

        // Poll status until BUSY clears (countdown)
        let mut busy_count = 0;
        for _ in 0..20 {
            let s = fdc.get_status();
            if s & 0x01 == 0 { break; }
            busy_count += 1;
        }
        eprintln!("  Read Address side 0: busy for {} polls", busy_count);

        let status = fdc.get_status();
        assert!(status & 0x10 == 0, "Side 0 Read Address should NOT have RNF error");

        // Read the 6-byte ID field
        for _ in 0..6 {
            fdc.get_data();
        }

        // Step 5: Read Address on side 1 — should fail (SSDD has no side 1)
        // On a real WD1793, this stays BUSY while scanning, then sets RNF.
        fdc.set_side(true);
        fdc.put_command(0xC0);
        assert!(fdc.raise_nmi, "Read Address side 1 should raise NMI");
        fdc.raise_nmi = false;

        // The error path MUST set BUSY with a countdown (like the success path)
        // so the BIOS can poll and see BUSY→not-BUSY transition before checking RNF.
        let status_immediate = fdc.get_status();
        eprintln!("  Read Address side 1 immediate status: 0x{:02x} (busy:{})",
            status_immediate, status_immediate & 0x01 != 0);

        // Poll until BUSY clears
        let mut final_status = status_immediate;
        for _ in 0..20 {
            final_status = fdc.get_status();
            if final_status & 0x01 == 0 { break; }
        }
        eprintln!("  Read Address side 1 final status: 0x{:02x} (rnf:{})",
            final_status, final_status & 0x10 != 0);
        assert!(final_status & 0x10 != 0, "Side 1 Read Address should have RNF error");

        // Step 6: After the error, try reading sector 0 on side 0 track 0.
        // The BIOS should still be able to read SSDD sectors.
        fdc.set_side(false);
        fdc.head_position = 0;
        fdc.put_sector(0);
        fdc.put_command(0x80); // READ SECTOR
        assert!(fdc.raise_nmi, "Read Sector should raise NMI");
        fdc.raise_nmi = false;

        let status = fdc.get_status();
        eprintln!("  Read Sector side 0 sector 0 status: 0x{:02x} (busy:{}, rnf:{})",
            status, status & 0x01 != 0, status & 0x10 != 0);

        // Should be BUSY with data ready (not RNF)
        assert!(status & 0x01 != 0, "Read Sector should be BUSY");
        assert!(status & 0x10 == 0, "Read Sector side 0 sector 0 should NOT have RNF");

        // Read 512 bytes of sector data
        let mut sector_data = Vec::new();
        for _ in 0..512 {
            sector_data.push(fdc.get_data());
        }
        let all_e5 = sector_data.iter().all(|&b| b == 0xE5);
        assert!(all_e5, "Sector data should be all 0xE5");

        eprintln!("  PASS: SSDD disk in DSDD machine - side 0 readable, side 1 returns RNF");
    }
}
