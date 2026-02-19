// Copyright (c) 2020 Douglas Miller <durgadas311@gmail.com>

// This is paired with VirtualPPort interface. A device attached to PIO (etc) provides
// this interface to VirtualPPort(s), and uses the VirtualPPort interface
// to update input bits.
public interface PPortDevice {
	void refresh();		// poke any passive inputs
	boolean ready();	// ready for output?
	void outputs(int val);	// outputs have changed
	String dumpDebug();
}
