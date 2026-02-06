# Kaypro emulator on the terminal

## What is this?

This is a Kaypro emulator that runs in a terminal window on Linux and OSX (no Windows support at this time). It supports multiple Kaypro models and can boot and use disk images.  For best display results, set your terminal window to 86 x 27.

Uses the [iz80](https://github.com/ivanizag/iz80) library. Made with Rust.

## What is/was a Kaypro II computer?

The Kaypro II computer was a luggable computer from 1982 capable of running CP/M 2.2. It was considered "a rugged, functional and practical computer system marketed at a reasonable price." (From [Wikipedia](https://en.wikipedia.org/wiki/Kaypro))

It's a typical CP/M computer of the early 80s, built on a metal case with standard components, a 9" green monochrome CRT, a detachable keyboard and two disk drives. Main features:

- Zilog Z80 at 2.5 MHz
- 64 KB of main RAM
- 2 KB of ROM
- 2 KB of video RAM
- 80*24 text mode (no graphics capabilities)
- Two single or double side double density drives with 200kb/400kb capacity
- A serial port (not emulated by izkaypro)
- A parallel port (not emulated by izkaypro)

## Supported Models
This version of the emulator expands support to the Kaypro 4/83, 2X/4-84, and TurboROM-enabled Kaypro 4-84s.

| Model | ROM | Disk Format | Video Mode |
|-------|-----|-------------|------------|
| Kaypro II | 81-149c | SSDD (200KB) | Memory-mapped |
| Kaypro 4/83 | 81-232 | DSDD (400KB) | Memory-mapped |
| Kaypro 2X/4/84 | 81-292a | DSDD (400KB) | SY6545 CRTC |
| TurboROM 3.4 | trom34 | DSDD (400KB) | SY6545 CRTC |

## Configuration

Edit `izkaypro.toml` to select the Kaypro model by uncommenting the desired configuration:

```toml
# --- Kaypro II ---
# model = "kaypro_ii"

# --- Kaypro 4/83 ---
# model = "kaypro4_83"

# --- Kaypro 4/84 (default) ---
model = "kaypro4_84"

# --- TurboROM 3.4 ---
# model = "turbo_rom"
```

Optional: override default disk images by adding:
```toml
disk_a = "disks/my_boot_disk.img"
disk_b = "disks/my_data_disk.img"
```

## Usage examples

izkaypro does not require installation, you just need the executable. It has the ROM embedded as well as the boot CP/M disk and a blank disk. You can provide additional disk images as separate files.

### Usage with no arguments
Run the executable on a terminal and type the CP/M commands (you can try DIR and changing drives with B:). Press F4 to exit back to the host shell prompt.

Emulation of the Kaypro 4-84 computer

/===================================Kaypro 4-84=====================================\\
||                                                                                  ||
|| KAYPRO 63K CP/M Version 2.2G                                                     ||
||                                                                                  ||
|| A0>dir                                                                           ||
|| A: ASM      COM : CONFIG   COM : COPY     COM : D        COM                     ||
|| A: DDT      COM : DISK7    COM : LOAD     COM : MAKE     COM                     ||
|| A: MFDISK   COM : MOVCPM   COM : PIP      COM : STAT     COM                     ||
|| A: SUBMIT   COM : SYSGEN   COM : TERM     COM : VERIFY   COM                     ||
|| A: VERIFY   DOC : VIEW     COM : XSUB     COM : BAUDM    COM                     ||
|| A: BAUDP    COM : CLS      COM : DUMP     COM : ED       COM                     ||
|| A: KAYPRO   DOC : ST       COM : STD      COM : COMPARE  COM                     ||
|| A: DSKPRAM  COM : DU-V78   COM : EDFILE   COM : EDIT     COM                     ||
|| A: FBAD57   COM : PROBE    COM : RAMMAP   COM : UNERA    COM                     ||
|| A: LASM     COM : LSWEEP   COM : MLOAD    COM : UNCR     COM                     ||
|| A: MBASIC   COM : OBASIC   COM : KAYCLK   COM : COPYSS   COM                     ||
|| A: ACCESS   COM                                                                  ||
|| A0>                                                                              ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
\\================================================ F1 for help ==== F4 to exit =====//
```
### Usage with external images
You can provide up two disk images as binary files to use as A: and B: drives. If only an image is provided, it will be the A: disk, B: will be a blank disk.

If using the Kaypro II configuration, the images have to be raw binary images of single sided disks. The size must be 204800 bytes. See [disk images](doc/disk_images.md).

If using the Kaypro IV, 4-84, or 4-84 TurboROM configurations, images can be wither SDDD or DSDD disk images.  Sample images of both sides are provided in the disks directory.  If you swap disks, make sure to enter Ctrl-C to warm boot and re-load the new disk so that the BIOS will properly detect SSDD or DSDD format.

```
casa@servidor:~/$ ./izkaypro disks/cpmish.img disks/WordStar33.img 
B: disks/WordStar33.img
Kaypro https://github.com/ivanizag/izkaypro
Emulation of the Kaypro II computer

//==================================================================================\\
||                                                                                  ||
|| CP/Mish 2.2r0 for Kaypro II                                                      ||
||                                                                                  ||
|| A>dir                                                                            ||
|| COPY    .COM  |  DUMP    .COM  |  ASM     .COM  |  STAT    .COM                  ||
|| BBCBASIC.COM  |  SUBMIT  .COM  |  QE      .COM                                   ||
|| A>dir b:                                                                         ||
|| WS      .COM  |  WSOVLY1 .OVR  |  WSMSGS  .OVR  |  WS      .INS                  ||
|| WINSTALL.COM  |  PRINT   .TST                                                    ||
|| A>stat                                                                           ||
|| A: R/W, space: 135/195kB                                                         ||
|| B: R/W, space: 27/195kB                                                          ||
||                                                                                  ||
|| A>bbcbasic                                                                       ||
|| BBC BASIC (Z80) Version 3.00+1                                                   ||
|| (C) Copyright R.T.Russell 1987                                                   ||
|| >PRINT "Hi!"                                                                     ||
|| Hi!                                                                              ||
|| >*BYE                                                                            ||
||                                                                                  ||
|| A>_                                                                              ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
\\================================================= F1 for help ==== F4 to exit ====//
```
### Online help
Press F1 to get additional help:

//==================================================================================\\
||                                                                                  ||
|| KAYPRO II 64k CP/M vers 2.2                                                      ||
||                                                                                  ||
|| A>_                                                                              ||
||        +----------------------------------------------------------------+        ||
||        |  izkaypro: Kaypro II emulator for console terminals            |        ||
||        |----------------------------------------------------------------|        ||
||        |  F1: Show/hide help           | Host keys to Kaypro keys:      |        ||
||        |  F2: Show/hide disk status    |  Delete to DEL                 |        ||
||        |  F4: Quit the emulator        |  Insert to LINEFEED            |        ||
||        |  F5: Select file for drive A: |                                |        ||
||        |  F6: Select file for drive B: |                                |        ||
||        |  F7: Save BIOS to file        |                                |        ||
||        |  F8: Toggle CPU trace         |                                |        ||
||        |  F9: Set CPU speed (MHz)      |                                |        ||
||        +----------------------------------------------------------------+        ||
||        |  Loaded images:                                                |        ||
||        |  A: CPM/2.2 embedded (transient)                               |        ||
||        |  B: Blank disk embedded (transient)                            |        ||
||        +----------------------------------------------------------------+        ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
||                                                                                  ||
\\================================================= F1 for help ==== F4 to exit ====//

## Build from source

To build from source, install the latest Rust compiler, clone the repo and run `cargo rust --release`. To build and run directly execute `cargo run`.

## Command line usage
```
USAGE:
    izkaypro [FLAGS] [ARGS]

FLAGS:
    -b, --bdos-trace     Traces calls to the CP/M BDOS entrypoints
    -c, --cpu-trace      Traces CPU instructions execuions
    -f, --fdc-trace      Traces access to the floppy disk controller
    -h, --help           Prints help information
    -i, --io-trace       Traces ports IN and OUT
    -r, --rom-trace      Traces calls to the ROM entrypoints
    -s, --system-bits    Traces changes to the system bits values
    -V, --version        Prints version information

ARGS:
    <DISKA>    Disk A: image file. Empty or $ to load CP/M
    <DISKB>    Disk B: image file. Default is a blank disk
```

## Resources

- [ROM disassembled and commented](https://github.com/ivanizag/kaypro-disassembly)
- [Kaypro manuals in bitsavers](http://bitsavers.informatik.uni-stuttgart.de/pdf/kaypro/)
- [Disks from retroarchive](http://www.retroarchive.org/maslin/disks/kaypro/)
- [ImageDisk and system images](http://dunfield.classiccmp.org/img/index.htm)
