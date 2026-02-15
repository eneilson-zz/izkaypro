// Copyright (c) 2016 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.Properties;
import java.util.Vector;
import java.awt.event.*;
import java.io.*;

public class WD1002_05 implements IODevice, GppListener, GenericDiskDrive,
			DiskController, ActionListener {
	static final int INVALID = 0;
	static final int ST506 = 1;	//  5.3MB (Seagate ST506 drive)
	static final int ST412 = 2;	// 10.6MB
	static final int CM5206 = 3;	//  4.4MB
	static final int CM5410 = 4;	//  8.9MB
	static final int CM5616 = 5;	// 13.3MB
	static final int RO201 = 6;	//  5.5MB
	static final int RO202 = 7;	// 11.1MB
	static final int RO203 = 8;	// 16.7MB
	static final int RO204 = 9;	// 22.3MB
	static final int other_other = 10;	// not supported
	static final int NUM_DRV_TYPE = 11;

	static final int adr_Data_c = 0;
	static final int adr_Error_c = 1;
	static final int adr_Precomp_c = 1;
	static final int adr_SecCnt_c = 2;
	static final int adr_Sector_c = 3;
	static final int adr_CylLo_c = 4;
	static final int adr_CylHi_c = 5;
	static final int adr_SDH_c = 6;
	static final int adr_Status_c = 7;
	static final int adr_Cmd_c = 7;

	public static final int sts_Busy_c = 0x80;
	public static final int sts_Ready_c = 0x40;
	public static final int sts_WriteFault_c = 0x20;
	public static final int sts_SeekDone_c = 0x10;
	public static final int sts_Drq_c = 0x08;
	public static final int sts_Corr_c = 0x04;
	public static final int sts_Error_c = 0x01;

	public static final int err_BadBlock_c = 0x80;
	public static final int err_Uncorr_c = 0x40;
	public static final int err_CRC_c = 0x20;
	public static final int err_ID_nf_c = 0x10;
	public static final int err_Aborted_c = 0x04;
	public static final int err_TR000_c = 0x02;
	public static final int err_DAM_nf_c = 0x01;

	// after reset... diagnostic errors
	public static final int dia_WD1015_err_c = 0x05;
	public static final int dia_WD1014_err_c = 0x04;
	public static final int dia_Buffer_err_c = 0x03;
	public static final int dia_WD1010_err_c = 0x02;
	public static final int dia_WD2797_err_c = 0x01;
	public static final int dia_Pass_c = 0x00;

	// SystemPort bits
	static final int ctrl_Reset_c = 0x02;	// active low

	static final int[] sszs = new int[]{ 256, 512, 1024, 128 };

	// WD1002-05 Command codes
	static final byte cmd_Restore_c = 0x10;
	static final byte cmd_Seek_c = 0x70;
	static final byte cmd_Read_c = 0x20;
	static final byte cmd_Write_c = 0x30;
	static final byte cmd_FormatTrack_c = 0x50;

	// Command augment bits
	static final byte cmd_Intr_c = 0x08;		// READ
	static final byte cmd_MultiSec_c = 0x04;	// READ, WRITE
	static final byte cmd_Long_c = 0x02;		// READ, WRITE
	static final byte cmd_StepRate_c = 0x0f;	// SEEK, RESTORE

	// State Machine
	// Read:
	//	cmd_Read_c	sts_Busy_c
	//			!sts_Busy_c, sts_Drq_c
	//	(read data)	...
	//			!sts_Drq_c
	//
	// Write:
	//	cmd_Write_c	sts_Busy_c
	//			sts_Drq_c
	//	(write data)	...
	//			!sts_Busy_c, !sts_Drq_c

	private enum State {
		IDLE,
		COMMAND,
		DATA_IN,
		DATA_OUT,
		SENSE,
		DRIVECB,
		STATUS,
	};
	private State curState;

	private RandomAccessFile driveFd;
	private Interruptor intr;
	private LED leds_m;
	private javax.swing.Timer timer;
	private int driveSecLen;
	private int sectorsPerTrack;
	private long capacity;
	private int driveCode;
	private int driveCnum;
	private String driveMedia;
	private int driveType;
	private long mediaSpt;
	private long mediaSsz;
	private long mediaCyl;
	private long mediaHead;
	private long mediaLat;
	private boolean formatted;

	// mode COMMAND
	private byte[] cmdBuf = new byte[8];
	private byte curCmd;
	private byte preComp;
	// mode DATA_IN/DATA_OUT
	private byte[] dataBuf;
	private int dataLength;
	private int dataIx;
	private long wrOff;

	private String name;

	// Expected drive characteristic by drive: (XEBEC S1410 manual)
	// Drive    cyl  hd red-wr precomp
	// CM5206   256  2  256    256
	// CM5410   256  4  256    256
	// CM5616   256  6  256    256
	// HD561    180  2  128    180
	// HD562    180  2  128    180
	// RMS503   153  2  77     77
	// RMS506   153  4  77     77
	// RMS512   153  8  77     77
	// ST506    153  4  128    64
	// ST412    306  4  128    64	*** normal Kaypro 10 drive ***
	// TM602S   153  4  128    153
	// TM603S   153  6  128    153
	// TM603SE  230  6  128    128
	// TI5.25+  153  4  64     64
	// RO101    192  2  96     0
	// RO102    192  4  96     0
	// RO103    192  6  96     0
	// RO104    192  8  96     0
	// RO201    321  2  132    0
	// RO202    321  4  132    0
	// RO203    321  6  132    0
	// RO204    321  8  132    0
	// MS1-006  306  2  153    0
	// MS1-012  306  4  153    0
	//
	private int[][] params = new int[][] {
		{ 0, 0, 0, 0 },		// /* INVALID=0    */
		{ 153, 4, 128, 64 },	// /*[ST506] */
		{ 306, 4, 128, 64 },	// /*[ST412] */
		{ 256, 2, 256, 256 },	// /*[CM5206]*/
		{ 256, 4, 256, 256 },	// /*[CM5410]*/
		{ 256, 6, 256, 256 },	// /*[CM5616]*/
		{ 321, 2, 132, 0 },	// /*[RO201] */
		{ 321, 4, 132, 0 },	// /*[RO202] */
		{ 321, 6, 132, 0 },	// /*[RO203] */
		{ 321, 8, 132, 0 },	// /*[RO204] */
	};

	// TODO: handle multiple drives
	// TODO: handle programmable sector size?
	WD1002_05(Properties props, LEDHandler lh,
			Interruptor intr, GeneralPurposePort gpio) {
		this.intr = intr;
		
		driveFd = null;
		driveSecLen = 0;
		sectorsPerTrack = 0;
		capacity = 0;
		driveCode = 0;
		driveCnum = 0;
		driveMedia = null;
		driveType = INVALID;
		mediaSpt = 0;
		mediaSsz = 0;
		mediaCyl = 0;
		mediaHead = 0;
		mediaLat = 0;
		dataBuf = null;
		dataLength = 0;
		dataIx = 0;

		int sectorSize = 512;

		// NOTE: Kaypro 10 seems to expect LUN 1, not 0.
		// TODO: support more than one LUN
		timer = new javax.swing.Timer(500, this);
		name = "WD1002-1";
		driveMedia = name;
		String s = props.getProperty("wd1002_drive1");
		if (s != null) {
			String[] ss = s.split("\\s", 2);
			driveType = getType(ss[0]);
			if (ss.length > 1) {
				// Front panel name, not media name
				name = ss[1];
			}
		} else {
			driveType = ST412;
		}
		s = props.getProperty("wd1002_disk1");
		if (s != null) {
			// TODO: any parameters follow?
			driveMedia = s;
		}
		// Always show drive on front panel, even if not usable
		leds_m = lh.registerLED(name);
		if (driveType == INVALID) {
			return;
		}
		driveSecLen = sectorSize;

		if (sectorSize == 256) {
			sectorsPerTrack = 32;
		} else if (sectorSize == 512) {
			sectorsPerTrack = 17;
		} else {
			// not supported, but do something...
			if (sectorSize == 0) {
				sectorsPerTrack = 0;
			} else {
				sectorsPerTrack = 8192 / sectorSize;
			}
		}

		capacity = params[driveType][0] * params[driveType][1] * sectorsPerTrack * sectorSize;
		dataBuf = new byte[sectorSize + 4];	// space for ECC for "long" commands
		dataLength = sectorSize;

		RandomAccessFile fd;
		try {
			fd = new RandomAccessFile(driveMedia, "rw");
			byte[] buf = new byte[128];
			if (fd.length() == 0) {
				// special case: new media - initialize it.
				mediaCyl = params[driveType][0];
				mediaHead = params[driveType][1];
				mediaSsz = sectorSize;
				mediaSpt = sectorsPerTrack;
				mediaLat = 1;
				formatted = false;
				Arrays.fill(buf, (byte)0);
				String hdr = String.format("%dc%dh%dz%dp%dl\n",
					mediaCyl, mediaHead, mediaSsz, mediaSpt, mediaLat);
				System.arraycopy(hdr.getBytes(), 0, buf, 0, hdr.length());
				// NOTE: 'buf' includes a '\n'...
				System.err.format("Initializing new media %s as %s", driveMedia, hdr);
				fd.setLength(capacity + buf.length);
				fd.seek(capacity);	// i.e. END - sizeof(buf)
				fd.write(buf);
			} else {
				formatted = true;
				fd.seek(fd.length() - buf.length);
				int n = fd.read(buf);
				if (n != buf.length || (n = checkHeader(buf)) < 0) {
					System.err.format("WD1002_05: file %s is not GenericSASIDrive\n", driveMedia);
					return;
				}
				if (mediaSpt != sectorsPerTrack ||
						mediaSsz != driveSecLen ||
				    		params[driveType][0] != mediaCyl ||
				    		params[driveType][1] != mediaHead) {
					// TODO: fatal or warning?
					System.err.format("Media/Drive mismatch: %s\n", driveMedia);
					return;
				}
				System.err.format("Mounted existing media %s as %s\n", driveMedia, new String(buf, 0, n));
			}
		} catch (Exception ee) {
			System.err.format("WD1002_05: Unable to open media %s\n", driveMedia);
			return;
		}
		driveFd = fd;
		gpio.addGppListener(this);
	}

	private int getType(String type) {
		int etype = INVALID;
		if (type.equals("ST506")) {
			etype = ST506;
		} else if (type.equals("ST412")) {
			etype = ST412;
		} else if (type.equals("CM5206")) {
			etype = CM5206;
		} else if (type.equals("CM5410")) {
			etype = CM5410;
		} else if (type.equals("CM5616")) {
			etype = CM5616;
		} else if (type.equals("RO201")) {
			etype = RO201;
		} else if (type.equals("RO202")) {
			etype = RO202;
		} else if (type.equals("RO203")) {
			etype = RO203;
		} else if (type.equals("RO204")) {
			etype = RO204;
		} else {
			// System.err.format("Invalid drive designation: %s\n", type);
		}
		return etype;
	}

	private int checkHeader(byte[] buf) {
		int m = 0;
		int e = 0;
		while (buf[e] != '\n' && buf[e] != '\0' && e < buf.length) {
			int p = 0;
			while (e < buf.length && Character.isDigit(buf[e])) {
				p = (p * 10) + (buf[e] - '0');
				++e;
			}
			if (e >= buf.length) {
				// too harsh?
				return -1;
			}
			// TODO: removable flag, others?
			// NOTE: removable media requires many more changes.
			switch (Character.toLowerCase(buf[e])) {
			case 'c':
				if (p > 0x0ffff) {
					break;
				}
				m |= 0x01;
				mediaCyl = p & 0x0ffff;
				break;

			case 'h':
				if (p > 0x0f) {
					break;
				}
				m |= 0x02;
				mediaHead = p & 0x0f;
				break;

			case 'z':
				if (p != 128 && p != 256 && p !=512 && p != 1024) {
					break;
				}
				m |= 0x04;
				mediaSsz = p & 0x0fff;
				break;

			case 'p':
				if (p > 0x0ff) {
					break;
				}
				m |= 0x08;
				mediaSpt = p & 0x0ff;
				break;

			case 'l':
				if (p > 0x0ff) {
					break;
				}
				m |= 0x10;
				mediaLat = p & 0x0ff;
				break;

			default:
				return -1;
			}

			e++;
		}
		return m == 0x1f ? e : -1;
	}

	private int getCyl() {
		return ((cmdBuf[adr_CylHi_c] & 0xff) << 8) | (cmdBuf[adr_CylLo_c] & 0xff);
	}

	private int getSec() {
		return cmdBuf[adr_Sector_c] & 0xff;
	}

	private int getCount() {
		return cmdBuf[adr_SecCnt_c] & 0xff;
	}

	private int getHead() {
		return cmdBuf[adr_SDH_c] & 0x07;
	}

	private int getLUN(byte b) {
		return (b & 0x18) >> 3;
	}

	private int getLUN() {
		return getLUN(cmdBuf[adr_SDH_c]);
	}

	private int getSSZ() {
		return sszs[(cmdBuf[adr_SDH_c] & 0x60) >> 5];
	}

	private long getOff() {
		long off = (getCyl() * mediaHead + getHead()) * sectorsPerTrack + getSec();
		off *= driveSecLen;
		return off;
	}

	public int interestedBits() {
		return ctrl_Reset_c;
	}

	public void gppNewValue(int gpio) {
		if ((gpio & ctrl_Reset_c) != 0) {
			reset(); // something else? more?
		} else {
			// technically, we shouldn't reset until here...
			// or at least should not accept anything until now.
		}
	}

	public GenericDiskDrive findDrive(String name) {
		if (name.equals(this.name)) {
			return this;
		}
		return null;
	}
	public Vector<GenericDiskDrive> getDiskDrives() {
		// Only return REMOVABLE drives...
		Vector<GenericDiskDrive> drives = new Vector<GenericDiskDrive>();
		//drives.add(this);
		return drives;
	}

	public int getBaseAddress() { return 0x80; }
	public int getNumPorts() { return 8; }

	public int in(int port) {
		port &= 7;
		int val = cmdBuf[port] & 0xff;
		switch(port) {
		case adr_Data_c:
			getData();
			break;
		case adr_Error_c:
			// TODO: reset bits?
			break;
		case adr_Status_c:
			// TODO: reset bits?
			//cmdBuf[adr_Status_c] &= ~sts_Error_c;
			break;
		}
		//System.err.format("WD1002_05 in %02x %02x\n", port, val);
		return val;
	}

	public void out(int port, int val) {
		port &= 7;
		//System.err.format("WD1002_05 out %02x %02x\n", port, val);
		switch(port) {
		case adr_Precomp_c:
			// anything?
			preComp = (byte)val;
			return;
		case adr_Cmd_c:
			//cmdBuf[adr_Status_c] &= ~sts_SeekDone_c;
			curCmd = (byte)val;
			cmdBuf[adr_Status_c] &= ~sts_Error_c;
			cmdBuf[adr_Error_c] = 0;
			processCmd();
			return;
		case adr_Data_c:
			putData(val);
			break;
		case adr_SDH_c:
			// NOTE: Kaypro 10 seems to expect LUN 1, not 0.
			// TODO: support more than one LUN
			if (getLUN((byte)val) == 1) {
				cmdBuf[adr_Status_c] |= sts_Ready_c;
			} else {
				cmdBuf[adr_Status_c] &= ~sts_Ready_c;
			}
			break;
		}
		cmdBuf[port] = (byte)val;
	}

	public void reset() {
		dataIx = 0;
		Arrays.fill(cmdBuf, (byte)0);
		curCmd = 0;
		if (driveFd != null) {
			cmdBuf[adr_Status_c] |= sts_Ready_c;
			cmdBuf[adr_Status_c] |= sts_SeekDone_c;
		}
		// On RESET, self-test (diags) are run.
		// Results are reported in error register, w/o status err bit.
		// Kaypro software (HDFMT) expects this error,
		// As the model of controller used has no floppy chip.
		cmdBuf[adr_Error_c] = dia_WD2797_err_c;
	}

	private void putData(int val) {
		if (dataIx < dataLength) {
			dataBuf[dataIx++] = (byte)val;
			if (dataIx < dataLength) {
				cmdBuf[adr_Status_c] |= sts_Drq_c;
			} else {
				processData();
			}
			return;
		}
		setDone();
	}

	private void getData() {
		if (dataIx < dataLength) {
			cmdBuf[adr_Data_c] = dataBuf[dataIx++];
			cmdBuf[adr_Status_c] |= sts_Drq_c;
			return;
		}
		dataIx = 0;
		if ((curCmd & cmd_MultiSec_c) != 0) {
			if (cmdBuf[adr_SecCnt_c] > 0) {
				--cmdBuf[adr_SecCnt_c];
				++cmdBuf[adr_Sector_c];
				cmdBuf[adr_Sector_c] %= mediaSpt;
				// TODO: carry over to cylinder? head?
			}
			if (cmdBuf[adr_SecCnt_c] > 0) {
				processCmd();
				return;
			}
		}
		setDone();
	}

	private void setDone() {
		cmdBuf[adr_Status_c] &= ~sts_Drq_c;
		cmdBuf[adr_Status_c] &= ~sts_Busy_c;
	}

	private void setError() {
		cmdBuf[adr_Status_c] |= sts_Error_c;
		setDone();
	}

	private void dumpIO(String op, long off) {
		System.err.format("%s at %d (%d %d %d %d):",
			op, off, getLUN(), getCyl(), getHead(), getSec());
		for (int x = 0; x < 16; ++x) {
			System.err.format(" %02x", dataBuf[x]);
		}
		System.err.format("\n");
	}

	private void processCmd() {
		long off;
		long e;

		if (driveFd == null) {
			setError();
			return;
		}
		// NOTE: Kaypro 10 seems to expect LUN 1, not 0.
		if (getLUN() != 1) {
			setError();
			return;
		}
		timer.removeActionListener(this);
		timer.addActionListener(this);
		timer.restart();
		// TODO: minimize redundant calls?
		leds_m.set(true);
		switch (curCmd & 0xf0) {
		case cmd_Restore_c:
			// no-op: just return success
			cmdBuf[adr_Status_c] |= sts_SeekDone_c;
			break;
		case cmd_Read_c:
			off = getOff();
			if (off >= capacity) {
				setError();
				break;
			}
			// Drives are shipped formatted?
			//if (!formatted) {
			//	// TODO: is this err_ID_nf_c or err_DAM_nf_c?
			//	cmdBuf[adr_Error_c] |= err_ID_nf_c;
			//	setError();
			//	break;
			//}
			try {
				driveFd.seek(off);
				// dataBuf includes (fake) ECC, must limit read()...
				e = driveFd.read(dataBuf, 0, driveSecLen);
				if (e != driveSecLen) {
					setError();
					break;
				}
			} catch (Exception ee) {
				setError();
				break;
			}
			dataLength = driveSecLen;
			if ((curCmd & cmd_Long_c) != 0) {
				// TODO: must we compute ECC?
				dataBuf[dataLength++] = 0;
				dataBuf[dataLength++] = 0;
				dataBuf[dataLength++] = 0;
				dataBuf[dataLength++] = 0;
			}
			dataIx = 0;
			getData(); // prime first byte
			break;
		case cmd_Write_c:
			// Prepare for command... but must wait for data...
			wrOff = getOff();
			if (wrOff >= capacity) {
				setError();
				break;
			}
			dataLength = driveSecLen;
			if ((curCmd & cmd_Long_c) != 0) {
				dataLength += 4;
			}
			dataIx = 0;
			cmdBuf[adr_Status_c] |= sts_Drq_c;
			cmdBuf[adr_Status_c] |= sts_Busy_c;
			break;
		case cmd_Seek_c:
			cmdBuf[adr_Status_c] |= sts_SeekDone_c;
			// validate address, but otherwise just return success.
			off = getOff();
			if (off >= capacity) {
				setError();
				break;
			}
			break;
		case cmd_FormatTrack_c:
			// validate address
			off = getOff();
			if (off >= capacity) {
				setError();
				break;
			}
			// set DRQ, get data (ignored), "format" track
			// (zero data?)
			dataIx = 0;
			cmdBuf[adr_Status_c] |= sts_Drq_c;
			cmdBuf[adr_Status_c] |= sts_Busy_c;
			break;
		default:
			break;
		}

	}

	private void processData() {
		long off;
		long e;

		switch (curCmd & 0xf0) {
		case cmd_Write_c:
			// Drives are shipped formatted?
			//if (!formatted) {
			//	cmdBuf[adr_Error_c] |= err_ID_nf_c;
			//	setError();
			//	break;
			//}
			//off = getOff();
			off = wrOff;
			try {
				driveFd.seek(off);
				// dataBuf includes (fake) ECC, must limit write()...
				driveFd.write(dataBuf, 0, driveSecLen);
			} catch (Exception ee) {
				setError();
				break;
			}
			dataIx = 0;
			if ((curCmd & cmd_MultiSec_c) != 0) {
				if (cmdBuf[adr_SecCnt_c] > 0) {
					--cmdBuf[adr_SecCnt_c];
					++cmdBuf[adr_Sector_c];
					cmdBuf[adr_Sector_c] %= mediaSpt;
					// TODO: carry over to cylinder? head?
				}
				if (cmdBuf[adr_SecCnt_c] > 0) {
					cmdBuf[adr_Status_c] |= sts_Drq_c;
					cmdBuf[adr_Status_c] |= sts_Busy_c;
					break;
				}
			}
			setDone();
			break;
		case cmd_FormatTrack_c:
			// sector redirection table - ignored
			// TODO: track-by-track, or just once for all?
			formatted = true;
			setDone();
			break;
		default:
			break;
		}
	}

	public void actionPerformed(ActionEvent e) {
		if (e.getSource() != timer) {
			return;
		}
		timer.removeActionListener(this);
		leds_m.set(false);
	}

	public String getMediaName() {
		return driveMedia != null ? driveMedia : "";
	}

	public boolean isReady() {
		return driveFd != null;
	}

	public void insertDisk(GenericFloppyDisk disk) { }
	public int getRawBytesPerTrack() { return 0; }
	public int getNumTracks() { return 0; }
	public int getNumHeads() { return 0; } 
	public int getMediaSize() { return 0; }
	public boolean isRemovable() { return false; }

	public String getDriveName() {
		return name;
	}

	public String getDeviceName() { return "WD1002-05"; }

	public String dumpDebug() {
		String ret = String.format(
			"[0] data    %02x\n" +
			"[1] error   %02x %02x (precomp)\n" +
			"[2] sec cnt %02x\n" +
			"[3] sector  %02x\n" +
			"[4] cyl lo  %02x\n" +
			"[5] cyl hi  %02x\n" +
			"[6] S/D/H   %02x\n" +
			"[7] status  %02x %02x (cmd)\n" +
			"data index = %d\n",
			cmdBuf[0] & 0xff,
			cmdBuf[1] & 0xff, preComp & 0xff,
			cmdBuf[2] & 0xff,
			cmdBuf[3] & 0xff,
			cmdBuf[4] & 0xff,
			cmdBuf[5] & 0xff,
			cmdBuf[6] & 0xff,
			cmdBuf[7] & 0xff, curCmd & 0xff,
			dataIx);
		return ret;
	}
}
