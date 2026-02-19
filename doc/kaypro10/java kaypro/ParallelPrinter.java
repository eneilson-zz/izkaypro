// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Vector;
import java.util.Properties;
import java.io.*;
import java.awt.event.*;
import javax.swing.*;
import java.lang.reflect.Constructor;
import java.util.concurrent.Semaphore;

public class ParallelPrinter
		implements IODevice, GppListener, GppProvider, VirtualPPort {
	static final int ctrl_PRSTB_c = 0x08;
	static final int ctrl_PRBSY_c = 0x40;	// "1" == "Busy"

	private int data;
	private boolean strobe;
	private PPortDevice attObj;
	private OutputStream attFile;
	private boolean excl = true;
	private Semaphore wait;

	public ParallelPrinter(Properties props, GeneralPurposePort gpio) {
		wait = new Semaphore(0);
		String s = props.getProperty("pprinter_att");
		if (s != null && s.length() > 1) {
			if (s.charAt(0) == '>') { // redirect output to file
				attachFile(s.substring(1));
			} else if (s.charAt(0) == '|') { // pipe to program
				attachPipe(s.substring(1));
			} else {
				attachClass(props, s);
			}
		}
		reset();
		gpio.addGppListener(this);
		gpio.addGppProvider(this);
	}

	private void attachFile(String s) {
		String[] args = s.split("\\s");
		setupFile(args, 0);
	}

	private void setupFile(String[] args, int start) {
		boolean append = false;
		for (int x = start + 1; x < args.length; ++x) {
			if (args[x].equals("a")) {
				append = true;
			} else if (args[x].equals("+")) {
				excl = false;
			}
		}
		if (args[start].equalsIgnoreCase("syslog")) {
			attFile = System.err;
			excl = false;
		} else try {
			attFile = new FileOutputStream(args[start], append);
		} catch (Exception ee) {
			System.err.format("Invalid file in attachment: %s\n", args[start]);
			return;
		}
	}

	private void attachPipe(String s) {
		System.err.format("Pipe attachments not yet implemented: %s\n", s);
	}

	private void attachClass(Properties props, String s) {
		String[] args = s.split("\\s");
		for (int x = 1; x < args.length; ++x) {
			if (args[x].startsWith(">")) {
				excl = false;
				args[x] = args[x].substring(1);
				setupFile(args, x);
				// TODO: truncate args so Class doesn't see?
			}
		}
		Vector<String> argv = new Vector<String>(Arrays.asList(args));
		// try to construct from class...
		try {
			Class<?> clazz = Class.forName(args[0]);
			Constructor<?> ctor = clazz.getConstructor(
					Properties.class,
					argv.getClass(),
					VirtualPPort.class);
			// funky "new" avoids "argument type mismatch"...
			Object obj = ctor.newInstance(
					props,
					argv,
					(VirtualPPort)this);
			attach((PPortDevice)obj);
		} catch (Exception ee) {
			//ee.printStackTrace();
			System.err.format("Invalid class in attachment: %s\n", s);
			return;
		}
	}

	public int getBaseAddress() { return 0x18; }
	public int getNumPorts() { return 4; }

	public void reset() {
		data = 0;
	}

	public int in(int addr) {
		// not possible
		return 0;
	}

	public void out(int addr, int val) {
		// Nothing happens until STROBE...
		data = val;
	}

	public String getDeviceName() {
		return "ParallelPrinter";
	}

	public int gppInputs() {
		return (attObj != null && !attObj.ready() ? ctrl_PRBSY_c : 0);
	}

	public int interestedBits() {
		return ctrl_PRSTB_c;
	}

	public void gppNewValue(int gpio) {
		// falling edge of ctrl_PRSTB_c triggers action
		if ((gpio & ctrl_PRSTB_c) == 0) {
			// we have a byte to send...
			if (attFile != null) {
				try {
					attFile.write(data);
				} catch (Exception ee) {}
			}
			if (attObj != null) {
				attObj.outputs(data);
			}
		}
	}

	// new VirtualPPort interface
	public void poke(int val, int msk) {}	// no inputs
	public boolean attach(PPortDevice periph) {
		if (attObj != null) {
			return false;
		}
		attObj = periph;
		return true;
	}
	public void detach() {
		// Wake up sleepers, dereference object.
		// Must null attObj first to prevent looping
		System.err.format("%s detaching peripheral\n", getDeviceName());
		attObj = null;
		wait.release();
	}

	public String dumpDebug() {
		String ret = String.format(
			"ParallelPrinter DAT=%02x STB=%s\n", data, strobe);
		if (attObj != null) {
			ret += attObj.dumpDebug();
		}
		return ret;
	}
}
