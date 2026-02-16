# HDFMT.COM Disassembly Analysis

## Source
Disassembly of `playground/hdfmt.com` (Kaypro 10/12 Winchester Disk Formatter v1.02)
using `z80dasm -a -l -o 0x100`. Full disassembly in `0x100` (19231 lines).

## HD Controller I/O Subroutines (0x3267–0x3541)

All HD controller interaction is in this region. The program directly accesses
WD1002-05 ports 0x80-0x87 (bypasses CP/M BIOS).

### Port Map
```
Port 0x80 = Data register (read/write)
Port 0x81 = Error register (read) / Write Precomp (write)
Port 0x82 = Sector Count
Port 0x83 = Sector Number
Port 0x84 = Cylinder Low
Port 0x85 = Cylinder High
Port 0x86 = SDH (Size/Drive/Head)
Port 0x87 = Status (read) / Command (write)
```

---

### sub_34f7 — SET SDH REGISTER
```asm
; Input: A = head number, D = SDH base (0xA0 or 0x80), E = extra OR bits
; SDH = (head << 3) | D | E
; Then waits for READY
034f7:  add a,a         ; head * 2
        add a,a         ; head * 4
        add a,a         ; head * 8  => head << 3
        or d            ; OR with base (0xA0 for 512-byte mode, 0x80 for 256-byte)
        or e            ; OR with extra bits (usually 0)
        out (086h),a    ; write SDH register
        call 034c5h     ; wait_ready: poll status for READY bit
        ret
```

**CRITICAL FINDING**: Head is placed in bits 4:3 of SDH via `head << 3`.
Per the WD1002-05 manual, bits 4:3 are "Drive Select" (LUN) and bits 2:0
are "Head Select". HDFMT uses the drive select field for the head because
the Kaypro 10 has only one physical drive.

SDH values produced:
- Head 0: (0<<3) | 0xA0 = **0xA0** — bits 4:3=00, bits 2:0=000
- Head 1: (1<<3) | 0xA0 = **0xA8** — bits 4:3=01, bits 2:0=000
- Head 2: (2<<3) | 0xA0 = **0xB0** — bits 4:3=10, bits 2:0=000
- Head 3: (3<<3) | 0xA0 = **0xB8** — bits 4:3=11, bits 2:0=000

Our emulator's `get_lun()` extracts bits 4:3, so it sees LUN 0,1,2,3.
Our emulator's `get_head()` extracts bits 2:0, so it always sees head 0.
**This means our CHS calculation is wrong for heads 1-3, AND heads 2-3
are rejected by the LUN>1 check.**

### sub_34ad — WAIT NOT BUSY
```asm
; Poll status register until BUSY bit (bit 7) clears.
; Uses nested countdown loop for timeout.
; Returns Z=timeout, NZ=success (not busy)
034ad:  push hl
        push de
        ld h,007h       ; outer loop = 7
        ld de,00000h    ; inner loop = 65536
034b4:  in a,(087h)     ; read status
        cpl             ; invert (so BUSY=0 becomes bit=1)
        bit 7,a         ; test inverted BUSY
        jp nz,034c2h    ; if was NOT busy, done
        call 0353ch     ; decrement timeout counter
        jp nz,034b4h    ; keep polling
034c2:  pop de
        pop hl
        ret             ; Z=timed out, NZ=not busy
```

### sub_034c5 — WAIT READY
```asm
; First waits NOT BUSY, then polls for READY (bit 6)
034c5:  call 034adh     ; wait not busy
        ret z           ; return if timed out
        push hl
        push de
        ld h,005h
        ld de,00000h
034d0:  in a,(087h)     ; read status
        bit 6,a         ; test READY
        jp nz,034ddh    ; if READY, done
        call 0353ch     ; decrement timeout
        jp nz,034d0h
034dd:  pop de
        pop hl
        ret
```

### sub_034e0 — WAIT SEEK DONE
```asm
; Polls for SEEK DONE (bit 4) in status
034e0:  push hl
        push de
        ld h,00ah
        ld de,00000h
034e7:  in a,(087h)
        bit 4,a         ; test SEEK_DONE
        jp nz,034f4h
        call 0353ch     ; timeout
        jp nz,034e7h
034f4:  pop de
        pop hl
        ret
```

