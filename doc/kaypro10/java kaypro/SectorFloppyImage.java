// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.io.*;
import java.util.Arrays;
import java.util.Vector;

public class SectorFloppyImage implements GenericFloppyDisk {

	private String imageName_m;
	private RandomAccessFile imageFd_m;
	private byte[] secBuf_m;
	private int bufferedTrack_m;
	private int bufferedSide_m;
	private int bufferedSector_m;
	private long bufferOffset_m;
	private boolean bufferDirty_m;
	private int hypoTrack_m;	// ST media in DT drive
	private int hyperTrack_m;	// DT media in ST drive
	private boolean interlaced_m;
	private int dsa_m;	// double-sided algorithm...
	private int mediaLat_m;
	private int secLenCode_m;
	private int gapLen_m;
	private int indexGapLen_m;
	private long writePos_m;
	private int dataPos_m;
	private int dataLen_m;

	private boolean writeProtect_m;
	private int densityFactor_m;
	private long trackLen_m;
	private int numTracks_m;
	private int numSectors_m;
	private int numSides_m;
	private int secSize_m;
	private int mediaSize_m;
	private int mediaSec_m; // > 0 = hard-sectored media (num sectors)

	// TODO: If constructor fails, the drive should not mount this disk!
	public SectorFloppyImage(GenericDiskDrive drive, Vector<String> argv) {
		imageName_m = null;
		imageFd_m = null;
		secBuf_m = null;
		bufferedTrack_m = -1;
		bufferedSide_m = -1;
		bufferedSector_m = -1;
		bufferOffset_m = 0;
		hypoTrack_m = 0;
		hyperTrack_m = 0;
		interlaced_m = false;
		dsa_m = 0;
		mediaLat_m = 0;
		mediaSec_m = 0;
		secLenCode_m = 0;
		gapLen_m = 0;
		indexGapLen_m = 0;
		writePos_m = 0;
		dataPos_m = 0;
		dataLen_m = 0;
		writeProtect_m = true;
		densityFactor_m = 0;
		numTracks_m = 0;
		numSectors_m = 0;
		numSides_m = 0;
		secSize_m = 0;
		mediaSize_m = 0;

		if (argv.size() < 1) {
			System.err.format("SectorFloppyImage: no file specified\n");
			return;
		}
		File media = new File(argv.get(0));
		for (int x = 1; x < argv.size(); ++x) {
			if (argv.get(x).equals("rw")) {
				writeProtect_m = false;
			}
		}
		if (!writeProtect_m && !media.canWrite()) {
			System.err.format("Image not writeable: %s\n", argv.get(0));
			writeProtect_m = true;
		}
		RandomAccessFile fd;
		try {
			fd = new RandomAccessFile(media, writeProtect_m ? "r" : "rw");
			byte[] buf = new byte[128];
			fd.seek(fd.length() - buf.length);
			int n = fd.read(buf);
			if (n != buf.length || !checkHeader(buf)) {
				System.err.format("SectorFloppyImage: file %s is not SectorFloppyImage\n", argv.get(0));
				return;
			}
		} catch (Exception ee) {
			System.err.format("SectorFloppyImage: unable to open file - %s\n", argv.get(0));
			return;
		}
		if (drive.getNumTracks() > numTracks_m) {
			hypoTrack_m = (drive.getNumTracks() / numTracks_m);
		} else if (drive.getNumTracks() < numTracks_m) {
			hyperTrack_m = (numTracks_m / drive.getNumTracks());
		}
		trackLen_m = getTrackLen(densityFactor_m);
		secLenCode_m = Integer.numberOfTrailingZeros(secSize_m); // 128==7, 1024==10
		if (secLenCode_m < 11) {
			secLenCode_m -= 7;	// 128==0, 256=1, 512=2, 1024==3
			secLenCode_m &= 0x03;	// just to be sure.
		}
		if (mediaSec_m > 0 && mediaSec_m != numSectors_m) {
			System.err.format("Hard sectored disk sec/trk mismatch\n");
			mediaSec_m = numSectors_m;
		}
		secBuf_m = new byte[secSize_m];
		imageFd_m = fd;
		imageName_m = argv.get(0);
		System.err.format(
			"mounted %d\" floppy %s: sides=%d tracks=%d spt=%d DD=%s R%s\n",
			mediaSize_m, imageName_m, numSides_m, numTracks_m, numSectors_m,
			densityFactor_m > 1 ? "yes" : "no", writeProtect_m ? "O" : "W");
	}

	private int getTrackLen(int df) {
		int tl = (mediaSize_m == 5 ? 3200 : 6400);
		if (df > 1) {
			tl *= df;
		}
		return tl;
	}

