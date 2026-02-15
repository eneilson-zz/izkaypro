// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Vector;
import java.util.Properties;
import java.io.*;
import java.awt.Font;

public class Diablo630Parallel extends InputStream implements PPortDevice, Runnable {
	String file;
	Diablo630 front_end;
	java.util.concurrent.LinkedBlockingDeque<Integer> fifo;

	public Diablo630Parallel(Properties props, Vector<String> argv, VirtualPPort pprt) {
		file = "out.ps";
		fifo = new java.util.concurrent.LinkedBlockingDeque<Integer>();
		pprt.attach(this);
		// Defaulting to 10 cpi, 6 lpi...
		front_end = new Diablo630(props, argv, this);
		Thread t = new Thread(this);
		t.start();
	}

	// InputStream interface, for Diablo630
	public int read() {
		// We can sleep here...
		try {
			return fifo.take();
		} catch (Exception ee) {
			// ee.printStackTrace();
			return -1;
		}
	}
	public int available() {
		return fifo.size();
	}

	// PPortDevice interface, for 'pprt'
	public void refresh() {}	// poke any passive inputs
	public boolean ready() {	// ready for output?
		return fifo.size() == 0; // effectively limits to 1 char
	}
	public void outputs(int val) {	// outputs have changed
		// TODO: truncate fifo to 1 char?
		fifo.add(val & 0xff);
	}

	public String dumpDebug() {
		String ret = String.format("Diablo630Parallel fifo=%d\n", fifo.size());
		return ret;
	}

	public void run() {
		front_end.runPrinter(file); // this callsback our InputStream
		System.err.println("Diablo 630 detaching");
	}
}
