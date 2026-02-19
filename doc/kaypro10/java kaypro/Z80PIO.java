// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Vector;
import java.util.Properties;
import java.io.*;
import java.lang.reflect.Constructor;

public class Z80PIO implements IODevice, InterruptController {
	private Interruptor intr;
	private int src;
	private int basePort;
	private String name = null;

	private Z80PIOPort[] ports = new Z80PIOPort[2];
	private int intrs = 0;

	private int abMask = 1;
	private int abShr = 0;
	private int cdMask = 2;
	private int cdShr = 1;

	public Z80PIO(Properties props, String pfxA, String pfxB, int base,
			Interruptor intr, boolean swap) {
		name = String.format("Z80PIO-%02x", base);
		if (swap) {
			abMask = 2;
			abShr = 1;
			cdMask = 1;
			cdShr = 0;
		}
		this.intr = intr;
		src = intr.registerINT(0);
		intr.addIntrController(this);
		basePort = base;
		ports[0] = new Z80PIOPort(props, pfxA, 0);
		ports[1] = new Z80PIOPort(props, pfxB, 1);
		reset();
	}

	///////////////////////////////
	/// Interfaces for IODevice ///
	public int in(int port) {
		int x = (port & abMask) >> abShr;
		return ports[x].in(port >> cdShr);
	}

	public void out(int port, int val) {
		int x = (port & abMask) >> abShr;
		ports[x].out(port >> cdShr, val);
	}

	public void reset() {
		intrs = 0;
		intr.lowerINT(0, src);
		ports[0].reset();
		ports[1].reset();
	}
	public int getBaseAddress() {
		return basePort;
	}
	public int getNumPorts() {
		return 4;
	}
	public String getDeviceName() {
		return name;
	}

	private void raiseINT(int idx) {
		int i = intrs;
		intrs |= (1 << idx);
		if (i == 0) {
			intr.raiseINT(0, src);
		}
	}

	private void lowerINT(int idx) {
		int i = intrs;
		intrs &= ~(1 << idx);
		if (i != 0 && intrs == 0) {
			intr.lowerINT(0, src);
		}
	}

	public VirtualPPort portA() { return ports[0]; }
	public VirtualPPort portB() { return ports[1]; }

	public int readDataBus() {
		if (intrs == 0) {
			return -1;
		}
		int vec = -1;
		if ((intrs & 1) != 0) {
			vec = ports[0].readDataBus();
		} else if ((intrs & 2) != 0) {
			vec = ports[1].readDataBus();
		}
		return vec;
	}

	public boolean retIntr() {
		if (intrs == 0) {
			return false;
		}
		if ((intrs & 1) != 0) {
			ports[0].retIntr();
		} else if ((intrs & 2) != 0) {
			ports[1].retIntr();
		}
		if (intrs == 0) {
			intr.lowerINT(0, src);
		}
		return true;
	}

	class Z80PIOPort implements VirtualPPort {
		private PPortDevice attObj;
		private OutputStream attFile;
		private boolean excl = true;
		private int index;
		private int mode;	// Operating mode
		private int vec;	// Interrupt vector
		private boolean intrEnable;
		private boolean ready;
		private boolean avail;
		private int data;
		private int datai;
		private int inputs;	// I/O Mask (1 = Input)
		private int disables;	// INT disable mask (1 = disable)
		private boolean and;	// All must be active for INT
		private boolean high;	// All must be high for INT

		private boolean ctl;	// control byte follows
		private boolean mask;	// mask byte follows

		public Z80PIOPort(Properties props, String pfx, int idx) {
			attObj = null;
			attFile = null;
			index = idx;
			if (pfx == null) {
				return;
			}
			String s = props.getProperty(pfx + "_att");
			if (s != null && s.length() > 1) {
				if (s.charAt(0) == '>') { // redirect output to file
					attachFile(s.substring(1));
				} else if (s.charAt(0) == '!') { // pipe to/from program
					attachPipe(s.substring(1));
				} else {
					attachClass(props, s);
				}
			}
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
				if (excl) {
					// handshake?
				}
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
				System.err.format("Invalid class in attachment: %s\n", s);
				return;
			}
		}

		// Conditions affecting interrupts have changed, ensure proper signal.
		private void chkIntr() {
			int intr = 0;
			int bits = 0;
			switch (mode) {
			case 0:
			case 2:
				if (!avail) {
					intr |= 1;
				}
				if (mode == 0) {
					break;
				}
			case 1:
				if (ready) {
					intr |= 2;
				}
				break;
			case 3:
				int val = (data & ~inputs) | (datai & inputs);
				// RTC example:
				// EI, OR (!and), HI (high)
				// disables = 10111111b
				// data=00000000b => (11111111b != 0xff: intr off)
				// data=01000000b => (10111111b != 0xff: intr on)
				if (high) {
					// convert to active-low
					bits = ~val;
				} else {
					bits = val;
				}
				bits |= disables;
				bits &= 0xff;
				if (and && bits == disables) {
					intr |= 4;
				} else if (!and && bits != 0xff) {
					intr |= 4;
				}
				break;
			}
			if (!intrEnable || intr == 0) {
				lowerINT(index);
			} else if (intr != 0) {
				raiseINT(index);
			}
		}

