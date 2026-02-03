; EMUTEST.COM - Emulator ROM and RAM diagnostics
; Based on diag4.mac from Non-Linear Systems, Inc. (1983)
; Implements proper ROM bank switching by relocating code to 0x8000
; 
; Test 3 (Video RAM) uses SY6545 CRTC transparent addressing:
;   Port 0x1C: Register select (R18=addr_hi, R19=addr_lo)
;   Port 0x1D: Register data
;   Port 0x1F: VRAM data read/write + strobe control
;   Wait for port 0x1C bit 7 = 1 (Update Ready) after strobe

; CP/M BDOS calls
bdos:           equ     5
conout:         equ     2
prtstr:         equ     9

; Kaypro 4-84 ports
bitport:        equ     014h

; SY6545 CRTC ports and commands (from diag4.mac univ=true)
crtc_reg:       equ     01Ch    ; CRTC register select
crtc_data:      equ     01Dh    ; CRTC register data
crtc_vram:      equ     01Fh    ; CRTC VRAM data / strobe
strcmd:         equ     01Fh    ; Strobe command value

        org     0100h

; ============================================================================
; Main program
; ============================================================================
start:
        ld      sp, stack

        ; Print banner
        ld      de, msg_banner
        call    print

        ; Test 1: ROM checksum
        call    rom_test

        ; Test 2: RAM tests
        call    ram_test

        ; Test 3: Video RAM test (SY6545 CRTC)
        call    vram_test

        ; Print completion message
        ld      de, msg_done
        call    print

        ; Return to CP/M
        rst     0

; ============================================================================
; ROM Checksum Test
; Copies test code to 0x8000, executes there to read ROM while banked in
; ============================================================================
rom_test:
        ld      de, msg_rom
        call    print

        ; Copy ROM test routine to 0x8000
        ld      hl, rom_code_start
        ld      de, 08000h
        ld      bc, rom_code_end - rom_code_start
        ldir

        ; Jump to relocated code at 0x8000
        ; It will calculate checksum and return here via rom_return
        jp      08000h

rom_return:
        ; HL now contains the checksum (returned from relocated code)
        push    hl

        ; Print checksum value
        ld      de, msg_chksum
        call    print
        pop     hl
        call    print_hex16

        ld      de, msg_crlf
        call    print
        ret

; ============================================================================
; ROM test code - gets copied to 0x8000 and executed there
; This code must be position-independent or use absolute addresses at 0x8000
; ============================================================================
rom_code_start:
        ; This code runs at 0x8000
        
        ; Save current bank state
        in      a, (bitport)
        push    af

        ; Switch to ROM bank (set bit 7)
        or      080h
        out     (bitport), a

        ; Calculate checksum of first 4KB (0x0000-0x0FFF)
        ; HL = checksum accumulator, DE = address pointer
        ld      hl, 0
        ld      de, 0
        ld      bc, 01000h          ; 4KB = 0x1000 bytes

rom_calc_loop:
        ld      a, (de)             ; Read ROM byte
        add     a, l                ; Add to low byte of checksum
        ld      l, a
        jr      nc, rom_no_carry
        inc     h                   ; Carry to high byte
rom_no_carry:
        inc     de
        dec     bc
        ld      a, b
        or      c
        jr      nz, rom_calc_loop

        ; Save checksum in IX (safe across bank switch)
        push    hl
        pop     ix

        ; Switch back to RAM bank (clear bit 7)
        pop     af                  ; Get saved port value
        res     7, a
        out     (bitport), a

        ; Restore checksum to HL from IX
        push    ix
        pop     hl

        ; Jump back to main code (absolute address)
        jp      rom_return

rom_code_end:

