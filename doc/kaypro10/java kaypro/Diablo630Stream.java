// Copyright (c) 2016 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Vector;
import java.util.Properties;
import java.io.*;
import java.awt.Font;

public class Diablo630Stream extends OutputStream implements Runnable {
	String file;
	Diablo630 front_end;
	java.util.concurrent.LinkedBlockingDeque<Integer> fifo;

	public Diablo630Stream(Properties props, Vector<String> argv) {
		fifo = new java.util.concurrent.LinkedBlockingDeque<Integer>();
		file = "out.ps";
		for (String arg : argv) {
			if (arg.startsWith("file=")) {
				file = arg.substring(5);
				break;
			}
		}
		// Defaulting to 10 cpi, 6 lpi...
		front_end = new Diablo630(props, argv, new PrinterInput());
		Thread t = new Thread(this);
		t.start();
	}

	class PrinterInput extends InputStream {
		public int read() {
			try {
				return fifo.take() & 0xff;
			} catch (Exception ee) {
				return -1;
			}
		}
		public int available() {
			return fifo.size();
		}
	}

	public void write(int b) {
		fifo.add(b);
	}

	public void run() {
		front_end.runPrinter(file);
		System.err.println("Diablo 630 detaching");
	}
}
