// Copyright (c) 2020 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Vector;
import java.util.Properties;
import java.io.*;
import java.awt.Font;

public class ProgramModem extends InputStream implements SerialDevice, Runnable {
	VirtualUART uart;
	RunProgram prog;
	java.util.concurrent.LinkedBlockingDeque<Integer> fifo;

	public ProgramModem(Properties props, Vector<String> argv, VirtualUART uart) {
		this.uart = uart;
		fifo = new java.util.concurrent.LinkedBlockingDeque<Integer>();
		uart.attachDevice(this);
		// WARNING! destructive to 'argv'!
		argv.removeElementAt(0);
		prog = new RunProgram(argv, this);
		// Start program later, by modem control
	}

	// TODO: work out exactly how to do this...
	private void start() {
		if (prog.excp == null) {
			Thread t = new Thread(this);
			t.start();
			prog.excp = null;
			// TODO: allow special program codes to change?
			uart.setModem(VirtualUART.SET_CTS | VirtualUART.SET_DSR);
		} else {
			prog.excp.printStackTrace();
			uart.detach(); // repetative?
			fifo.clear();
		}
	}

	private void stop() {
		if (prog.excp == null) {
			prog.proc.destroy();
			try {
				prog.proc.waitFor();
			} catch (Exception ee) {}
			prog.excp = new IllegalThreadStateException("Terminated");
		} else {
			prog.excp.printStackTrace();
			uart.detach(); // repetative?
			fifo.clear();
		}
	}

	// SerialDevice interface
	public void write(int b) {	// CPU is writing the serial data port
		fifo.add(b);
	}
	//int read();		// CPU is reading the serial data port
	//int available();	// returns number available bytes on Rx (0/1)
	public void rewind() {}
	// bits a la VirtualUART get/setModem()
	public void modemChange(VirtualUART me, int mdm) {
System.err.format("MODEM LINES %04x\n", mdm);
		// if ... start();
		// else stop();
	}
	public int dir() { return SerialDevice.DIR_OUT; }
	public String dumpDebug() {
		return "";	// TODO: debug data
	}

	// InputStream interface
	public int read() {
		try {
			int c = fifo.take();
			return c;
		} catch (Exception ee) {
			// ee.printStackTrace();
			return -1;
		}
	}
	public int available() {
		return fifo.size();
	}
	public void close() {
		uart.detach();
		fifo.clear();
	}

	// This thread reads program stdout and sends to UART
	public void run() {
		while (true) {
			try {
				// This probably needs to be throttled...
				int c = prog.proc.getInputStream().read();
				uart.put(c, true);
			} catch (Exception ee) {
				ee.printStackTrace();
				uart.detach();
				break;
			}
		}
	}
}