; ============================================================================
; RAM Tests
; ============================================================================
ram_test:
        ; Test region 0x4000-0x7FFF
        ld      de, msg_ram1
        call    print

        ld      hl, 04000h
        ld      de, 07FFFh
        call    sliding_data
        jr      nz, ram_fail1

        ld      hl, 04000h
        ld      de, 07FFFh
        call    address_data
        jr      nz, ram_fail1

        ld      de, msg_pass
        call    print
        jr      ram_test2

ram_fail1:
        ld      de, msg_fail
        call    print

ram_test2:
        ; Test region 0x8000-0xBFFF
        ld      de, msg_ram2
        call    print

        ld      hl, 08000h
        ld      de, 0BFFFh
        call    sliding_data
        jr      nz, ram_fail2

        ld      hl, 08000h
        ld      de, 0BFFFh
        call    address_data
        jr      nz, ram_fail2

        ld      de, msg_pass
        call    print
        ret

ram_fail2:
        ld      de, msg_fail
        call    print
        ret

; ============================================================================
; Sliding Data Test
; Input: HL = start, DE = end
; Output: Z=pass, NZ=fail
; ============================================================================
sliding_data:
        push    hl
        push    de

        ld      b, 1            ; Initial pattern
sd_outer:
        ld      c, 8            ; 8 bit positions

sd_bit:
        pop     de
        pop     hl
        push    hl
        push    de

sd_write:
        ld      (hl), b
        ld      a, h
        cp      d
        jr      nz, sd_winc
        ld      a, l
        cp      e
        jr      z, sd_verify
sd_winc:
        inc     hl
        jr      sd_write

sd_verify:
        pop     de
        pop     hl
        push    hl
        push    de

sd_read:
        ld      a, (hl)
        cp      b
        jr      nz, sd_fail
        ld      a, h
        cp      d
        jr      nz, sd_rinc
        ld      a, l
        cp      e
        jr      z, sd_next
sd_rinc:
        inc     hl
        jr      sd_read

sd_next:
        rlc     b
        dec     c
        jr      nz, sd_bit

        ld      a, b
        cp      1
        jr      nz, sd_done
        ld      b, 0FEh
        jr      sd_outer

sd_done:
        pop     de
        pop     hl
        xor     a
        ret

sd_fail:
        pop     de
        pop     hl
        ld      a, 1
        or      a
        ret

; ============================================================================
; Address Data Test
; Input: HL = start, DE = end
; Output: Z=pass, NZ=fail
; ============================================================================
address_data:
        push    hl
        push    de

        ; Write low bytes
        pop     de
        pop     hl
        push    hl
        push    de

ad_wlo:
        ld      (hl), l
        ld      a, h
        cp      d
        jr      nz, ad_wlinc
        ld      a, l
        cp      e
        jr      z, ad_vlo
ad_wlinc:
        inc     hl
        jr      ad_wlo

ad_vlo:
        ; Verify low bytes
        pop     de
        pop     hl
        push    hl
        push    de

ad_rlo:
        ld      a, (hl)
        cp      l
        jr      nz, ad_fail
        ld      a, h
        cp      d
        jr      nz, ad_rlinc
        ld      a, l
        cp      e
        jr      z, ad_whi
ad_rlinc:
        inc     hl
        jr      ad_rlo

ad_whi:
        ; Write high bytes
        pop     de
        pop     hl
        push    hl
        push    de

ad_wh:
        ld      (hl), h
        ld      a, h
        cp      d
        jr      nz, ad_whinc
        ld      a, l
        cp      e
        jr      z, ad_vhi
ad_whinc:
        inc     hl
        jr      ad_wh

ad_vhi:
        ; Verify high bytes
        pop     de
        pop     hl
        push    hl
        push    de

ad_rhi:
        ld      a, (hl)
        cp      h
        jr      nz, ad_fail
        ld      a, h
        cp      d
        jr      nz, ad_rhinc
        ld      a, l
        cp      e
        jr      z, ad_done
ad_rhinc:
        inc     hl
        jr      ad_rhi

