#!/usr/bin/env python3
"""Generate RTCTEST.COM — a CP/M program that tests RTC interrupts.

Sets up IM2 with PIO vector, configures the MM58167A for 1/sec interrupts,
and displays updating date/time on the console.

Usage: python3 tools/mkrtctest.py
Output: disks/rtctest.com
"""

class Asm:
    """Minimal Z80 assembler with label resolution."""
    def __init__(self, org=0x0100):
        self.org = org
        self.code = bytearray()
        self.labels = {}
        self.fixups = []

    def addr(self):
        return self.org + len(self.code)

    def label(self, name):
        self.labels[name] = self.addr()

    def db(self, *args):
        for a in args:
            if isinstance(a, int):
                self.code.append(a & 0xFF)
            elif isinstance(a, (str, bytes)):
                for c in (a.encode() if isinstance(a, str) else a):
                    self.code.append(c if isinstance(c, int) else ord(c))

    def emit(self, *bs):
        for b in bs:
            self.code.append(b & 0xFF)

    def _ref(self, label):
        if isinstance(label, int):
            return label
        self.fixups.append((len(self.code), label, 'abs16'))
        return None

    def jp(self, label):
        self.emit(0xC3)
        r = self._ref(label)
        if r is not None:
            self.emit(r & 0xFF, (r >> 8) & 0xFF)
        else:
            self.emit(0, 0)

    def jp_nz(self, label):
        self.emit(0xC2)
        r = self._ref(label)
        if r is not None:
            self.emit(r & 0xFF, (r >> 8) & 0xFF)
        else:
            self.emit(0, 0)

    def call(self, label):
        self.emit(0xCD)
        r = self._ref(label)
        if r is not None:
            self.emit(r & 0xFF, (r >> 8) & 0xFF)
        else:
            self.emit(0, 0)

    def jr(self, label):
        self.emit(0x18)
        self.fixups.append((len(self.code), label, 'rel8'))
        self.emit(0)

    def jr_z(self, label):
        self.emit(0x28)
        self.fixups.append((len(self.code), label, 'rel8'))
        self.emit(0)

    def jr_nz(self, label):
        self.emit(0x20)
        self.fixups.append((len(self.code), label, 'rel8'))
        self.emit(0)

    def ld_a_mem(self, label):
        self.emit(0x3A)
        r = self._ref(label)
        if r is not None:
            self.emit(r & 0xFF, (r >> 8) & 0xFF)
        else:
            self.emit(0, 0)

    def ld_mem_a(self, label):
        self.emit(0x32)
        r = self._ref(label)
        if r is not None:
            self.emit(r & 0xFF, (r >> 8) & 0xFF)
        else:
            self.emit(0, 0)

    def ld_hl(self, label):
        self.emit(0x21)
        r = self._ref(label)
        if r is not None:
            self.emit(r & 0xFF, (r >> 8) & 0xFF)
        else:
            self.emit(0, 0)

    def ld_mem_hl(self, addr):
        self.emit(0x22, addr & 0xFF, (addr >> 8) & 0xFF)

    def resolve(self):
        for offset, label, typ in self.fixups:
            addr = self.labels[label] if isinstance(label, str) else label
            if typ == 'abs16':
                self.code[offset] = addr & 0xFF
                self.code[offset + 1] = (addr >> 8) & 0xFF
            elif typ == 'rel8':
                rel = addr - (self.org + offset + 1)
                assert -128 <= rel <= 127, f"JR out of range: {rel} for {label}"
                self.code[offset] = rel & 0xFF

    def save(self, filename):
        self.resolve()
        with open(filename, 'wb') as f:
            f.write(self.code)
        print(f"Wrote {len(self.code)} bytes to {filename}")


BDOS = 0x0005
PIO_VECTOR = 0x20
I_REG = 0x04
VECTOR_TABLE = I_REG << 8  # 0x0300
VECTOR_ENTRY = VECTOR_TABLE | PIO_VECTOR  # 0x0320