### sub_03496 — WAIT DRQ
```asm
; Polls for DRQ (bit 3) in status
03496:  push hl
        push de
        ld h,003h
        ld de,00000h
0349d:  in a,(087h)
        bit 3,a         ; test DRQ
        jp nz,034aah    ; if DRQ, done
        call 0353ch     ; timeout
        jp nz,0349dh
034aa:  pop de
        pop hl
        ret
```

---

### SASI RESET + INIT SEQUENCE (0x3267)
```asm
; Called at start of formatting
03267:  ld h,002h           ; delay
        ld de,00000h
        call 0353ch
        jr nz,$-3           ; delay loop

; Toggle port 0x14 bit 1 for SASI reset
        di
        ld a,0fdh
        ld (0fff7h),a       ; store for later reference
03277:  in a,(014h)
        set 1,a             ; set bit 1 HIGH
        out (014h),a
        push af
        ld h,001h
        ld de,00000h
        call 0353ch         ; delay
        jr nz,$-3
        pop af
        res 1,a             ; clear bit 1 LOW — triggers SASI reset
        out (014h),a
        ei

; Wait for controller to become not busy (reset diagnostics complete)
        call 034adh         ; wait_not_busy
        ld a,001h
        jp z,03511h         ; timeout → error code 1

; Check diagnostic result
        in a,(081h)         ; read error register
        cp 001h             ; expect 0x01 (DIAG_WD2797 = no floppy chip)
        ld a,002h
        jp nz,03511h        ; wrong diag → error code 2

; Select drive: head=param, d=0xA0 (512-byte sectors), e=0
        ld a,(ix+002h)      ; head number from parameter
        ld d,0a0h           ; SDH base: CRC mode, 512-byte sectors
        ld e,000h
        call 034f7h         ; set_sdh: SDH = (head<<3) | 0xA0
        ld a,003h
        jp z,03511h         ; READY timeout → error code 3

; Issue RESTORE command (0x10)
        ld a,010h
        out (087h),a        ; RESTORE command
        call 034e0h         ; wait_seek_done
        ld a,01ch
        jp z,03511h         ; timeout → error code 0x1C

; Wait not busy after RESTORE
        call 034adh
        ld a,004h
        jp z,03511h         ; timeout → error code 4

; Check for errors after RESTORE
        in a,(087h)
        bit 0,a             ; ERROR bit
        ld a,005h
        jp nz,03511h        ; error → error code 5

; Success
        jp 03532h           ; return HL=0 (success)
```

**NOTE**: The SASI reset sequence does: SET bit 1, delay, CLEAR bit 1.
Port 0x14 active-low means clearing bit 1 asserts the reset signal through
the 7406 inverter. Our emulator detects the HIGH→LOW transition correctly.

---

### sub_32ce — FORMAT TRACK
```asm
; Parameters on stack frame (via ix):
;   ix+002: head
;   ix+004: cylinder low
;   ix+005: cylinder high
;   ix+006: starting sector number (used as E in SDH setup)
; Calls sub_336e (the core format/io dispatch)
032ce:  call 03502h         ; save registers, set up ix as frame pointer
        call 0351eh         ; ensure bit 1 of port 0x14 is clear (SASI active)

; Set SDH: head in (ix+2), base=0xA0, extra bits from (ix+6)
        ld a,(ix+002h)      ; head
        ld d,0a0h           ; SDH base
        ld e,(ix+006h)      ; extra bits (sector start?)
        call 034f7h         ; SDH = (head<<3) | 0xA0 | e
        ld a,006h
        jp z,03511h         ; READY timeout → error code 6

; Set up format parameters
        ld hl,0357fh        ; HL → interleave table (34 bytes at 0x357F)
        ld b,011h           ; B = 17 (sectors per track)
        ld c,002h           ; C = 2 (two OTIRs = 512 bytes total)
        jp 0336eh           ; → core format dispatch
```

