// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Vector;
import java.util.Properties;

public class SystemPortPIO implements GeneralPurposePort, PPortDevice {
	private int gpo;
	private Vector<GppListener> notif;
	private Vector<GppProvider> inps;
	static final int inputs = 0x08;
	VirtualPPort pport;

	public SystemPortPIO(Properties props, VirtualPPort pport) {
		this.pport = pport;
		notif = new Vector<GppListener>();
		inps = new Vector<GppProvider>();
		pport.attach(this);
	}

	// Used by memory controller for bank select
	public int get() {
		return gpo;
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

	public void refresh() {		// poke any passive inputs
		int val = 0;
		for (GppProvider inp : inps) {
			val |= (inp.gppInputs() & inputs);
		}
		pport.poke(val, inputs);
	}
	public boolean ready() { return true; }	// ready for output?
	public void outputs(int val) {	// outputs have changed
		int diff = gpo ^ val;
		gpo = val;
		if (diff != 0) {
			notify(diff);
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
