// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.io.*;
import java.util.Properties;

public class KayproRoms {
	protected byte[] mon; // 2K or 4K at 0x0000
	protected int monMask;

	public KayproRoms(Properties props, String defRom) {
		InputStream fi = null;
		String s = props.getProperty("monitor_rom");
		if (s == null) {
			System.err.format("No Monitor ROM specified, using %s\n", defRom);
			s = defRom;
		}
		try {
			fi = new FileInputStream(s);
		} catch (FileNotFoundException nf) {
			try {
				fi = this.getClass().getResourceAsStream(s);
			} catch (Exception ee) {
				ee.printStackTrace();
				System.exit(1);
			}
		} catch (Exception ee) {
			ee.printStackTrace();
			System.exit(1);
		}
		try {
			// TODO: check for power-of-two and viable ROM sizes
			monMask = fi.available() - 1;
			mon = new byte[fi.available()];
			fi.read(mon);
			fi.close();
		} catch (Exception ee) {
			ee.printStackTrace();
			System.exit(1);
		}
	}
}