		// Outputting data must be preserved over mode change.
		// SYSPORT initializes data while port is INPUT(1),
		// then changes to BIT CONTROL mode (3) and must be
		// able to read data that was previously written.
		// We fudge this in poke(), where mode 1 or 2 bypass
		// the 'datai' variable.
		public int in(int port) {
			int x = port & 1;
			int val = 0;
			if (x == 0) {	// data
				if (mode != 0 && attObj != null) {
					attObj.refresh();
				}
				switch (mode) {
				case 0:
					break;
				case 1:
				case 2:
					val = data;
					ready = true;
					chkIntr();
					break;
				case 3:
					val = (data & ~inputs) | (datai & inputs);
					break;
				}
			} else {	// control
				// TODO: what does this mean?
			}
			return val;
		}

		public void out(int port, int val) {
			int x = port & 1;
			val &= 0xff; // necessary?
			if (x == 0) {	// data
				switch (mode) {
				case 0:
				case 2:
					data = (val & 0xff);
					avail = true;
					chkIntr();
					break;
				case 1:
					data = (val & 0xff);
					break;
				case 3:
					data = (val & 0xff);
					chkIntr();	// ??
					break;
				}
				if (attObj != null && mode != 1) {
					// this doubles as STB
					attObj.outputs(data | inputs);
				}
			} else {	// control
				if (ctl) {
					ctl = false;
					inputs = val & 0xff;
					return;
				}
				if (mask) {
					mask = false;
					disables = val & 0xff;
					chkIntr();
					return;
				}
				if ((val & 1) == 0) {
					// xxxxxxx0 = interrupt vector
					vec = val;
					return;
				}
				switch (val & 0x0f) {
				case 0x0f:	// set mode
					// xxxx1111 = set mode
					mode = (val >> 6);
					ctl = (mode == 3);
					if (mode == 0) {
						inputs = 0;
					} else {
						inputs = 0xff;
					}
					return;
				case 0x07:	// interrupt control
					// xxxx0111 = interrupt control
					intrEnable = ((val & 0x80) != 0);
					and = ((val & 0x40) != 0);
					high = ((val & 0x20) != 0);
					mask = ((val & 0x10) != 0);
					if (mask) {
						lowerINT(index);
					} else {
						chkIntr();
					}
					return;
				case 0x03:	// interrupt enable/disable
					// xxxx0011 = interrupt enable set
					intrEnable = ((val & 0x80) != 0);
					return;
				}
			}
		}

		public void reset() {
			ready = false;
			avail = false;
			intrEnable = false;
			mode = 1;
			inputs = 0xff;
			disables = 0xff;
			lowerINT(index);
		}

		public int readDataBus() {
			return vec;
		}

		public void retIntr() {
			chkIntr(); // normally results in lowerINT(index)
		}

		// New interface
		public void poke(int val, int msk) {
			if (mode == 3) {
				datai = (datai & ~msk) | (val & msk);
			} else {
				// 'msk' should be FF...
				data = (data & ~msk) | (val & msk);
			}
			chkIntr();
		}

		public boolean attach(PPortDevice periph) {
			if (attObj != null) {
				return false;
			}
			attObj = periph;
			if (attObj != null) {
				attObj.outputs(data | inputs);
			}
			return true;
		}
		public void detach() {
			// Wake up any sleepers, dereference object.
			System.err.format("%s-%c detaching peripheral\n",
				getDeviceName(), index + 'A');
			attObj = null; // must precede wakeups
			// iwait.release();
			// owait.release();
		}

		public String getDeviceName() { return name; }

		public String dumpDebug() {
			String ret = new String();
			ret += String.format("ch %c, vec = %02x mode = %d\n" +
				"dir = %02x data = %02x\n" +
				"IE = %s, AND = %s, HI = %s mask = %02x\n" +
				"avail = %s, ready = %s\n",
				index + 'A', vec, mode, inputs, data,
				intrEnable, and, high, disables,
				avail, ready);
			if (attObj != null) {
				ret += attObj.dumpDebug();
			}
			return ret;
		}
	}

	public String dumpDebug() {
		String ret = new String();
		ret += String.format("port %02x intrs = %02x\n", basePort, intrs);
		ret += ports[0].dumpDebug();
		ret += '\n';
		ret += ports[1].dumpDebug();
		return ret;
	}
}