PORT_CLKADD = 0x20
PORT_CLKCTL = 0x22
PORT_CLKDAT = 0x24

a = Asm(org=0x0100)

# --- Entry point ---
a.jp('main')

# --- Variables ---
a.label('flag')
a.db(0)
a.label('tick_count')
a.db(0, 0)  # 16-bit tick counter

# --- Strings ---
a.label('str_title')
a.db('\x1b=\x20\x20')  # ESC = space space → cursor to 0,0
a.db('RTC Interrupt Clock Test\r\n')
a.db('========================\r\n\r\n$')

a.label('str_reading')
a.db('Reading RTC directly:  $')

a.label('str_setup')
a.db('\r\n\r\nSetting up IM2 + PIO + RTC 1/sec interrupt...\r\n$')

a.label('str_done')
a.db('Done. Waiting for ticks.\r\n\r\n$')

a.label('str_exit')
a.db('\r\n\r\nDisabling interrupts, exiting.\r\n$')

a.label('str_tick')
a.db('\r  Tick $')

a.label('str_date')
a.db('   Date: $')

a.label('str_time')
a.db('  Time: $')

a.label('str_slash')
a.db('/$')

a.label('str_colon')
a.db(':$')

a.label('str_space')
a.db('  $')

a.label('str_press')
a.db('Press any key to exit.\r\n\r\n$')

# --- Main program ---
a.label('main')
# Print title
a.emit(0x0E, 0x09)           # LD C, 9 (print string)
a.ld_hl('str_title')          # LD DE, str_title (actually need DE)
# Oops, need to use DE for BDOS. Let me use a print_str subroutine.
# Actually let me just inline: LD DE,str / LD C,9 / CALL 5

# Let me redo this with a helper
a.code = a.code[:6]  # Reset to after variables (JP=3 + flag=1 + tick_count=2)
a.labels = {'flag': 0x0103, 'tick_count': 0x0104}
a.fixups = [(1, 'main', 'abs16')]

# Redefine strings
a.label('str_title')
a.db('\x1b=\x20\x20')
a.db('RTC Interrupt Clock Test\r\n')
a.db('========================\r\n\r\n$')

a.label('str_reading')
a.db('Reading RTC directly:  $')

a.label('str_setup')
a.db('\r\n\r\nSetting up IM2 + PIO + RTC 1/sec interrupt...\r\n$')

a.label('str_done')
a.db('Done. Waiting for ticks.\r\n\r\n$')

a.label('str_exit')
a.db('\r\n\r\nDisabling interrupts, exiting.\r\n$')

a.label('str_tick')
a.db('\r  Tick $')

a.label('str_date')
a.db('   Date: $')

a.label('str_time')
a.db('  Time: $')

a.label('str_slash')
a.db('/$')

a.label('str_colon')
a.db(':$')

a.label('str_press')
a.db('Press any key to exit.\r\n\r\n$')

# --- print_str: DE = string address ($ terminated), calls BDOS 9 ---
a.label('print_str')
a.emit(0x0E, 0x09)           # LD C, 9
a.emit(0xCD, 0x05, 0x00)     # CALL BDOS
a.emit(0xC9)                  # RET

# --- print_bcd: A = BCD value, prints two ASCII digits ---
a.label('print_bcd')
a.emit(0xF5)                  # PUSH AF
a.emit(0x0F)                  # RRCA
a.emit(0x0F)                  # RRCA
a.emit(0x0F)                  # RRCA
a.emit(0x0F)                  # RRCA
a.emit(0xE6, 0x0F)            # AND 0x0F
a.emit(0xC6, 0x30)            # ADD A, '0'
a.emit(0x5F)                  # LD E, A
a.emit(0x0E, 0x02)            # LD C, 2 (console output)
a.emit(0xCD, 0x05, 0x00)      # CALL BDOS
a.emit(0xF1)                  # POP AF
a.emit(0xE6, 0x0F)            # AND 0x0F
a.emit(0xC6, 0x30)            # ADD A, '0'
a.emit(0x5F)                  # LD E, A
a.emit(0x0E, 0x02)            # LD C, 2
a.emit(0xCD, 0x05, 0x00)      # CALL BDOS
a.emit(0xC9)                  # RET

