// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Vector;
import java.util.Properties;
import java.awt.event.*;
import javax.swing.*;

public class KayproFloppy extends WD1793
		implements DiskController, GppListener, WD1793Controller {
	// TODO: These need to come from CPU (Z80)...
	static final int EI = 0xfb;
	static final int RST6 = 0xf7;

	static final int BasePort_c = 0x10;
	static final int Wd1793_Offset_c = 0;
	static final int KayproFloppy_NumPorts_c = 4;
	static final String KayproFloppy_Name_c = "Floppy";

	static final int ctrl_DS1N_c = 0x01;
	static final int ctrl_DS2N_c = 0x02;
	static final int ctrl_DSxN_c = (ctrl_DS1N_c | ctrl_DS2N_c);
	static final int ctrl_Side_c = 0x04;
	static final int ctrl_Motor_c = 0x10;
	static final int ctrl_SetMFMRecordingN_c = 0x20;

	// Differences for models II and 4/83
	static final int ctr2_Motor_c = 0x40;
	static final int ctr2_Side_c = 0x00;	// no side-select...

	private int controlReg_m = 0;
	private GenericFloppyDrive[] drives_m;
	private LED[] leds_m;
	private Interruptor intr;
	private int numDisks_m = 2;
	private int interesting;
	private boolean HDDrives = false;
	private int xor = 0;
	private int motorBit = ctrl_Motor_c;
	private int sselBit = ctrl_Side_c;

	public KayproFloppy(Properties props, LEDHandler lh,
			Interruptor intr, GeneralPurposePort gpio, int numDrives) {
		super(BasePort_c + Wd1793_Offset_c, intr);
		super.setController(this);
		this.intr = intr;
		Interruptor.Model mod = intr.getModel();
		boolean HD = (mod == Interruptor.Model.K12X ||
				mod == Interruptor.Model.K4X ||
				mod == Interruptor.Model.KROBIE);
		if (mod == Interruptor.Model.K2 ||
				mod == Interruptor.Model.K4) {
			motorBit = ctr2_Motor_c;
			if (mod == Interruptor.Model.K2) {
				sselBit = ctr2_Side_c;
			}
			xor = (ctr2_Motor_c | sselBit | ctrl_DSxN_c);
		}
		interesting = ctrl_DSxN_c | sselBit |
			motorBit | ctrl_SetMFMRecordingN_c;
		// Caller must know if SASI is installed, and set numDrives
		if (numDrives > 4) {
			numDrives = 4;
		}
		if (numDrives < 2) {
			interesting &= ~ctrl_DS2N_c;
		}
		numDisks_m = numDrives;
		leds_m = new LED[numDisks_m];
		drives_m = new GenericFloppyDrive[numDisks_m];
		String s;
		Arrays.fill(drives_m, null);
		Arrays.fill(leds_m, null);
		// TODO: force all drives same type...
		HDDrives = HD;

		// First identify what drives are installed.
		int n = 0;
		GenericFloppyDrive drv;
		for (int x = 0; x < numDisks_m; ++x) {
			String prop = String.format("floppy_drive%d", x + 1);
			s = props.getProperty(prop);
			if (s != null) {
				++n;
				drv = installDrive(x, s);
				// TODO: validate with 'HD'
				if (lh != null && drv != null) {
					leds_m[x] = lh.registerLED(drv.getDriveName());
				}
			}
		}
		if (n <= 0) {
			if (HD) {
				s = "FDD_5_25_DS_QT";
			} else if (mod == Interruptor.Model.K2) {
				s = "FDD_5_25_SS_ST";
			} else {
				s = "FDD_5_25_DS_ST";
			}
			for (int x = 0; x < numDisks_m; ++x) {
				drv = installDrive(x, s);
				if (lh != null && drv != null) {
					leds_m[x] = lh.registerLED(drv.getDriveName());
				}
			}
		}

		// Next identify what diskettes are pre-inserted.
		for (int x = 0; x < numDisks_m; ++x) {
			String prop = String.format("floppy_disk%d", x + 1);
			s = props.getProperty(prop);
			if (s != null) {
				drv = getDrive(x);
				if (drv != null) {
					drv.insertDisk(SectorFloppyImage.getDiskette(drv,
						new Vector<String>(
							Arrays.asList(s.split("\\s")))));
				}
			}
		}
		reset();
		gpio.addGppListener(this);
	}

	private GenericFloppyDrive installDrive(int x, String s) {
		String[] ss = s.split("\\s", 2);
		String n = getDriveName(x);
		if (ss.length >= 2) {
			n = ss[1];
		}
		GenericFloppyDrive drv = GenericFloppyDrive.getInstance(ss[0], n);
		if (!connectDrive(x, drv)) {
			return null;
		}
		return drv;
	}

	public int getBaseAddress() { return BasePort_c; }
	public int getNumPorts() { return KayproFloppy_NumPorts_c; }

	private int driveNum(int val) {
		// TODO: must not select "B" if SASI is installed...
		// ctrl_DS2N_c is used as SASI reset/select
		if (numDisks_m > 2) {
			// drive A = 10 -> 00
			// drive B = 01 -> 01
			// drive C = 00 -> 10
			// drive D = 11 -> 11
			int drv = val | ~ctrl_DSxN_c;	// ensure no borrow
						// 2 1 0 3
			drv ^= ctrl_DSxN_c;	// 1 2 3 0
			drv -= 1;		// 0 1 2 3
			return (drv & ctrl_DSxN_c);
		}
		if ((val & ctrl_DS1N_c) == 0) {
			return 0;
		}
		if (numDisks_m > 1 && (val & ctrl_DS2N_c) == 0) {
			return 1;
		}
		return -1;
	}

	private void setCtrlReg(int val) {
		int prev = driveNum(controlReg_m);
		int diff = controlReg_m ^ val;
		controlReg_m = val;
		int next = driveNum(controlReg_m);
		if ((diff & ctrl_DSxN_c) != 0) {
			if (prev >= 0 && leds_m[prev] != null) {
				leds_m[prev].set(false);
			}
			//System.err.format("Drive select OFF\n");
			if (next >= 0 && leds_m[next] != null) {
				leds_m[next].set(true);
			}
			//System.err.format("Drive select %d\n", next);
		}
		if (next >= 0 && drives_m[next] != null) {
			drives_m[next].selectSide((sselBit != 0 &&
				(controlReg_m & sselBit) == 0) ? 1 : 0);
			drives_m[next].motor((controlReg_m & motorBit) != 0);
		}
		
	}

	public GenericFloppyDrive getCurDrive() {
		int n = driveNum(controlReg_m);
		if (n >= 0) {
			// This could be null
			return drives_m[n];
		} else {
			return null;
		}
	}

	public int getClockPeriod() {
		return 1000;
	}

	public Vector<GenericDiskDrive> getDiskDrives() {
		Vector<GenericDiskDrive> drives = new Vector<GenericDiskDrive>();
		for (int x = 0; x < numDisks_m; ++x) {
			drives.add(drives_m[x]);	// might be null, but must preserve number.
		}
		return drives;
	}

	public int getDriveSize(int index) {
		return 5;
	}

	public String getDriveName(int index) {
		if (index < 0 || index >= numDisks_m) {
			return null;
		}
		String str = String.format("%s-%d", KayproFloppy_Name_c, index + 1);
		return str;
	}

	public String getDeviceName() { return KayproFloppy_Name_c; }

	public GenericDiskDrive findDrive(String ident) {
		for (int x = 0; x < numDisks_m; ++x) {
			if (drives_m[x] == null) {
				continue;
			}
			if (ident.equals(drives_m[x].getDriveName())) {
				return drives_m[x];
			}
		}
		return null;
	}

	public void destroy() {
		for (int x = 0; x < numDisks_m; ++x) {
			if (drives_m[x] != null) {
				drives_m[x].insertDisk(null);
			}
		}
	}

	public void reset() {
		super.reset();
	}

	public int in(int addr) {
		int val = 0;
		val = super.in(addr);
		if (addr == 0x10 && !HDDrives) {
			val &= ~stat_NotReady_c; // always READY
		}
		return val;
	}

	public void out(int addr, int val) {
		super.out(addr, val);
	}

	public GenericFloppyDrive getDrive(int unitNum) {
		if (unitNum < numDisks_m) {
			return drives_m[unitNum];
		}
		return null;
	}

	public boolean connectDrive(int unitNum, GenericFloppyDrive drive) {
		if (unitNum >= numDisks_m) {
			System.err.format("KayproFloppy Invalid unit number (%d)\n", unitNum);
			return false;
		}
		if (drives_m[unitNum] != null) {
			System.err.format("KayproFloppy %d: drive already connect\n", unitNum);
			return false;
		}
		if (getDriveSize(unitNum) != drive.getMediaSize()) {
			System.err.format("KayproFloppy %d: drive incompatible\n", unitNum);
			return false;
		}
		drives_m[unitNum] = drive;
		return true;
	}

	public boolean removeDrive(int unitNum) {
		return false;
	}

	public void raisedIntrq() {
		// TODO: ensure it was OFF?
		intr.setNMI(true);
	}

	public void raisedDrq() {
		// TODO: ensure it was OFF?
		intr.setNMI(true);
	}

	public void loweredIntrq() {
		if (!drqRaised_m) {
			intr.setNMI(false);
		}
	}

	public void loweredDrq() {
		if (!intrqRaised_m) {
			intr.setNMI(false);
		}
	}

	public void loadedHead(boolean load) {
	}

	// returns current density relative to std 5" SD (FM @ 1MHz)
	public int densityFactor() {
		int dd = (controlReg_m & ctrl_SetMFMRecordingN_c) == 0 ? 1 : 0;
		if (HDDrives) {
			if ((controlReg_m & motorBit) != 0) {
				// high speed... lower density...
			} else {
				// low speed... higher density...
				++dd;
			}
		}
		return dd + 1;	// => 1,2,3
	}

	public int interestedBits() {
		return interesting;
	}

	public void gppNewValue(int gpio) {
		gpio ^= xor;
		setCtrlReg(gpio);
	}

	public String dumpDebug() {
		String ret = String.format(
			"FDC-STS=%02x FDC-CMD=%02x\n" +
			"FDC-TRK=%d FDC-SEC=%d FDC-DAT=%02x\n" +
			"DRQ=%s INTRQ=%s HLD=%s\n",
			statusReg_m, cmdReg_m,
			trackReg_m, sectorReg_m, dataReg_m,
			Boolean.toString(drqRaised_m), Boolean.toString(intrqRaised_m),
			Boolean.toString(headLoaded_m));
		ret += String.format("SIDE=%d DDEN=%s MOTOR=%s\n",
			(sselBit != 0 && (controlReg_m & sselBit) == 0) ? 1 : 0,
			(controlReg_m & ctrl_SetMFMRecordingN_c) == 0,
			(controlReg_m & motorBit) != 0);
		return ret;
	}
}
