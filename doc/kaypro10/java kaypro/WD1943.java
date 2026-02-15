// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>
import java.util.Vector;

// User must instantiate two of these per "chip" for the Kaypro I/O decoding scheme.
public class WD1943 implements IODevice {

	private int basePort;
	private int numPorts;
	private String devName;
	private int divisor;
	private Vector<BaudListener> lstns;

	static int[] bauds = new int[]{
		// NOTE: "134" is actually "134.5"... if anyone cares...
		50, 75, 110, 134, 150, 300, 600, 1200,
		1800, 2000, 2400, 3600, 4800, 7200, 9600,
		19200,
	};

	public WD1943(int base, int num, String name) {
		basePort = base;
		numPorts = num;
		devName = name;
		lstns = new Vector<BaudListener>();
		reset();
	}

	public void reset() {
		divisor = 0;
	}

	public int getBaseAddress() {
		return basePort;
	}

	public int getNumPorts() {
		return numPorts;
	}
	public int in(int port) {
		// possible?
		return 0;
	}

	public void out(int port, int value) {
		divisor = value & 0x0f;
		for (BaudListener lstn : lstns) {
			lstn.setBaud(bauds[divisor] * 16);
		}
	}

	public void addBaudListener(BaudListener lstn) {
		lstns.add(lstn);
		lstn.setBaud(bauds[divisor] * 16);
	}

	public String getDeviceName() { return devName; }

	public String dumpDebug() {
		String str = String.format("%s Div = %02x, Baud = %d\n",
			devName, divisor, bauds[divisor]);
		return str;
	}
}
