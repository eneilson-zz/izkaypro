// Copyright (c) 2020 Douglas Miller <durgadas311@gmail.com>

// This is paired with PPortDevice interface. A PIO (etc) provides
// this interface to PPortDevice(s), and uses the PPortDevice interface
// to trigger events on attached devices.
public interface VirtualPPort {
	void poke(int val, int msk);	// active input(s) changed
	boolean attach(PPortDevice periph);
	void detach();		// Peripheral no longer usable
}