### sub_336e — CORE FORMAT/IO DISPATCH
```asm
; Input: HL = data buffer, B = sector count, C = transfer mode
;        (C=2 → two 256-byte blocks, C=1 → one 256-byte block)
; SDH already set. Cylinder/sector registers set from ix params.
0336e:  call 034adh         ; wait not busy
        ld a,01bh
        jp z,03511h         ; timeout → error 0x1B

; Set up task file registers
        ld a,b
        out (082h),a        ; sector count = B (17 for format)
        ld a,(ix+004h)
        out (084h),a        ; cylinder low
        ld a,(ix+005h)
        out (085h),a        ; cylinder high
        ld a,0ffh
        out (081h),a        ; write precomp = 0xFF
        ld a,000h
        out (083h),a        ; sector number = 0

; Issue FORMAT TRACK command
        ld a,050h
        out (087h),a        ; FORMAT TRACK command (0x50)

; Wait for DRQ
        call 03496h         ; wait_drq
        ld a,00ch
        jp z,03511h         ; timeout → error 0x0C

; Send data via OTIR
        ld a,c              ; C = transfer mode
        ld b,000h           ; B = 0 → OTIR does 256 iterations
        ld c,080h           ; C = port 0x80 (data register)
        cp 002h             ; two blocks?
        jp nz,033a3h        ; skip first OTIR if only one block
        otir                ; first OTIR: 256 bytes from (HL) to port 0x80
033a3:  otir                ; second OTIR: 256 bytes from (HL) to port 0x80

; Wait for command completion
        call 034adh         ; wait not busy
        ld a,00dh
        jp z,03511h         ; timeout → error 0x0D

; Check for errors
        in a,(087h)
        bit 0,a             ; ERROR bit
        ld a,00eh
        jp nz,03511h        ; error → error code 0x0E

; Success
        jp 03532h           ; return HL=0 (success)
```

**CRITICAL FINDING**: FORMAT TRACK sends **512 bytes** via two OTIRs of 256 bytes each.
The data comes from the buffer at HL (0x357F = interleave table, only 34 bytes of
meaningful data, but 512 bytes total are sent from sequential memory).

Our emulator sets `data_length = 34` (17 sectors × 2 bytes), so it triggers
`process_data()` after only 34 bytes. The remaining 478 bytes hit the overflow
path in `put_data()`, which calls `set_done()` and clears DRQ. But OTIR keeps
writing to port 0x80 regardless — those writes just go to `put_data()` which
sees `data_ix >= data_length` and calls `set_done()` each time.

The real problem: after FORMAT completes (with our 34-byte length), the track
is marked as formatted BUT the status may not be what HDFMT expects. Also,
the early `set_done()` clears BUSY before all 512 bytes are sent.

---

### sub_32ee — READ SECTOR(S)
```asm
032ee:  call 03502h         ; frame setup
        call 0351eh         ; ensure SASI active
        ld d,0a0h           ; SDH base
        call 033b9h         ; set SDH (head from ix+4) + set cylinder/sector regs
        ld a,007h
        jp z,03511h         ; error code 7
        ld c,002h           ; C = 2 (two INIRs = 512 bytes)
        ld b,(ix+00ch)      ; B = sector count
        jp 033dah           ; → read_sector_loop
```

### sub_033b9 — SET SDH + CYLINDER + SECTOR REGISTERS
```asm
033b9:  ld a,(ix+004h)      ; head
        ld e,(ix+008h)      ; extra SDH bits
        call 034f7h         ; SDH = (head<<3) | D | E
        ret z               ; timeout
        ld a,0ffh
        out (081h),a        ; write precomp = 0xFF
        ld a,(ix+00ah)      ; sector number
        out (083h),a        ; sector number register
        ld a,(ix+006h)      ; cylinder low
        out (084h),a        ; cylinder low register
        ld a,(ix+007h)      ; cylinder high
        out (085h),a        ; cylinder high register
        ld a,001h
        and a               ; clear Z flag (success)
        ret
```

**NOTE**: Here head comes from `(ix+004h)`, not `(ix+002h)` — different
stack frame layout than the init/format functions. But the same sub_34f7h
is used: `SDH = (head << 3) | D`.

