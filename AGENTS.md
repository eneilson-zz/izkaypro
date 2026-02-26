# izkaypro Agent Handoff Notes

## Scope
- Project: Rust Kaypro emulator (`izkaypro`)
- Supported platforms: Linux, macOS, FreeBSD, Windows 11

## Current Known State
- All shipping models boot in diagnostics:
  - Kaypro II, 4/84, TurboROM, TurboROM+HD, KayPLUS, Kaypro 10: PASS

## Kaypro 10 Hard Disk + Floppy Architecture

### Drive Mapping
- A: and B: are hard disk partitions (WD1002-05 controller, ports 0x80–0x87)
- C: is the single floppy drive (WD1793 FDC, ports 0x10–0x13)
- F5 inserts a floppy as "Drive C (floppy)"; F6 is disabled (one floppy only)

### HD Boot Priority
The 81-478c ROM checks FDC NOT READY (port 0x10, bit 7) at power-on.
NOT READY → HD boot, READY → floppy boot. The `disk_in_drive` flag on
`FloppyController` controls this independently of motor state. Set
`disk_in_drive = false` when booting from HD without a user-specified floppy.

### Floppy Motor Control — Kaypro 10 vs Other Models
**Critical difference:** On all floppy-only models (Kaypro II, 4/84,
TurboROM, KayPLUS), bit 4 of port 0x14 controls the floppy motor. Their
BIOSes explicitly set this bit before floppy I/O.

On the Kaypro 10, the 81-478c ROM **never sets bit 4**. The floppy motor
is auto-started by drive selection on the real hardware. In the emulator,
`update_system_bits_k484()` forces `motor_on = true` when `hard_disk` is
present, ignoring bit 4. Without this, the FDC never reports index pulses
or Head Loaded, and SELDSK for drive C fails with "Bdos Err On C: Select".

### WD1793 NOT READY Bit (Status Bit 7)
On real hardware, NOT READY is a real-time signal reflecting the drive's
READY input — not a latched value from the last command. `get_status()`
dynamically applies the `disk_in_drive` flag to bit 7, so inserting or
removing a disk takes effect immediately without requiring a new FDC command.

### FDC Trace Routing (`fdc_log!` macro)
FDC trace output uses the `fdc_log!` macro which writes to `trace_file`
when set, otherwise falls back to `println!`. HDC trace output uses
`hdc_log!` with the same pattern. The `trace_file` fields on
`FloppyController` and `HardDisk` can be set programmatically for
file-based tracing.

## TurboROM+HD Architecture (`turbo_rom_hd` model)

### Overview
TurboROM 3.4 with WD1002-05 hard disk controller. Uses the same WD1002-05
emulation as Kaypro 10 but with TurboROM's Advent-style HD BIOS instead of
the Kaypro 10 Universal ROM.

### HD Detection — ADV1 Signature
TurboROM identifies hard disks by reading a parameter sector at Cylinder 0,
Head 0, Sector 8 (offset 0x2000 in the image). It looks for an "ADV1"
signature (bytes 41 44 56 31) at the start. This sector contains the disk
geometry, partition layout, and BIOS configuration. Without this signature,
TurboROM falls back to floppy-only boot (loading TURBO-BIOS from disk).

### LUN Configuration
TurboROM probes both LUN 1 (SDH=0xC8) and LUN 2 (SDH=0xD0). Only LUN 1
should report READY. If both LUNs report READY with the same backing image,
the ROM sees two identical drives and creates 4 HD partitions instead of 2.
The default `ready_lun_mask` (LUN 1 only) is correct.

### Drive Mapping
- A: and B: are 5MB hard disk partitions (via TurboROM Advent BIOS)
- C: and D: are floppy drives (via Advent Personality/Decoder Board)
- No floppies are inserted by default; use F5/F6 or `--drivea`/`--driveb`
- The UI labels floppy drives as C and D (set via `screen.floppy_drive_labels`)

### HD Image Setup
The HD image (`disks/system/turborom.hd`) must be prepared:
1. Format with HDFMT (creates sector headers on all tracks)
2. Write system tracks to physical sector 0 (e.g., from turb5600.sys)
3. The parameter sector at C0/H0/S8 must contain the ADV1 signature

