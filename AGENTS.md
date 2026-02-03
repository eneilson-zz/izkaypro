# izkaypro - Kaypro Emulator

## Project Overview

A Rust-based emulator for Kaypro computers that runs in a terminal. Uses the [iz80](https://github.com/ivanizag/iz80) library for Z80 CPU emulation.

## Supported Models

- **Kaypro II** - Original single-sided double-density (SSDD) 200KB disks
- **Kaypro 4-84** - Double-sided double-density (DSDD) 400KB disks

## Build & Run

```bash
cargo build --release
cargo run -- [options] [disk_a] [disk_b]
```

### Trace Options
- `-c, --cpu-trace` - Trace CPU instructions
- `-i, --io-trace` - Trace I/O port access
- `-f, --fdc-trace` - Trace floppy disk controller
- `-r, --rom-trace` - Trace ROM entry points
- `-b, --bdos-trace` - Trace CP/M BDOS calls
- `-s, --system-bits` - Trace system bit changes

## Disk Formats

| Format | Size | Tracks | Sides | Sectors/Track | Bytes/Sector |
|--------|------|--------|-------|---------------|--------------|
| SSDD   | 200KB | 40 | 1 | 10 | 512 |
| DSDD   | 400KB | 40 | 2 | 10 (0-9 side0, 10-19 side1) | 512 |

## Recent Changes (Kaypro 4-84 Support)

### Port 0x14 System PIO (Kaypro 4-84)
Added support for port 0x14 which controls system bits on Kaypro 4-84:
- Bit 7: BANK (0=RAM, 1=ROM/Video)
- Bit 6: Character set
- Bit 5: Density (0=double, 1=single)
- Bit 4: Motor (0=off, 1=on)
- Bit 3: Centronics strobe
- Bit 2: Side select (1=side0, 0=side1) - **inverted from port 0x1C**
- Bits 1-0: Drive select (A=01, B=10)

### FDC Behavior Fixes
- Removed incorrect sector register auto-increment after READ SECTOR
- Fixed READ ADDRESS to not modify sector register
- READ ADDRESS returns head byte = 0 for both sides (Kaypro 4-84 format)

### ROM Shadowing
ROM content is copied to RAM at startup. This is needed for ROMs (like Turbo ROM) that switch to RAM bank mode and expect to continue executing the same code from RAM. The shadowing happens in `KayproMachine::new()` in kaypro_machine.rs.

### SY6545 CRTC Emulation (Kaypro 2X/4/84)
The 81-292a ROM uses the SY6545 CRTC for video instead of memory-mapped VRAM:

**I/O Ports:**
- Port 0x1C: CRTC register select (0-17 for timing, 18-19 for VRAM address)
- Port 0x1D: CRTC register data
- Port 0x1F: Control/strobe AND character data

**Strobe Flag:**
- The strobe flag is set by writing 0x20 to port 0x1F, cleared by writing 0x00
- Used only for detecting ROM vs diag4 protocol in port 0x1F write handling
- R18/R19 writes always update addr_latch regardless of strobe state

**VRAM Write Protocol:**
1. `OUT 0x1F, 0x00` - Strobe OFF (CPU mode)
2. `OUT 0x1D, addr_hi` - R18: Set addr_latch high byte (auto-increments to R19)
3. `OUT 0x1D, addr_lo` - R19: Set addr_latch low byte (auto-increments to R18)
4. `OUT 0x1F, character` - Write character to VRAM at addr_latch

Note: R18/R19 auto-increment between each other on writes to port 0x1D.
The ROM uses both strobe modes - ON for hardware setup, OFF for VRAM writes.

**Display Layout:**
- 80-byte linear rows starting from R12:R13 (start_addr = 0x0000)
- VRAM addresses 0x0000-0x077F cover 24 rows of 80 columns
- Boot screen at rows 10-13 (0x0320-0x044F), CP/M at rows 1-3 (0x0050-0x013F)

**Port 0x1F Dual Purpose (two protocols):**

*ROM protocol* (81-292a) - reg_index is NOT 0x1F:
- Value 0x00: Clear strobe only (no VRAM write)
- Value 0x20: Write space to VRAM at addr_latch AND set strobe (enables screen clear)
- Other values: Write character to VRAM at addr_latch

*diag4 protocol* - strobe command (0x1F) sent to port 0x1C first (reg_index == 0x1F):
- All values (including 0x00 and 0x20) are written as data to VRAM
- Addresses are set explicitly via R18/R19 before each access

Both protocols auto-increment addr_latch after writes.

**Hardware Scrolling:**
- ROM uses R12:R13 to change start_addr for hardware scrolling
- VRAM is treated as a 2KB (0x800) circular buffer
- Display addresses wrap: `addr = (start_addr + row * 80 + col) & 0x7FF`

### Switching Configurations

To switch between Kaypro models, update **both** files:

1. **src/kaypro_machine.rs** - Comment/uncomment the ROM selection
2. **src/floppy_controller.rs** - Comment/uncomment the matching disk selection

Available configurations:
- **Kaypro II**: 81-149c.rom + cpm22-rom149.img (SSDD)
- **Kaypro 4/83**: 81-232.rom + k484-cpm22f-boot.img (DSDD)
- **Kaypro 2X/4/84 (81-292a)**: 81-292a.rom + k484-cpm22f-boot.img (DSDD) ← **Currently active**
  - Uses SY6545 CRTC emulation for "transparent" VRAM addressing
- **Kaypro 4-84 Turbo ROM**: trom34.rom + k484_turborom_63k_boot.img (DSDD, 8KB ROM)
  - ⚠️ **Not working**: Turbo ROM 3.4 polls port 0x1C bit 7 expecting external hardware to set the Bank bit.

### Key Files
- `src/kaypro_machine.rs` - Machine emulation, port handling, memory banking, ROM selection
- `src/floppy_controller.rs` - WD1793 FDC emulation, disk selection
- `src/media.rs` - Disk image handling and sector mapping
- `src/keyboard_unix.rs` - Terminal keyboard handling, function keys, escape sequences
- `src/screen.rs` - Terminal display, help overlay, disk selection prompts

### Debugging Methodology

When implementing new features or fixing issues, follow this incremental approach:

1. **Make small, testable changes** - One feature or fix at a time
2. **Use tracing flags** to understand behavior:
   - `-v` (crtc-trace): VRAM writes, register changes
   - `-i` (io-trace): All I/O port access
   - `-f` (fdc-trace): Floppy controller commands
   - `-c` (cpu-trace): CPU instruction execution
3. **Test immediately** after each change with `cargo run`
4. **If behavior worsens**, revert and investigate further before proceeding
5. **Document discoveries** in AGENTS.md and code comments

### Key Debugging Insights from SY6545 Work

**Scrolling Issue Root Cause:**
- The 81-292a ROM uses hardware scrolling via R12:R13 (start_addr)
- VRAM addresses wrap at 2KB (0x800), not the full 16KB
- Fix: `addr = (start + row * 80 + col) & 0x7FF` in screen.rs

**Critical Discovery - Port 0x1F Dual Behavior:**
- Value 0x00: Strobe OFF only (no VRAM write)
- Value 0x20: Write space to VRAM AND set strobe ON (enables screen clear)
- Other values: Write character to VRAM at addr_latch, then auto-increment

**Memory-Mapped to CRTC VRAM Translation:**
- Memory-mapped (0x3000-0x3FFF): 128-byte row stride
- CRTC VRAM: 80-byte linear rows
- Mirroring code must translate: `crtc_addr = row * 80 + col` (if col < 80)

### EMUTEST.COM Diagnostic Tool

Located in `disks/cpm22-emudiags.img` (Drive B), source in `doc/emutest.asm`:

1. **ROM Checksum Test**: Relocates code to 0x8000, switches to ROM bank via port 0x14 bit 7, calculates checksum, switches back
2. **RAM Tests**: Sliding-data and address-data patterns on 0x4000-0x7FFF and 0x8000-0xBFFF
3. **VRAM Test**: Tests 2KB (0x000-0x7FF) via SY6545 CRTC transparent addressing protocol

Run built-in diagnostics: `cargo run -- --diagnostics`

### SY6545 CRTC Transparent Addressing Protocol (for VRAM access)

Based on diag4.mac `cr4`/`cr5`/`cr6` routines for Kaypro 4 universal (`univ=true`):

**Read VRAM byte at address HL:**
1. `OUT 0x1C, 0x12` - Select R18 (Update Address High)
2. `OUT 0x1D, H` - Write high byte of address
3. `OUT 0x1C, 0x13` - Select R19 (Update Address Low)  
4. `OUT 0x1D, L` - Write low byte of address
5. `OUT 0x1C, 0x1F` - Send strobe command
6. Wait: `IN 0x1C` until bit 7 = 1 (Update Ready)
7. `IN 0x1F` - Read VRAM data byte

**Write VRAM byte at address HL:**
1-6. Same as read (set address and strobe, wait for ready)
7. `OUT 0x1F, A` - Write VRAM data byte
8. Wait: `IN 0x1C` until bit 7 = 1 (Write complete)

**Constants from diag4.mac:**
- `rwcmd = 0x121C` (B=0x12 for R18, C=0x1C for port)
- `strcmd = 0x1F` (strobe command value)
- `vcdata = 0x1F` (VRAM data port)

### Function Keys (macOS)
macOS terminals send application-mode escape sequences. The emulator handles both Linux and macOS:
- F1 (Help): `OP`/`Op`
- F2 (Status): `OQ`/`Oq` - **may be intercepted by macOS for brightness**
- F4 (Quit): `OS`/`Os`
- F5 (Disk A): `[15~`/`Ot`
- F6 (Disk B): `[17~`/`Ou`
- F7 (Save BIOS): `[18~`/`Ov` - **may be intercepted by macOS for media controls**
- F8 (CPU Trace): `[19~`/`Ol`

ESC cancels disk selection prompts (F5/F6).

## Code Architecture

```
main.rs
├── kaypro_machine.rs  (Machine trait impl, I/O ports)
│   ├── Port 0x10-0x13: FDC registers
│   ├── Port 0x14: System PIO (Kaypro 4-84)
│   └── Port 0x1C: System bits (Kaypro II)
├── floppy_controller.rs (WD1793 commands)
│   └── media.rs (Disk image format handling)
├── keyboard_unix.rs
└── screen.rs
```

## Testing

To test Kaypro 4-84 mode with DSDD disk:
```bash
cargo run -- disks/k484-cpm22f-boot.img
```

To test Kaypro II mode with SSDD disk:
```bash
cargo run -- disks/cpm22-rom149.img
```
