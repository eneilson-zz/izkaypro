; EMUTEST.COM - Emulator ROM and RAM diagnostics
; Based on diag4.mac from Non-Linear Systems, Inc. (1983)
; Implements proper ROM bank switching by relocating code to 0x8000

; CP/M BDOS calls
bdos:           equ     5
conout:         equ     2
prtstr:         equ     9

; Kaypro 4-84 port
bitport:        equ     014h

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
