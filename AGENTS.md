# izkaypro Agent Handoff Notes

## Scope
- Project: Rust Kaypro emulator (`izkaypro`)
- Supported platforms: Linux, macOS, FreeBSD, Windows 11

## Current Known State
- All shipping models boot in diagnostics:
  - Kaypro II, 4/84, TurboROM, KayPLUS: PASS
- Kaypro 10 hard disk support is on the `kaypro10-wip` branch (incomplete, not merged)

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
when `--trace-log` is active, otherwise falls back to `println!`. This
allows full FDC tracing without disrupting screen rendering. When
`--trace-log FILE` is specified, FDC command-level tracing is auto-enabled
and output goes to `FILE-fdc.log`.

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

## Logs and Test Command
- Run boot diagnostics:
  - `cargo run -q -- --boot-test`

## Working Rule
- After **every** code change, run:
  - `cargo run -q -- --boot-test`
- All four models must PASS before committing.