### FORMAT TRACK Persistence
When HDFMT formats a track, the emulator fills the track data with 0xE5
(standard CP/M blank fill) and persists it to the image file. This ensures
`detect_formatted_tracks` correctly identifies formatted tracks on reload.
Without this, formatted-but-never-written tracks appear as unformatted
(all zeros) after reloading the image, causing WRITE SECTOR to fail with
ID NOT FOUND.

### Advent Personality/Decoder Board PIO (ports 0x88–0x8B)
The Advent board provides additional floppy drive select lines beyond
the 2 native lines on port 0x14. This is needed because port 0x14 bit 1
is shared with the WD1002-05 SASI reset on systems with a hard disk.

Port emulation:
- **0x88 write**: PIO output latch (accepted, not used by emulator)
- **0x88 read**: Returns 0xFF (unconnected input pins — no RAM disk)
- **0x89 write**: PIO control (accepted, ignored)
- **0x8A write**: Drive select (values 1–2 → floppy drive 0–1)
- **0x8B read**: Status switch (returns 0x00 = all drives present)

The Advent board is enabled for HD systems that are NOT Kaypro 10
hardware (`advent_pio_enabled = has_hard_disk && !is_kaypro10_hardware`).

### Advent RAM Disk — Why It's Disabled
The Advent RAM Disk is a physical hardware add-on (up to 2MB) that
TurboROM auto-detects through the PIO data port (0x88). On real
hardware, the PIO read port returns input pin state (separate from
the output latch). The RAM disk echoes back written data; without one,
pins float high (0xFF).

Previously, port 0x88 used a FIFO that echoed writes back on reads.
This caused TurboROM to falsely detect a RAM disk as Drive E. Since
the emulator has no backing storage for the RAM disk, file operations
failed with "CANNOT CLOSE DESTINATION FILE". Returning 0xFF (no device)
prevents detection entirely.

### SASI Reset — Quick Reset for Advent Board
TurboROM checks BUSY after a very short delay when toggling port 0x14
bit 1 to decide if the bit is shared with SASI reset. On the Advent
board, the WD1002-05 must appear READY quickly so `sltmsk` stays at
0x03 (4-drive select mode, allowing 2 floppy drives). The `quick_reset`
flag on `HardDisk` sets `busy_countdown=1` for immediate completion.

### Existing Image Loading
When loading an existing HD image, all tracks are marked as formatted.
This avoids the heuristic failure where formatted-but-empty tracks (all
zeros) would be incorrectly marked as unformatted. New blank images start
with all tracks unformatted (requiring HDFMT before use).

## Cross-Platform Architecture

### Terminal I/O — Why Two Keyboard Modules
The emulator renders directly to the terminal using hand-written ANSI escape
sequences and raw keyboard input. `crossterm` was evaluated but rejected due
to unacceptable rendering performance in the emulation loop.

The solution uses conditional compilation (`#[cfg(unix)]` / `#[cfg(windows)]`)
with platform-specific keyboard modules that share the same public API:

- **`src/keyboard_unix.rs`** — Uses the `termios` crate for raw mode and
  parses ANSI escape sequences (CSI/SS3) from stdin for function keys and
  arrow keys. Unix-only dependencies: `termios`, `libc`.

- **`src/keyboard_win.rs`** — Uses `windows-sys` Win32 Console API.
  `ReadConsoleInputW` provides `KEY_EVENT` records with virtual key codes
  (VK_F1, VK_UP, etc.), avoiding ANSI parsing entirely.
  **Important:** Do NOT set `ENABLE_VIRTUAL_TERMINAL_INPUT` on the stdin
  handle — it converts key presses into ANSI sequences, breaking VK code
  detection and causing function keys to stop working after `read_line()`.

- **`src/screen.rs`** — Uses ANSI escape sequences on all platforms. On
  Windows, `Screen::init()` calls `SetConsoleMode` with
  `ENABLE_VIRTUAL_TERMINAL_PROCESSING` on the **stdout** handle so the
  existing ANSI output works unchanged.

### Serial Port (`src/sio.rs`)
Serial device support (`--serial`) uses Unix-only APIs (`termios`,
`libc::ioctl` for modem control, `libc::tcsendbreak`). On Windows,
`open_serial()` returns an error. Serial support can be added later using
the `serialport` crate.

