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

        let enable_trace = name.contains("Advent 1k DSDD");
        let mut fdc = FloppyController::new(
            "__nonexistent_test_a__",
            "__nonexistent_test_b__",
            format,
            side1_sector_base,
            enable_trace,
            enable_trace,
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
            &[(false, 2, 10, 0, 0)],
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
}