ad_done:
        pop     de
        pop     hl
        xor     a
        ret

ad_fail:
        pop     de
        pop     hl
        ld      a, 1
        or      a
        ret

; ============================================================================
; Video RAM Test (SY6545 CRTC transparent addressing)
; Tests 2KB of VRAM at 0x0000-0x07FF via CRTC registers
; ============================================================================
vram_test:
        ld      de, msg_vram
        call    print

        ; Save current VRAM contents to backup buffer at 0x9000
        ld      hl, 0               ; VRAM address
        ld      de, 09000h          ; Backup buffer
        ld      bc, 0800h           ; 2KB
vram_save:
        push    bc
        push    de
        call    crtc_read           ; Read VRAM[HL] -> A
        pop     de
        ld      (de), a             ; Save to backup
        inc     de
        inc     hl
        pop     bc
        dec     bc
        ld      a, b
        or      c
        jr      nz, vram_save

        ; Perform sliding-data test on VRAM 0x0000-0x07FF
        ld      hl, 0
        ld      de, 07FFh
        call    vram_sliding
        jr      nz, vram_fail

        ; Perform address-data test on VRAM 0x0000-0x07FF
        ld      hl, 0
        ld      de, 07FFh
        call    vram_address
        jr      nz, vram_fail

        ; Restore VRAM contents from backup
        call    vram_restore

        ld      de, msg_pass
        call    print
        ret

vram_fail:
        ; Restore VRAM contents even on failure
        call    vram_restore

        ld      de, msg_fail
        call    print
        ret

vram_restore:
        ld      hl, 0               ; VRAM address
        ld      de, 09000h          ; Backup buffer
        ld      bc, 0800h           ; 2KB
vram_rest_loop:
        push    bc
        push    hl
        ld      a, (de)             ; Get from backup
        call    crtc_write          ; Write VRAM[HL] <- A
        pop     hl
        inc     hl
        inc     de
        pop     bc
        dec     bc
        ld      a, b
        or      c
        jr      nz, vram_rest_loop
        ret

; ============================================================================
; VRAM Sliding Data Test
; Input: HL = start VRAM addr, DE = end VRAM addr
; Output: Z=pass, NZ=fail
; ============================================================================
vram_sliding:
        push    hl
        push    de

        ld      b, 1                ; Initial pattern 0x01
vsd_outer:
        ld      c, 8                ; 8 bit positions

vsd_bit:
        pop     de
        pop     hl
        push    hl
        push    de

        ; Write pattern B to all VRAM locations
vsd_write:
        push    bc
        push    de
        push    hl
        ld      a, b                ; Pattern to write
        call    crtc_write          ; Write to VRAM[HL]
        pop     hl
        pop     de
        pop     bc
        ; Check if HL == DE (end)
        ld      a, h
        cp      d
        jr      nz, vsd_winc
        ld      a, l
        cp      e
        jr      z, vsd_verify
vsd_winc:
        inc     hl
        jr      vsd_write

vsd_verify:
        ; Verify pattern B in all VRAM locations
        pop     de
        pop     hl
        push    hl
        push    de

vsd_read:
        push    bc
        push    de
        push    hl
        call    crtc_read           ; Read VRAM[HL] -> A
        pop     hl
        pop     de
        pop     bc                  ; Restore pattern (B) and counter (C)
        cp      b                   ; Compare read value (A) with expected pattern (B)
        jr      nz, vsd_fail
        ; Check if HL == DE (end)
        ld      a, h
        cp      d
        jr      nz, vsd_rinc
        ld      a, l
        cp      e
        jr      z, vsd_next
vsd_rinc:
        inc     hl
        jr      vsd_read

vsd_next:
        rlc     b                   ; Rotate pattern left
        dec     c                   ; Decrement bit counter
        jr      nz, vsd_bit

        ; After 8 rotations of 0x01, switch to 0xFE
        ld      a, b
        cp      1
        jr      nz, vsd_done
        ld      b, 0FEh
        jr      vsd_outer