# --- print_decimal_16: HL = 16-bit value, prints as decimal ---
a.label('print_dec16')
# Simple: divide by 10000, 1000, 100, 10, 1
# For tick counts up to 65535 this works
a.emit(0x01, 0x10, 0x27)      # LD BC, 10000
a.call('print_digit')
a.emit(0x01, 0xE8, 0x03)      # LD BC, 1000
a.call('print_digit')
a.emit(0x01, 0x64, 0x00)      # LD BC, 100
a.call('print_digit')
a.emit(0x01, 0x0A, 0x00)      # LD BC, 10
a.call('print_digit')
a.emit(0x7D)                   # LD A, L
a.emit(0xC6, 0x30)             # ADD A, '0'
a.emit(0x5F)                   # LD E, A
a.emit(0x0E, 0x02)             # LD C, 2
a.emit(0xCD, 0x05, 0x00)       # CALL BDOS
a.emit(0xC9)                   # RET

a.label('print_digit')
a.emit(0x3E, 0x2F)             # LD A, '0' - 1
a.label('print_digit_loop')
a.emit(0x3C)                   # INC A
a.emit(0xB7)                   # OR A (clear carry)
a.emit(0xED, 0x42)             # SBC HL, BC
a.emit(0x30, 0xFB)             # JR NC, print_digit_loop (-5)
a.emit(0x09)                   # ADD HL, BC
a.emit(0x5F)                   # LD E, A
a.emit(0x0E, 0x02)             # LD C, 2
a.emit(0xCD, 0x05, 0x00)       # CALL BDOS
a.emit(0xC9)                   # RET

# --- read_rtc_reg: A = register number, returns A = BCD value ---
a.label('read_rtc_reg')
a.emit(0xD3, PORT_CLKADD)     # OUT (0x20), A  — select register
a.emit(0xDB, PORT_CLKDAT)     # IN A, (0x24)   — read value
a.emit(0xC9)                  # RET

# --- display_datetime: reads and prints "MM/DD  HH:MM:SS" ---
a.label('display_datetime')
# Date
a.emit(0x11)                   # LD DE, str_date
a.fixups.append((len(a.code), 'str_date', 'abs16'))
a.emit(0, 0)
a.call('print_str')

a.emit(0x3E, 0x07)            # LD A, 0x07 (month register)
a.call('read_rtc_reg')
a.call('print_bcd')

a.emit(0x11)
a.fixups.append((len(a.code), 'str_slash', 'abs16'))
a.emit(0, 0)
a.call('print_str')

a.emit(0x3E, 0x06)            # LD A, 0x06 (day register)
a.call('read_rtc_reg')
a.call('print_bcd')

# Time
a.emit(0x11)
a.fixups.append((len(a.code), 'str_time', 'abs16'))
a.emit(0, 0)
a.call('print_str')

a.emit(0x3E, 0x04)            # LD A, 0x04 (hours register)
a.call('read_rtc_reg')
a.call('print_bcd')

a.emit(0x11)
a.fixups.append((len(a.code), 'str_colon', 'abs16'))
a.emit(0, 0)
a.call('print_str')

a.emit(0x3E, 0x03)            # LD A, 0x03 (minutes register)
a.call('read_rtc_reg')
a.call('print_bcd')

a.emit(0x11)
a.fixups.append((len(a.code), 'str_colon', 'abs16'))
a.emit(0, 0)
a.call('print_str')

a.emit(0x3E, 0x02)            # LD A, 0x02 (seconds register)
a.call('read_rtc_reg')
a.call('print_bcd')

a.emit(0xC9)                  # RET

