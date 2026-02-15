// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Vector;
import java.util.Properties;
import java.io.*;
import java.awt.Font;

public class ProgramParallel extends InputStream implements PPortDevice, Runnable {
	VirtualPPort pport;
	RunProgram prog;
	java.util.concurrent.LinkedBlockingDeque<Integer> fifo;

	public ProgramParallel(Properties props, Vector<String> argv, VirtualPPort pport) {
		this.pport = pport;
		fifo = new java.util.concurrent.LinkedBlockingDeque<Integer>();
		// WARNING! destructive to caller's 'argv'!
		argv.removeElementAt(0);
		prog = new RunProgram(argv, this, true);
		if (prog.excp == null) {
			Thread t = new Thread(this);
			t.start();
		} else {
			prog.excp.printStackTrace();
		}
	}

	// PPortDevice interface
	public void refresh() {		// poke any passive inputs
	}
	public boolean ready() {	// ready for output?
		// throttle to 1 char?
		return (fifo.size() == 0);
	}
	public void outputs(int val) {	// outputs have changed
		fifo.add(val);
	}
	public String dumpDebug() {
		// TODO: print command, process and fifo status
		return "";
	}

	// InputStream interface
	public int read() {
		try {
			int c = fifo.take();
			return c;
		} catch (Exception ee) {
			return -1;
		}
	}
	public int available() {
		return fifo.size();
	}
	public void close() {
		pport.detach();
	}

	// This thread reads program stdout and sends to PPort.
	// If the port does not accept input, it should spend
	// all it's time sleeping in read().
	public void run() {
		while (true) {
			try {
				// This probably needs to be throttled...
				int c = prog.proc.getInputStream().read();
				pport.poke(c, 0xff);
			} catch (Exception ee) {
				ee.printStackTrace();
				pport.detach(); // repetative?
				break;
			}
		}
	}
}
