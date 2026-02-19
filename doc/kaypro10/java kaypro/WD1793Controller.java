// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

// The interface used by a WD1793 to assert signals to the disk controller board.
public interface WD1793Controller {
	void raisedIntrq();
	void raisedDrq();
	void loweredIntrq();
	void loweredDrq();
	void loadedHead(boolean load);
	int densityFactor();
	int getClockPeriod();
	GenericFloppyDrive getCurDrive();
}