# --- Interrupt handler ---
a.label('isr')
a.emit(0xF5)                  # PUSH AF
a.emit(0x3E, 0x10)            # LD A, 0x10 (interrupt status register)
a.emit(0xD3, PORT_CLKADD)     # OUT (0x20), A
a.emit(0xDB, PORT_CLKDAT)     # IN A, (0x24)  — read clears interrupt
a.emit(0x3E, 0x01)            # LD A, 1
a.ld_mem_a('flag')             # LD (flag), A
# Increment 16-bit tick counter
a.emit(0xF1)                  # POP AF  (restore AF temporarily)
a.emit(0xF5)                  # PUSH AF
a.emit(0xE5)                  # PUSH HL
a.ld_a_mem('tick_count')       # LD A, (tick_count)
a.emit(0x3C)                  # INC A
a.ld_mem_a('tick_count')       # LD (tick_count), A
a.jr_nz('isr_no_carry')
a.ld_a_mem('tick_count_hi')
a.emit(0x3C)                  # INC A
a.ld_mem_a('tick_count_hi')
a.label('isr_no_carry')
a.emit(0xE1)                  # POP HL
a.emit(0xF1)                  # POP AF
a.emit(0xFB)                  # EI
a.emit(0xED, 0x4D)            # RETI

# We need tick_count_hi to be at tick_count + 1
a.labels['tick_count_hi'] = a.labels['tick_count'] + 1

# --- Main code ---
a.label('main')

# Clear screen and print title
a.emit(0x11)
a.fixups.append((len(a.code), 'str_title', 'abs16'))
a.emit(0, 0)
a.call('print_str')

# Print "Reading RTC directly:"
a.emit(0x11)
a.fixups.append((len(a.code), 'str_reading', 'abs16'))
a.emit(0, 0)
a.call('print_str')

# Display date/time from direct RTC reads
a.call('display_datetime')

# Print setup message
a.emit(0x11)
a.fixups.append((len(a.code), 'str_setup', 'abs16'))
a.emit(0, 0)
a.call('print_str')

# --- Set up IM2 interrupt ---
a.emit(0xF3)                  # DI

# Set I register
a.emit(0x3E, I_REG)           # LD A, I_REG (0x03)
a.emit(0xED, 0x47)            # LD I, A

# Switch to IM 2
a.emit(0xED, 0x5E)            # IM 2

# Store ISR address in vector table
# Vector entry at (I << 8) | PIO_VECTOR = 0x0320
a.ld_hl('isr')
a.ld_mem_hl(VECTOR_ENTRY)     # LD (0x0320), HL

# Configure PIO Port A for interrupts
a.emit(0x3E, PIO_VECTOR)      # LD A, 0x20 (vector)
a.emit(0xD3, PORT_CLKCTL)     # OUT (0x22), A
a.emit(0x3E, 0xCF)            # LD A, 0xCF (mode 3)
a.emit(0xD3, PORT_CLKCTL)
a.emit(0x3E, 0xE0)            # LD A, 0xE0 (I/O mask)
a.emit(0xD3, PORT_CLKCTL)
a.emit(0x3E, 0x37)            # LD A, 0x37 (int control: active high, OR, mask follows)
a.emit(0xD3, PORT_CLKCTL)
a.emit(0x3E, 0xBF)            # LD A, 0xBF (int mask: monitor bit 6)
a.emit(0xD3, PORT_CLKCTL)

# Configure RTC: enable 1/sec interrupt
a.emit(0x3E, 0x11)            # LD A, 0x11 (interrupt control register)
a.emit(0xD3, PORT_CLKADD)     # OUT (0x20), A — select reg 0x11
a.emit(0x3E, 0x04)            # LD A, 0x04 (1/sec interrupt)
a.emit(0xD3, PORT_CLKDAT)     # OUT (0x24), A — enable interrupt

# Clear any pending interrupt status
a.emit(0x3E, 0x10)            # LD A, 0x10 (interrupt status register)
a.emit(0xD3, PORT_CLKADD)     # OUT (0x20), A
a.emit(0xDB, PORT_CLKDAT)     # IN A, (0x24) — read clears status

