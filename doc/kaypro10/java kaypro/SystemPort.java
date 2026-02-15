// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Vector;
import java.util.Properties;

public class SystemPort implements IODevice, GeneralPurposePort {
	private int gpo;
	private Interruptor intr;
	private Vector<GppListener> notif;
	private Vector<GppProvider> inps;
	static final int inputs = 0x40;

	public SystemPort(Properties props, Interruptor intr) {
		this.intr = intr;
		notif = new Vector<GppListener>();
		inps = new Vector<GppProvider>();
		reset();
	}

	// Used by memory controller for bank select
	public int get() {
		return gpo;
	}

	/// IODevice interface ///

	public void reset() {
		gpo = 0xff;	// special FF effectively resets to 1's
		notify(0xff); // might contradict device resets?
	}

	public int getBaseAddress() {
		return 0x14;
	}
	public int getNumPorts() {
		return 4;
	}
	public String getDeviceName() { return null; }

	public int in(int port) {
		// check port?
		int val = gpo & ~inputs;
		for (GppProvider inp : inps) {
			val |= (inp.gppInputs() & inputs);
		}
		return val;
	}
	public void out(int port, int value) {
		// check port?
		value &= 0xff;
		int diff = gpo ^ value;
		gpo = value;
		notify(diff);
	}

	private void notify(int diff) {
		if (diff == 0) {
			return;
		}
		for (GppListener lstn : notif) {
			if ((diff & lstn.interestedBits()) != 0) {
				lstn.gppNewValue(gpo);
			}
		}
	}

	public void addGppProvider(GppProvider inp) {
		inps.add(inp);
	}

	public void addGppListener(GppListener lstn) {
		notif.add(lstn);
		lstn.gppNewValue(gpo);
	}

	public String dumpDebug() {
		String ret = new String();
		ret += String.format("GPIO = %02x PRSTB = ?\n", gpo);
		return ret;
	}
}
