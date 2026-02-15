// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

public class GenericFloppyFormat {
	// These are the bytes used by the WD1797 for disk marks.
	static final int INDEX_AM_BYTE = 0xfc;
	static final int ID_AM_BYTE = 0xfe;
	static final int DATA_AM_BYTE = 0xfb;
	static final int CRC_BYTE = 0xf7;

	// These are the bytes used internally to identify disk marks.
	static final int INDEX_AM = -INDEX_AM_BYTE;
	static final int ID_AM = -ID_AM_BYTE;
	static final int DATA_AM = -DATA_AM_BYTE;
	static final int CRC = -CRC_BYTE;

	static final int ERROR = -1;
	static final int NO_DATA = -2;
}