vsd_done:
        pop     de
        pop     hl
        xor     a                   ; Z=pass
        ret

vsd_fail:
        pop     de
        pop     hl
        ld      a, 1
        or      a                   ; NZ=fail
        ret

; ============================================================================
; VRAM Address Data Test
; Input: HL = start VRAM addr, DE = end VRAM addr
; Output: Z=pass, NZ=fail
; ============================================================================
vram_address:
        push    hl
        push    de

        ; Write low byte of address to each location
        pop     de
        pop     hl
        push    hl
        push    de

vad_wlo:
        push    de
        push    hl
        ld      a, l                ; Low byte of address
        call    crtc_write          ; Write to VRAM[HL]
        pop     hl
        pop     de
        ; Check if HL == DE
        ld      a, h
        cp      d
        jr      nz, vad_wlinc
        ld      a, l
        cp      e
        jr      z, vad_vlo
vad_wlinc:
        inc     hl
        jr      vad_wlo

vad_vlo:
        ; Verify low bytes
        pop     de
        pop     hl
        push    hl
        push    de

vad_rlo:
        push    de
        push    hl
        call    crtc_read           ; Read VRAM[HL] -> A
        pop     hl
        pop     de
        cp      l                   ; Compare with expected (low byte of addr)
        jr      nz, vad_fail
        ; Check if HL == DE
        ld      a, h
        cp      d
        jr      nz, vad_rlinc
        ld      a, l
        cp      e
        jr      z, vad_whi
vad_rlinc:
        inc     hl
        jr      vad_rlo

vad_whi:
        ; Write high byte of address to each location
        pop     de
        pop     hl
        push    hl
        push    de

vad_wh:
        push    de
        push    hl
        ld      a, h                ; High byte of address
        call    crtc_write          ; Write to VRAM[HL]
        pop     hl
        pop     de
        ; Check if HL == DE
        ld      a, h
        cp      d
        jr      nz, vad_whinc
        ld      a, l
        cp      e
        jr      z, vad_vhi
vad_whinc:
        inc     hl
        jr      vad_wh

vad_vhi:
        ; Verify high bytes
        pop     de
        pop     hl
        push    hl
        push    de

vad_rhi:
        push    de
        push    hl
        call    crtc_read           ; Read VRAM[HL] -> A
        pop     hl
        pop     de
        cp      h                   ; Compare with expected (high byte of addr)
        jr      nz, vad_fail
        ; Check if HL == DE
        ld      a, h
        cp      d
        jr      nz, vad_rhinc
        ld      a, l
        cp      e
        jr      z, vad_done
vad_rhinc:
        inc     hl
        jr      vad_rhi

vad_done:
        pop     de
        pop     hl
        xor     a                   ; Z=pass
        ret

vad_fail:
        pop     de
        pop     hl
        ld      a, 1
        or      a                   ; NZ=fail
        ret

; ============================================================================
; CRTC VRAM Read - Read byte from VRAM via SY6545 transparent addressing
; Input: HL = VRAM address (0x0000-0x07FF)
; Output: A = byte read
; Clobbers: BC
; 
; Protocol (from diag4.mac cr4/cr6):
;   1. OUT 0x1C, 0x12       (select R18 - Update Address High)
;   2. OUT 0x1D, H          (write high byte)
;   3. OUT 0x1C, 0x13       (select R19 - Update Address Low)  
;   4. OUT 0x1D, L          (write low byte)
;   5. OUT 0x1C, 0x1F       (strobe command)
;   6. Wait for IN 0x1C bit 7 = 1 (Update Ready)
;   7. IN 0x1F              (read VRAM data)
; ============================================================================
crtc_read:
        push    hl
        ; Mask address to 11 bits (0-0x7FF)
        ld      a, h
        and     07h
        ld      h, a

        ; Select R18 and write high byte
        ld      a, 012h             ; R18 - Update Address High
        out     (crtc_reg), a
        ld      a, h
        out     (crtc_data), a

        ; Select R19 and write low byte
        ld      a, 013h             ; R19 - Update Address Low
        out     (crtc_reg), a
        ld      a, l
        out     (crtc_data), a

        ; Send strobe command
        ld      a, strcmd
        out     (crtc_reg), a

        ; Wait for Update Ready (bit 7 of port 0x1C)