	static GenericFloppyDisk getDiskette(GenericDiskDrive drv, Vector<String> argv) {
		GenericFloppyDisk gd = new SectorFloppyImage(drv, argv);
		if (!gd.isReady()) {
			return null;
		}
		return gd;
	}

	private boolean checkHeader(byte[] buf) {
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
				return false;
			}
			switch (Character.toLowerCase(buf[e])) {
			case 'm':
				if (p != 5 && p != 8) {
					break;
				}
				m |= 0x01;
				mediaSize_m = p;
				break;
			case 'z':
				if (p != 128 && p != 256 && p != 512 && p != 1024) {
					break;
				}
				m |= 0x02;
				secSize_m = p;
				break;
			case 'p':
				if (p == 0 || p > 0x0ff) {
					break;
				}
				m |= 0x04;
				numSectors_m = p;
				break;
			case 's':
				if (p < 1 || p > 2) {
					break;
				}
				m |= 0x08;
				numSides_m = p;
				break;
			case 't':
				if (p == 0 || p > 0x0ff) {
					break;
				}
				m |= 0x10;
				numTracks_m = p;
				break;
			case 'd':
				m |= 0x20;
				// convert 0, 1, 2 into 1, 2, 3
				densityFactor_m = p + 1;
				break;
			case 'i':
				m |= 0x40;
				interlaced_m = (p != 0);
				dsa_m = p;
				break;
			case 'l':	// optional ?
				// m |= 0x80;
				mediaLat_m = p;
				break;
			case 'h':	// optional ?
				// m |= 0x100;
				mediaSec_m = p;
				break;
			default:
				return false;
			}
			e++;
		}
		if (m != 0x7f) {
			// failed to interpret header
		}
		return m == 0x7f;
	}

	public void eject(String file) {
		cacheSector(-1, -1, -1);
		try {
			imageFd_m.close();
		} catch (Exception ee) {}
		imageFd_m = null;
	}

	private int mediaTrack(int trk) {
		if (hypoTrack_m > 0) {
			if ((trk % hypoTrack_m) != 0) {
				return GenericFloppyFormat.ERROR;
			}
			trk /= hypoTrack_m;
		} else if (hyperTrack_m > 0) {
			trk *= hyperTrack_m;
		}
		return trk;
	}

	private int mediaSide(int sid) {
		if (sid > 0 && sid >= numSides_m) {
			return GenericFloppyFormat.ERROR;
		}
		return sid;
	}

	// TODO: may need to translate sector number, if l-to-p table is used that way.
	boolean cacheSector(int side, int track, int sector) {
		if (bufferedSide_m == side && bufferedTrack_m == track && bufferedSector_m == sector) {
			return true;
		}
		if (imageFd_m == null) {
			return false;
		}
		if (bufferDirty_m && bufferedSide_m != -1 && bufferedTrack_m != -1 && bufferedSector_m != -1) {
			try {
				imageFd_m.seek(bufferOffset_m);
				imageFd_m.write(secBuf_m);
			} catch (Exception ee) {
				System.err.format("Cache flush failed, " +
						"trk %d sid %d sec %d, data lost\n", track, side, sector);
			}
		}
		bufferDirty_m = false;
		if (side < 0 || track < 0 || sector < 0) {
			return true;
		}
		track = mediaTrack(track);
		side = mediaSide(side);
		if (track < 0 || side < 0) {
			return false;
		}
		int secNum = sector;
		if (mediaSec_m == 0 && dsa_m != 2) {
			// TODO: need better way to know 1-based sectoring
			secNum -= 1;
		}
		if (dsa_m == 2 && side == 1) {
			// TODO: confirm secNum >= numSectors_m?
			secNum -= numSectors_m;
		}
		if (secNum >= numSectors_m) {
			// For multi-sector reads... end of track.
			return false;
		}
		if (interlaced_m) {
			bufferOffset_m = ((track * numSides_m + side) * numSectors_m + secNum) * secSize_m;

		} else {
			bufferOffset_m = ((side * numTracks_m + track) * numSectors_m + secNum) * secSize_m;
		}
		try {
			imageFd_m.seek(bufferOffset_m);
			int rd = imageFd_m.read(secBuf_m);
			if (rd != secSize_m) {
				bufferedSide_m = -1;
				bufferedTrack_m = -1;
				bufferedSector_m = -1;
				return false;
			}
		} catch (Exception ee) {
			System.err.format("Cache fill failed, " +
						"trk %d sid %d sec %d\n", track, side, sector);
			bufferedSide_m = -1;
			bufferedTrack_m = -1;
			bufferedSector_m = -1;
			return false;
		}
		bufferedSide_m = side;
		bufferedTrack_m = track;
		bufferedSector_m = sector;
		return true;
	}

	public int readData(int track, int side, int sector, int inSector) {
		int data = GenericFloppyFormat.NO_DATA;
		if (inSector < 0) {
			if (sector == 0xfd) {
				// Read Address - must validate track/side
				// Don't need to be on the same but must
				// have valid data on media there.
				if (mediaTrack(track) < 0 || mediaSide(side) < 0) {
					return GenericFloppyFormat.ERROR;
				}
				return GenericFloppyFormat.ID_AM;
			} else if (sector == 0xff) {
				return GenericFloppyFormat.INDEX_AM;
			} else if (cacheSector(side, track, sector)) {
				dataPos_m = 0;
				dataLen_m = dataPos_m + secSize_m;
				return GenericFloppyFormat.DATA_AM;
			}
			// For multi-sector read, "end of track"
			return GenericFloppyFormat.INDEX_AM;
		}
		if (sector == 0xfd) {
			switch (inSector) {
			case 0:
				data = mediaTrack(track);
				break;
			case 1:
				data = mediaSide(side);
				break;
			case 2:
				data = 1;	// anything will do? 'sector' is 0xfd...
				if (dsa_m == 2 && side == 1) {
					data += numSectors_m;
				}
				break;
			case 3:
				data = secLenCode_m;
				break;
			case 4:
				data = 0;	// CRC 1
				break;
			case 5:
				data = 0;	// CRC 2
				break;
			default:
				data = GenericFloppyFormat.CRC;
				break;
			}
			return data;
		} else if (sector == 0xff) {
			if (inSector < trackLen_m) {
				// TODO: implement this
				data = 0;
			} else {
				data = GenericFloppyFormat.CRC;
			}
			return data;
		}
		if (dataPos_m < dataLen_m) {
			data = secBuf_m[dataPos_m++] & 0xff;
		} else {
			data = GenericFloppyFormat.CRC;
			//System.err.format("data done %d %d %d (%d)\n", track, side, sector, data);
		}
		return data;
	}

	public int writeData(int track, int side, int sector, int inSector,
					  int data, boolean dataReady) {
		int result = GenericFloppyFormat.NO_DATA;
		if (checkWriteProtect()) {
			System.err.format("write protect\n");
			return GenericFloppyFormat.ERROR;
		}
		if (inSector < 0) {
			if ((sector & 0xfc) == 0xfc) {
				dataPos_m = 0;
				// some programs, like Heath CP/M FORMAT,
				// don't recognize a full track length so have to
				// avoid accepting all bytes.
				dataLen_m = getTrackLen(sector & 3) - 10;
				return GenericFloppyFormat.INDEX_AM;
			} else if (cacheSector(side, track, sector)) {
				dataPos_m = 0;
				dataLen_m = dataPos_m + secSize_m;
				return GenericFloppyFormat.DATA_AM;
			}
			// For multi-sector write, "end of track"
			return GenericFloppyFormat.INDEX_AM;
		}
		if ((sector & 0xfc) == 0xfc) {
			// passively ignore write track data
			if (dataPos_m < dataLen_m) {
				++dataPos_m;
				result = data;
			} else {
				// really should be INDEX_AM but need to differentiate
				result = GenericFloppyFormat.CRC;
			}
		} else if (dataPos_m < dataLen_m) {
			if (!dataReady) {
				result = GenericFloppyFormat.NO_DATA;
			} else {
				secBuf_m[dataPos_m++] = (byte)data;
				bufferDirty_m = true;
				if (dataPos_m < dataLen_m) {
					result = data;
				} else {
					cacheSector(-1, -1, -1); // write-thru cache
					result = GenericFloppyFormat.CRC;
				}
			}
		} else {
			cacheSector(-1, -1, -1); // write-thru cache
			result = GenericFloppyFormat.CRC;
		}
		return result;
	}

	public boolean isReady() {
		return imageFd_m != null;
	}

	public String getMediaName() {
		if (imageName_m == null) {
			return "BAD";
		}
		return imageName_m;
	}

	public boolean checkWriteProtect() {
		return writeProtect_m;
	}
	public int densityFactor() {
		return densityFactor_m;
	}
	public int mediaSize() {
		return mediaSize_m;
	}
	public long trackLen() {
		return trackLen_m;
	}
	public int hardSectored() {
		return mediaSec_m; // 0 == soft sectored; num hard sectors
	}
}