### sub_033da — READ SECTOR LOOP
```asm
; B = number of sectors to read, C = transfer mode, HL = buffer
033da:  ld l,(ix+002h)      ; buffer low
        ld h,(ix+003h)      ; buffer high
033e0:  call 034adh         ; wait not busy
        ld a,00fh
        jp z,03511h         ; timeout
        call 034c5h         ; wait ready
        ld a,010h
        jp z,03511h         ; timeout

; Set sector count = 1 (reads one sector at a time)
        ld a,001h
        out (082h),a        ; sector count = 1

; Issue READ command (0x20)
        ld a,020h
        out (087h),a        ; READ SECTOR command

; Wait not busy, then wait DRQ
        call 034adh
        ld a,011h
        jp z,03511h
        call 03496h         ; wait DRQ
        ld a,012h
        jp z,03511h

; Read data via INIR
        ld a,c              ; C = transfer mode
        push bc
        ld b,000h           ; 256 iterations
        ld c,080h           ; port 0x80
        cp 002h
        jp nz,03415h        ; skip first INIR if mode != 2
        inir                ; first INIR: 256 bytes from port 0x80
03415:  inir                ; second INIR: 256 bytes from port 0x80
        pop bc

; Check completion
        call 034adh         ; wait not busy
        ld a,019h
        jp z,03511h

; Check status for ERROR
        in a,(087h)
        bit 0,a             ; ERROR bit
        ld a,013h
        jp nz,03511h        ; error code 0x13

; Check error register for ABORTED (bit 2)
        in a,(081h)
        bit 2,a             ; ERR_ABORTED
        ld a,014h
        jp nz,03511h        ; error code 0x14

; Advance to next sector
        in a,(083h)         ; read current sector number
        inc a
        out (083h),a        ; sector number + 1
        djnz 033e0h         ; loop for next sector
        jp 03532h           ; done — return HL=0
```

### sub_3306 — WRITE SECTOR(S)
```asm
03306:  call 03502h         ; frame setup
        call 0351eh         ; ensure SASI active
        ld d,0a0h           ; SDH base
        call 033b9h         ; set SDH + cylinder + sector regs
        ld a,008h
        jp z,03511h
        ld c,002h           ; C = 2 (two OTIRs = 512 bytes)
        ld b,(ix+00ch)      ; B = sector count
        jp 0343ch           ; → write_sector_loop
```

### sub_0343c — WRITE SECTOR LOOP
```asm
0343c:  ld l,(ix+002h)      ; buffer low
        ld h,(ix+003h)      ; buffer high
03442:  call 034adh         ; wait not busy
        ld a,015h
        jp z,03511h
        call 034c5h         ; wait ready
        ld a,016h
        jp z,03511h

; Sector count = 1
        ld a,001h
        out (082h),a

; Issue WRITE command (0x30)
        ld a,030h
        out (087h),a        ; WRITE SECTOR command

; Wait for DRQ
        call 03496h         ; wait DRQ
        ld a,017h
        jp z,03511h

; Send data via OTIR
        ld a,c
        push bc
        ld b,000h           ; 256 iterations
        ld c,080h           ; port 0x80
        cp 002h
        jp nz,0346fh
        otir                ; first OTIR: 256 bytes
0346f:  otir                ; second OTIR: 256 bytes
        pop bc

; Check completion
        call 034adh         ; wait not busy
        ld a,01ah
        jp z,03511h

; Check errors
        in a,(087h)
        bit 0,a             ; ERROR
        ld a,017h
        jp nz,03511h
        in a,(081h)
        bit 2,a             ; ERR_ABORTED
        ld a,018h
        jp nz,03511h

; Advance sector
        in a,(083h)
        inc a
        out (083h),a
        djnz 03442h         ; loop
        jp 03532h           ; success
```

### sub_03511 — ERROR RETURN
```asm
; Returns error: H = error code (set before call), L = error register
03511:  ld h,a              ; error code → H
        in a,(081h)         ; read error register
        ld l,a              ; error register → L
        pop iy
        pop ix
        pop bc
        ld a,0ffh
        or a                ; set NZ (failure)
        ret                 ; returns HL = error code:error_reg, Z=0
```

### sub_03532 — SUCCESS RETURN
```asm
03532:  ld hl,00000h        ; HL = 0 (success)
        pop iy
        pop ix
        pop bc
        xor a               ; set Z (success)
        ret                 ; returns HL = 0, Z=1
```

---

## Interleave Table at 0x357F (34 bytes)