### Real-Time Clock (`src/rtc.rs`)
Timezone offset detection is platform-specific:
- Unix: `libc::localtime_r` → `tm_gmtoff`
- Windows: Inline FFI to `GetTimeZoneInformation` (defined locally to
  avoid `windows-sys` feature flag issues with `Win32_System_Time`)

### Platform Dependencies (`Cargo.toml`)
```toml
[target.'cfg(unix)'.dependencies]
termios = "^0.3.3"
libc = "0.2"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = [
    "Win32_System_Console",
    "Win32_Foundation",
] }
```

## GitHub Actions Release Workflow

`.github/workflows/release.yml` triggers on any `v*` tag push and builds
release binaries for four targets:

| Target | Runner | Archive |
|--------|--------|---------|
| `x86_64-unknown-linux-musl` | ubuntu-latest | tar.gz |
| `x86_64-apple-darwin` | macos-latest | tar.gz |
| `aarch64-apple-darwin` | macos-latest | tar.gz |
| `x86_64-pc-windows-msvc` | windows-latest | zip |

Linux uses the `musl` target for static linking (runs on any distro without
glibc version requirements).

Each archive includes the binary plus `disks/`, `roms/`, `izkaypro.toml`,
`README.md`, and `LICENSE` — everything needed to run.

The `release` job downloads all artifacts and creates a GitHub Release with
auto-generated release notes via `softprops/action-gh-release`.

## Known Issue: Kaypro 10 HD DIRBUF Overflow (NZ-COM Incompatibility)

### Problem
The Kaypro 10 BIOS does not deblock HD sectors. It transfers the full
512-byte physical sector to the DMA address set by SETDMA, even though
CP/M 2.2 logical sectors are 128 bytes. When the BDOS performs directory
reads, it sets DMA to the 128-byte DIRBUF (Directory Buffer) in the DPH.
The BIOS then writes 512 bytes starting at DIRBUF, overflowing 384 bytes
into whatever memory follows.

### Why It's Normally Harmless
The original Kaypro 10 CP/M 2.2 places DIRBUF in a memory location where
the 384-byte overflow lands in unused padding or safe areas. The original
system works fine despite the lack of deblocking.

### Why NZ-COM Breaks
NZ-COM (Z-System overlay) packs its internal structures (ZRDOS hash
tables, environment descriptors, RSX state) tightly in high memory. When
the BIOS overwrites 512 bytes at DIRBUF (0xFAF7 in the tested config),
the overflow (0xFB77–0xFCF6) corrupts ZRDOS internals. Symptoms include:
- Files "disappearing" from directory listings after a few operations
- BDOS issuing F_USERNUM calls with invalid values (e.g., 0xDF)
- Directory searches stuck reading the same sector repeatedly

### Verified Memory Layout (from tracing)
- DIRBUF: 0xFAF7 (128 bytes expected, 512 bytes actually written)
- ALV: 0xF848 (below DIRBUF — safe from overflow)
- CSV: 0x0000 (not used for HD)
- Overflow zone: 0xFB77–0xFCF6 (384 bytes of ZRDOS/CBIOS data corrupted)

### Potential Fix: Emulator-Level Deblocking
The proper fix would be to intercept BIOS READ/WRITE for HD sectors at
the emulator level and implement standard CP/M 2.2 sector deblocking:
- **READ**: Read 512-byte physical sector into a private buffer, copy
  only the target 128-byte logical sector to the DMA address.
- **WRITE**: Read 512-byte physical sector, overlay the 128-byte DMA
  data onto the correct slice, write the full 512 bytes back to disk.
This would require tracking the logical-to-physical sector mapping
(from the DPB sector translate table). Not yet implemented.

### Status
Kaypro 10 works correctly with its original CP/M 2.2. The DIRBUF
overflow only affects third-party DOS replacements (NZ-COM, ZCPR, etc.)
that pack data structures tightly after DIRBUF.

## Logs and Test Command
- Run boot diagnostics:
  - `cargo run -q -- --boot-test`

## Working Rule
- After **every** code change, run:
  - `cargo run -q -- --boot-test`
- All six models must PASS before committing.


