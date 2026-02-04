# izkaypro - Kaypro Emulator

## Project Overview

A Rust-based emulator for Kaypro computers that runs in a terminal. Uses the [iz80](https://github.com/ivanizag/iz80) library for Z80 CPU emulation.

## Supported Models

- **Kaypro II** - Original single-sided double-density (SSDD) 200KB disks, memory-mapped VRAM
- **Kaypro 4/83** - Double-sided double-density (DSDD) 400KB disks, memory-mapped VRAM
- **Kaypro 2X/4/84** - DSDD 400KB disks, SY6545 CRTC with port-based VRAM ← **Currently active**

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
- `-v, --crtc-trace` - Trace CRTC VRAM writes and register changes

## Disk Formats

| Format | Size | Tracks | Sides | Sectors/Track | Bytes/Sector |
|--------|------|--------|-------|---------------|--------------|
| SSDD   | 200KB | 40 | 1 | 10 | 512 |
| DSDD   | 400KB | 40 | 2 | 10 (0-9 side0, 10-19 side1) | 512 |

## Configuration

The emulator is configured via `izkaypro.toml` in the project root. Edit this file to select different Kaypro models and disk images.

### Configuration File (izkaypro.toml)

```toml
# Kaypro Model Presets:
#   kaypro_ii   - Kaypro II (SSDD, 81-149c ROM, memory-mapped video)
#   kaypro4_83  - Kaypro 4/83 (DSDD, 81-232 ROM, memory-mapped video)
#   kaypro4_84  - Kaypro 2X/4/84 (DSDD, 81-292a ROM, SY6545 CRTC)
#   turbo_rom   - Kaypro 4/84 with TurboROM 3.4 (DSDD, SY6545 CRTC)
#   custom      - Use custom settings below

model = "kaypro4_84"

# Custom settings (only used when model = "custom")
# rom_file = "roms/my_custom.rom"
# video_mode = "sy6545"       # "memory_mapped" or "sy6545"
# disk_format = "dsdd"        # "ssdd" or "dsdd"

# Disk images (optional, override model defaults)
# disk_a = "disks/my_boot_disk.img"
# disk_b = "disks/my_data_disk.img"
```

### Model Configurations

| Model | ROM | Disk Format | Video Mode |
|-------|-----|-------------|------------|
| kaypro_ii | 81-149c.rom | SSDD | Memory-mapped |
| kaypro4_83 | 81-232.rom | DSDD | Memory-mapped |
| kaypro4_84 | 81-292a.rom | DSDD | SY6545 CRTC |
| turbo_rom | trom34.rom | DSDD | SY6545 CRTC |

Command-line disk arguments override config file settings.

## Key Files

- `src/kaypro_machine.rs` - Machine emulation, port handling, memory banking, ROM selection
- `src/sy6545.rs` - SY6545 CRTC emulation for Kaypro 2X/4/84
- `src/floppy_controller.rs` - WD1793 FDC emulation, disk selection
- `src/media.rs` - Disk image handling and sector mapping
- `src/keyboard_unix.rs` - Terminal keyboard handling, function keys, escape sequences
- `src/screen.rs` - Terminal display, help overlay, attribute rendering

## System Bits Ports

### Port 0x1C (Kaypro II, 4/83 - MemoryMapped mode)

In MemoryMapped video mode, port 0x1C controls system bits:
- Bit 7: BANK (0=RAM, 1=ROM)
- Bit 6: Motors off
- Bit 5: Single density
- Bit 4: Centronics strobe
- Bit 3: Centronics ready
- Bit 2: Side 2
- Bits 1-0: Drive select (A=01, B=10)

### Port 0x14 (Kaypro 4-84 - Sy6545Crtc mode)

- Bit 7: BANK (0=RAM, 1=ROM/Video)
- Bit 6: Character set
- Bit 5: Density (0=double, 1=single)
- Bit 4: Motor (0=off, 1=on)
- Bit 3: Centronics strobe
- Bit 2: Side select (1=side0, 0=side1) - **inverted from port 0x1C**
- Bits 1-0: Drive select (A=10, B=01 for 81-292a ROM)

## SY6545 CRTC Emulation (Kaypro 2X/4/84)

The 81-292a ROM uses the SY6545 CRTC for video instead of memory-mapped VRAM.

### Memory Mapping (CRITICAL)

- In SY6545 CRTC mode, VRAM is accessed ONLY through I/O ports, NOT memory-mapped
- Memory addresses 0x3000-0x3FFF are regular RAM (used by programs like Zork)
- Memory-mapped VRAM at 0x3000-0x3FFF is only used in MemoryMapped video mode (Kaypro II, 4/83)
- This is why 63K CP/M works - the full 64K RAM is available except for ROM/BIOS area

### I/O Ports

- Port 0x1C (VIDCTL): Register select (write) / Status register (read)
- Port 0x1D (VIDDAT): Register data read/write
- Port 0x1F (VIDMEM): Video RAM data read/write

### Status Register (read from 0x1C)

- Bit 7 (SR7): UR - Update Ready (1 = ready for next update)
- Bit 5 (SR5): VRT - Vertical Retrace (1 = in vertical retrace)

### Video RAM Layout (4KB total, two 6116 SRAMs)

- Character RAM: 0x000-0x7FF (2KB) - displayed characters, initialized to 0x20 (space)
- Attribute RAM: 0x800-0xFFF (2KB) - display attributes, initialized to 0x00

### Attribute RAM Bit Definitions

- Bit 0: Reverse video (0=normal, 1=reverse)
- Bit 1: Half intensity (0=normal, 1=dim)
- Bit 2: Blink (0=steady, 1=blink)
- Bit 3: Underline
- Bits 4-7: Unused

Attributes are set via ADM-3A escape sequences: `[esc]B0` (reverse on), `[esc]C0` (reverse off), etc.

### VRAM Access Protocol

The ROM writes to VRAM using transparent addressing:

1. `OUT 0x1C, 0x12` - Select R18 (Update Address High)
2. `OUT 0x1D, addr_hi` - Write high byte (auto-increments to R19)
3. `OUT 0x1D, addr_lo` - Write low byte (auto-increments to R18)
4. `OUT 0x1C, 0x1F` - Select R31 (triggers strobe)
5. `OUT 0x1F, data` - Write data byte to VRAM

Port 0x1F writes do NOT auto-increment addr_latch. The ROM sets R18/R19 explicitly before each access.

**IMPORTANT**: Port 0x1F writes are only accepted when R31 is selected (reg_index == 0x1F). This prevents stray VRAM corruption when the ROM writes to port 0x1F after accessing other registers (e.g., cursor registers R14/R15).

### Display Layout

- 80-byte linear rows starting from R12:R13 (start_addr)
- Hardware scrolling via R12:R13 changes
- VRAM wraps at 2KB: `addr = (start_addr + row * 80 + col) & 0x7FF`

### Cursor Rendering

- Cursor position: R14:R15 (cursor address)
- Cursor mode (R10 bits 6-5): 0=steady, 1=invisible, 2/3=blink
- Rendered using ANSI reverse video at cursor address

## FDC (WD1793) Notes

- READ SECTOR does not auto-increment sector register
- READ ADDRESS does not modify sector register
- READ ADDRESS returns head byte = 0 for both sides (Kaypro 4-84 format)
- Index pulse (status bit 1) is simulated when motor is on - required for Turbo ROM
- STEP (0x20-0x3F), STEP IN (0x40-0x5F) and STEP OUT (0x60-0x7F) commands are supported
- STEP command moves in the same direction as the last STEP IN or STEP OUT (tracked via step_direction field)

## ROM Shadowing

ROM content is copied to RAM at startup for ROMs (like Turbo ROM) that switch to RAM bank mode and expect to continue executing from RAM.

## Function Keys

- F1: Help overlay
- F2: Disk status display
- F4: Quit emulator
- F5: Select file for drive A
- F6: Select file for drive B
- F7: Save BIOS to file
- F8: Toggle CPU trace

ESC cancels disk selection prompts (F5/F6).

## Testing

```bash
# Test with default configuration (Kaypro 4-84, 81-292a ROM)
cargo run

# Test with specific disk images
cargo run -- disks/cpm22g-rom292a.img disks/Zork.img

# Run with CRTC tracing
cargo run -- -v
```
