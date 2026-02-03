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

**Strobe Mode:**
- Strobe ON (0x20): R18/R19 writes set addr_hardware (for CRTC hardware use)
- Strobe OFF (0x00): R18/R19 writes set addr_latch (for port 0x1F character writes)

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

**Port 0x1F Dual Purpose:**
- Value 0x00: Clear strobe only (no VRAM write)
- Value 0x20: Write space to VRAM at addr_latch AND set strobe (enables screen clear)
- Other values: Write character to VRAM at addr_latch
- All writes auto-increment addr_latch

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