crtc_read_wait:
        in      a, (crtc_reg)
        or      a
        jp      p, crtc_read_wait   ; Loop while bit 7 = 0

        ; Read VRAM data
        in      a, (crtc_vram)

        pop     hl
        ret

; ============================================================================
; CRTC VRAM Write - Write byte to VRAM via SY6545 transparent addressing
; Input: HL = VRAM address (0x0000-0x07FF), A = byte to write
; Clobbers: BC
;
; Protocol (from diag4.mac cr5/cr6):
;   1-6. Same as read (set address and strobe)
;   7. OUT 0x1F, A          (write VRAM data)
;   8. Wait for IN 0x1C bit 7 = 1 (write complete)
; ============================================================================
crtc_write:
        push    hl
        push    af                  ; Save data byte
        ; Mask address to 11 bits (0-0x7FF)
        ld      a, h
        and     07h
        ld      h, a

        ; Select R18 and write high byte
        ld      a, 012h             ; R18 - Update Address High
        out     (crtc_reg), a
        ld      a, h
        out     (crtc_data), a

        ; Select R19 and write low byte
        ld      a, 013h             ; R19 - Update Address Low
        out     (crtc_reg), a
        ld      a, l
        out     (crtc_data), a

        ; Send strobe command
        ld      a, strcmd
        out     (crtc_reg), a

        ; Wait for Update Ready (bit 7 of port 0x1C)
crtc_write_wait1:
        in      a, (crtc_reg)
        or      a
        jp      p, crtc_write_wait1 ; Loop while bit 7 = 0

        ; Write VRAM data
        pop     af                  ; Restore data byte
        out     (crtc_vram), a

        ; Wait for write complete
crtc_write_wait2:
        in      a, (crtc_reg)
        or      a
        jp      p, crtc_write_wait2 ; Loop while bit 7 = 0

        pop     hl
        ret

; ============================================================================
; Utility routines
; ============================================================================

print:
        ld      c, prtstr
        jp      bdos

print_hex16:
        ld      a, h
        call    print_hex8
        ld      a, l

print_hex8:
        push    af
        rrca
        rrca
        rrca
        rrca
        call    print_nibble
        pop     af

print_nibble:
        and     00Fh
        add     a, 030h
        cp      03Ah
        jr      c, pn_out
        add     a, 7
pn_out:
        ld      e, a
        ld      c, conout
        jp      bdos

; ============================================================================
; Messages
; ============================================================================
msg_banner:
        db      13, 10
        db      "=== izkaypro Emulator Diagnostics ===", 13, 10
        db      "Based on diag4.mac (c) 1983 NLS Inc.", 13, 10
        db      13, 10, "$"

msg_rom:
        db      "ROM Checksum Test: $"

msg_chksum:
        db      "0x$"

msg_ram1:
        db      "RAM Test 0x4000-0x7FFF: $"

msg_ram2:
        db      "RAM Test 0x8000-0xBFFF: $"

msg_vram:
        db      "VRAM Test 0x0000-0x07FF: $"

msg_pass:
        db      "PASS", 13, 10, "$"

msg_fail:
        db      "FAIL", 13, 10, "$"

msg_done:
        db      13, 10
        db      "Diagnostics complete.", 13, 10, "$"

msg_crlf:
        db      13, 10, "$"

; ============================================================================
; Stack
; ============================================================================
        ds      64
stack:

        end