Used for FORMAT TRACK command. 17 entries of 2 bytes each:
```
Offset  Bytes  Meaning
0x357F: 00 00  bad_block=0, sector=0
0x3581: 00 07  bad_block=0, sector=7
0x3583: 00 0E  bad_block=0, sector=14
0x3585: 00 04  bad_block=0, sector=4
0x3587: 00 0B  bad_block=0, sector=11
0x3589: 00 01  bad_block=0, sector=1
0x358B: 00 08  bad_block=0, sector=8
0x358D: 00 0F  bad_block=0, sector=15
0x358F: 00 05  bad_block=0, sector=5
0x3591: 00 0C  bad_block=0, sector=12
0x3593: 00 02  bad_block=0, sector=2
0x3595: 00 09  bad_block=0, sector=9
0x3597: 00 10  bad_block=0, sector=16
0x3599: 00 06  bad_block=0, sector=6
0x359B: 00 0D  bad_block=0, sector=13
0x359D: 00 03  bad_block=0, sector=3
0x359F: 00 0A  bad_block=0, sector=10
```
Interleave pattern: 0,7,14,4,11,1,8,15,5,12,2,9,16,6,13,3,10 (3:1 interleave)

After 34 bytes, the remaining 478 bytes sent by OTIR come from memory
following the table — essentially garbage, but the controller must absorb them.

---

## HDFMT Operational Sequence

1. **SASI Reset**: Toggle port 0x14 bit 1 (set HIGH, delay, clear LOW)
2. **Check diagnostics**: Wait not-busy, read error reg, expect 0x01 (DIAG_WD2797)
3. **Select drive**: SDH = (head<<3) | 0xA0, wait READY
4. **RESTORE**: Command 0x10, wait SEEK_DONE
5. **For each track** (cylinder 0-305, heads 0-3):
   a. Set SDH = (head<<3) | 0xA0 | sector_start
   b. Set cylinder/sector registers
   c. Issue FORMAT TRACK (0x50)
   d. Wait DRQ
   e. Send 512 bytes via two OTIRs (interleave table + padding)
   f. Wait not-busy, check ERROR bit
6. **Write defect map**: Write sector(s) to track 0 via WRITE SECTOR (0x30)
7. **Verify**: Read back and check

---

## Bugs Found and Fixed

### Bug 1: SDH Head Extraction (FIXED)
**Initial (wrong) analysis**: We thought the Kaypro 10 puts head in bits 4:3
(repurposing the LUN field). This was based on a superficial reading of
sub_034f7h (`head<<3 | 0xA0 | extra`) and sub_19a7h in the ROM.

**Correct analysis**: Both the ROM and HDFMT use STANDARD WD1002-05 encoding:
- Bits 7:5 = sector size (101 = 512 bytes)
- Bits 4:3 = LUN/drive select (always 1 for the Kaypro 10 HD)
- Bits 2:0 = head number (0-3)

The `head<<3` in sub_034f7h and sub_19a7h is actually shifting the UNIT
number (always 1) into bits 4:3. The actual head is passed as the "extra"
parameter and goes into bits 2:0. Evidence:
- HDFMT: `(ix+002h)` = `(00109h)` = 1 (unit), `(ix+006h)` = head (0-3)
- ROM: B = `(0FD84h)` = drive code with unit in bits 1:0, `(ix+004h)` = head
- Java: `getHead() = sdh & 0x07` (bits 2:0), `getLUN() = (sdh>>3) & 0x03`

SDH values: head 0 = 0xA8, head 1 = 0xA9, head 2 = 0xAA, head 3 = 0xAB.

**Fix**: `get_head()` changed to `self.sdh & 0x07` (bits 2:0), matching Java.

### Bug 2: FORMAT TRACK data_length (FIXED)
We set `data_length = 34` (17×2). HDFMT sends 512 bytes. The controller must
absorb all 512 bytes before completing. Java leaves dataLength at 512.

**Fix**: Set `data_length = SECTOR_SIZE` (512) for FORMAT TRACK.

### Bug 3: FORMAT TRACK processing (FIXED)
Our 0xE5 fill was unnecessary. Java just sets `formatted = true` and calls
`setDone()`. Simplified to match Java — mark track formatted, don't fill data.
Keep per-track tracking (better than Java's global boolean).
```
