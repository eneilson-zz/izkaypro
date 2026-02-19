// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Properties;
import java.io.*;

public class Memory84X extends KayproRoms implements Memory, GppListener, IODevice {
	GeneralPurposePort gpp;
	private byte[] mem;
	private boolean rom;
	private int commPage;
	private int wrBank;
	private int rdBank;

	public Memory84X(Properties props, GeneralPurposePort gpio, String defrom) {
		super(props, defrom);
		gpp = gpio;
		mem = new byte[256*1024];
		// rely entirely on notifications for 'rom' value.
		gpp.addGppListener(this);
	}

	public int read(boolean rom, int bank, int address) {
		address &= 0xffff; // necessary?
		if (rom && address < 0x8000) {
			if (address <= monMask) {
				return mon[address] & 0xff;
			}
			return 0;
		}
		if (address < commPage) {
			address |= bank << 16;
		}
		return mem[address & 0x3ffff] & 0xff;
	}
	public int read(int address) {
		int bank = (address < commPage ? rdBank : 0);
		return read(rom, bank, address);
	}
	public void write(int address, int value) {
		if (rom && address < 0x8000) {
			// TODO: what did the Kaypro do in this case?
			return;
		}
		if (address < commPage) {
			address |= wrBank << 16;
		}
		mem[address & 0x3ffff] = (byte)value;
	}

	public int interestedBits() { return 0x80; }
	public void gppNewValue(int gpio) {
		rom = ((gpio & 0x80) != 0);
	}

	public void reset() {
		commPage = 0;
		wrBank = 0;
		rdBank = 0;
	}

	public int getBaseAddress() {
		return 0x3c;
	}

	public int getNumPorts() {
		return 4;
	}

	public int in(int port) {
		return 0;
	}

	public void out(int port, int value) {
		commPage = (value & 0xf0) << 8;
		wrBank = (value & 0x0c) >> 2;
		rdBank = (value & 0x03);
	}

	public String getDeviceName() {
		return "Memory84X";
	}

	public void dumpCore(String file) {
		try {
			OutputStream core = new FileOutputStream(file);
			core.write(mem);
			core.close();
		} catch (Exception ee) {
			KayproOperator.error(null, "Core Dump", ee.getMessage());
		}
	}

	public String dumpDebug() {
		String str = String.format("rom=%s comm=%04x rd=%d wr=%d\n",
			rom, commPage, rdBank, wrBank);
		return str;
	}
}