# Enable PIO interrupts
a.emit(0x3E, 0x83)            # LD A, 0x83 (interrupt enable)
a.emit(0xD3, PORT_CLKCTL)     # OUT (0x22), A

# Enable CPU interrupts
a.emit(0xFB)                  # EI

# Print "Done" message
a.emit(0x11)
a.fixups.append((len(a.code), 'str_done', 'abs16'))
a.emit(0, 0)
a.call('print_str')

# Print "Press any key"
a.emit(0x11)
a.fixups.append((len(a.code), 'str_press', 'abs16'))
a.emit(0, 0)
a.call('print_str')

# --- Main loop ---
a.label('main_loop')

# Check for keypress (BDOS 11 = console status)
a.emit(0x0E, 0x0B)            # LD C, 11
a.emit(0xCD, 0x05, 0x00)      # CALL BDOS
a.emit(0xB7)                  # OR A
a.jr_nz('exit')

# Check interrupt flag
a.ld_a_mem('flag')
a.emit(0xB7)                  # OR A
a.jr_z('main_loop')

# Clear flag
a.emit(0xAF)                  # XOR A
a.ld_mem_a('flag')

# Print tick count
a.emit(0x11)
a.fixups.append((len(a.code), 'str_tick', 'abs16'))
a.emit(0, 0)
a.call('print_str')

# Load tick count into HL and print
a.emit(0x2A)                  # LD HL, (tick_count)
a.fixups.append((len(a.code), 'tick_count', 'abs16'))
a.emit(0, 0)
a.call('print_dec16')

# Display date/time
a.call('display_datetime')

a.jr('main_loop')

# --- Exit ---
a.label('exit')
a.emit(0xF3)                  # DI

# Disable RTC interrupts
a.emit(0x3E, 0x11)            # LD A, 0x11
a.emit(0xD3, PORT_CLKADD)     # OUT (0x20), A
a.emit(0x3E, 0x00)            # LD A, 0x00 (disable all)
a.emit(0xD3, PORT_CLKDAT)     # OUT (0x24), A

# Disable PIO interrupts
a.emit(0x3E, 0x03)            # LD A, 0x03 (interrupt disable)
a.emit(0xD3, PORT_CLKCTL)     # OUT (0x22), A

# Restore IM 1
a.emit(0xED, 0x56)            # IM 1
a.emit(0xFB)                  # EI

# Consume the keypress
a.emit(0x0E, 0x01)            # LD C, 1 (console input)
a.emit(0xCD, 0x05, 0x00)      # CALL BDOS

# Print exit message
a.emit(0x11)
a.fixups.append((len(a.code), 'str_exit', 'abs16'))
a.emit(0, 0)
a.call('print_str')

# Return to CP/M
a.emit(0xC3, 0x00, 0x00)      # JP 0x0000 (warm boot)

# --- Pad to vector table ---
current = a.addr()
pad_needed = VECTOR_TABLE - current
if pad_needed < 0:
    print(f"ERROR: Code ({current:#x}) overlaps vector table ({VECTOR_TABLE:#x})!")
    print(f"  Code is {-pad_needed} bytes too long")
    exit(1)
print(f"Code ends at {current:#x}, padding {pad_needed} bytes to vector table at {VECTOR_TABLE:#x}")
for _ in range(pad_needed):
    a.emit(0x00)

# --- Vector table (256 bytes) ---
# Fill with zeros, then we'll patch the entry at runtime via LD (VECTOR_ENTRY),HL
# But we also need the .COM file to include this space
a.label('vector_table')
for _ in range(256):
    a.emit(0x00)

# Save
a.save('disks/rtctest.com')

# Print label map
print("\nLabel map:")
a.resolve()
for name, addr in sorted(a.labels.items(), key=lambda x: x[1]):
    if not name.startswith('print_digit_loop') and not name.startswith('isr_no_carry'):
        print(f"  {name:20s} = {addr:#06x}")
