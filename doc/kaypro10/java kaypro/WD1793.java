// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.util.Arrays;
import java.util.HashMap;

public class WD1793 implements ClockListener {
	static final int WD1793_NumPorts_c = 4;
	static final int StatusPort_Offset_c = 0;
	static final int CommandPort_Offset_c = 0;
	static final int TrackPort_Offset_c = 1;
	static final int SectorPort_Offset_c = 2;
	static final int DataPort_Offset_c = 3;

	protected boolean intrqRaised_m;
	protected boolean drqRaised_m;

	protected int trackReg_m;
	protected int sectorReg_m;
	protected int dataReg_m;
	protected int cmdReg_m;
	protected int statusReg_m;

	protected boolean dataReady_m;
	protected boolean headLoaded_m;
	private int sectorLength_m;
	private boolean lastIndexStatus_m;
	protected int indexCount_m;
	private boolean stepUpdate_m;
	private long stepSettle_m;
	private int missCount_m;
	private boolean isTypeI;

	/// type I parameters.
	private int seekSpeed_m;
	private boolean verifyTrack_m;

	/// type II parameters.
	private boolean multiple_m;
	private boolean delay_m;
	private int side_m;
	private boolean sideCmp_m;
	private boolean deleteDAM_m;
	private int[] addr_m = new int[6];

	static final int dir_out = -1;
	static final int dir_in = 1;

	private enum Command {
		restoreCmd,
		seekCmd,
		stepCmd,/// shared with step in/step out.
		readSectorCmd,
		writeSectorCmd,
		readAddressCmd,
		readTrackCmd,
		writeTrackCmd,
		forceInterruptCmd,
		// pseudo-commands/states used internally
		stepDoneCmd,
		noneCmd
	};
	private Command curCommand_m;

	private int stepDirection_m;

	private long curPos_m;

	private int sectorPos_m;

	///
	/// Commands sent to CommandPort_c
	///
	static final int cmd_Mask_c = 0xf0;
	static final int cmd_Restore_c = 0x00;	// 0000 hVrr
	static final int cmd_SeekTrack_c = 0x10;	// 0001 hVrr
	static final int cmd_StepRepeat_c = 0x20;	// 001T hVrr
	static final int cmd_StepIn_c = 0x40;	// 010T hVrr
	static final int cmd_StepOut_c = 0x60;	// 011T hVrr
	static final int cmd_ReadSector_c = 0x80;	// 100m SEC0
	static final int cmd_WriteSector_c = 0xa0;	// 101m SECa
	static final int cmd_ReadAddress_c = 0xc0;	// 1100 0E00
	static final int cmd_ReadTrack_c = 0xe0;	// 1110 0E00
	static final int cmd_WriteTrack_c = 0xf0;	// 1111 0E00
	static final int cmd_ForceInterrupt_c = 0xd0;	// 1101 IIII (I3/I2/I1/I0)

	///
	/// rr - Stepping Motor Rate
	/// ===============================================
	/// 00 -  6 mSec (5.25")
	/// 01 - 12 mSec (5.25")
	/// 10 - 20 mSec (5.25")
	/// 11 - 30 mSec (5.25")
	///
	static final int maxStepSpeeds_c = 4;
	static final int cmdop_StepMask_c = 0x03;	// 0000 0011

	///
	/// V - Track Verify
	/// ===============================================
	/// 0 - No verify
	/// 1 - Verify on destination
	///
	static final int cmdop_VerifyTrack_c = 0x04;

	///
	/// h - Head Load Flag
	/// ===============================================
	/// 0 - Load Head at the beginning
	/// 1 - Unload head at the beginning
	///
	static final int cmdop_HeadLoad_c = 0x08;

	///
	/// T - Track Update Flag
	/// ===============================================
	/// 0 - No update
	/// 1 - Update Track Register
	///
	static final int cmdop_TrackUpdate_c = 0x10;

	///
	/// m - Multiple Record Flag
	/// ===============================================
	/// 0 - Single Record
	/// 1 - Multiple records
	///
	static final int cmdop_MultipleRecord_c = 0x10;

	///
	/// a - Data Address Mark
	/// ===============================================
	/// 0 - FB(DAM)
	/// 1 - F8(delete DAM)
	///
	static final int cmdop_DataAddressMark_c = 0x01;

	///
	/// U - Side Compare
	/// ===============================================
	/// 0 - Do Not Compare Side
	/// 1 - Compare Side
	///
	static final int cmdop_CompareSide_c = 0x02;

	///
	/// E - 15 mSec Delay
	/// ===============================================
	/// 0 - No delay
	/// 1 - 15 mSec Delay
	///
	static final int cmdop_Delay_15ms_c = 0x04;

	///
	/// L - Sector Length Flag
	/// ---------------------------------------
	///     | LSB in Sector Length in ID field
	///     |   00   |   01   |   10  |   11
	/// ----+--------+--------+-------+-------
	/// L=0 |  256   |  512   |  1024 |  128
	/// L=1 |  128   |  256   |   512 | 1024
	///
	static final int cmdop_SideSelect_c = 0x08;
	static final int cmdop_SideSelect_Shift_c = 3;

	///
	/// Options to the Force Interrupt command
	///
	///  Ix
	/// ===============================================
	///  I3 - Immediate Interrupt, Requires a Reset
	///  I2 - Index Pulse
	///  I1 - Ready to Not Ready Transition
	///  I0 - Not Ready to Ready Transition
	///
	static final int cmdop_ImmediateInterrupt_c = 0x08;
	static final int cmdop_IndexPulse_c = 0x04;
	static final int cmdop_ReadyToNotReady_c = 0x02;
	static final int cmdop_NotReadyToReady_c = 0x01;

	///
	/// Status bit definitions
	///

	/// Type I commands Status
	/// ===============================================
	static final int stat_NotReady_c = 0x80;
	static final int stat_WriteProtect_c = 0x40;
	static final int stat_HeadLoaded_c = 0x20;
	static final int stat_SeekError_c = 0x10;
	static final int stat_CRCError_c = 0x08;
	static final int stat_TrackZero_c = 0x04;
	static final int stat_IndexPulse_c = 0x02;
	static final int stat_Busy_c = 0x01;

	/// Read Address Status
	/// ===============================================
	/// stat_NotReady_c       - 0x80;
	/// 0                     - 0x40;
	/// 0                     - 0x20;
	static final int stat_RecordNotFound_c = 0x10;
	/// stat_CRCError_c       - 0x08;
	static final int stat_LostData_c = 0x04;
	static final int stat_DataRequest_c = 0x02;
	/// stat_Busy_c           - 0x01;

	/// Read Sector Status
	/// ===============================================
	/// stat_NotReady_c       - 0x80;
	/// 0                     - 0x40;
	static final int stat_RecordType_c = 0x20;
	/// stat_RecordNotFound_c - 0x10;
	/// stat_CRCError_c       - 0x08;
	/// stat_LostData_c       - 0x04;
	/// stat_DataRequest_c    - 0x02;
	/// stat_Busy_c           - 0x01;

	/// Read Track Status
	/// ===============================================
	/// stat_NotReady_c       - 0x80;
	/// 0                     - 0x40;
	/// 0                     - 0x20;
	/// 0                     - 0x10;
	/// 0                     - 0x08;
	/// stat_LostData_c       - 0x04;
	/// stat_DataRequest_c    - 0x02;
	/// stat_Busy_c           - 0x01;

	/// Write Sector Status
	/// ===============================================
	/// stat_NotReady_c       - 0x80;
	/// stat_WriteProtect_c   - 0x40;
	static final int stat_WriteFault_c = 0x20;
	/// stat_RecordNotFound_c - 0x10;
	/// stat_CRCError_c       - 0x08;
	/// stat_LostData_c       - 0x04;
	/// stat_DataRequest_c    - 0x02;
	/// stat_Busy_c           - 0x01;

	/// Write Track Status
	/// ===============================================
	/// stat_NotReady_c       - 0x80;
	/// stat_WriteProtect_c   - 0x40;
	/// stat_WriteFault_c     - 0x20;
	/// 0                     - 0x10;
	/// 0                     - 0x08;
	/// stat_LostData_c       - 0x04;
	/// stat_DataRequest_c    - 0x02;
	/// stat_Busy_c           - 0x01;

	private WD1793Controller ctrl;

	protected int basePort_m;

	// 8" (2MHz) step rates, 5.25" (1MHz) are 2x.
	static final int[] speeds = new int[] { 3, 6, 10, 15 };

	static final int[][] sectorLengths = new int[][] {
		{ 256, 512, 1024, 128 },// [0]
		{ 128, 256, 512,  1024}	// [1]
	};

	public WD1793(int base, Interruptor intr) {
		intr.addClockListener(this);
		basePort_m = base;
		curPos_m = 0;
	}

	public void setController(WD1793Controller ctrl) {
		this.ctrl = ctrl;
	}

	private void updateReady(GenericFloppyDrive drive) {
		if (drive.isWriteProtect()) {
			statusReg_m |= stat_WriteProtect_c;
		} else {
			statusReg_m &= ~stat_WriteProtect_c;
		}
		if (drive.isReady()) {
			statusReg_m &= ~stat_NotReady_c;
		} else {
			statusReg_m |= stat_NotReady_c;
		}
	}

	public void reset() {
		trackReg_m = 0;
		sectorReg_m = 0;
		dataReg_m = 0;
		cmdReg_m = 0;
		statusReg_m = 0;
		dataReady_m = false;
		intrqRaised_m = false;
		drqRaised_m = false;
		headLoaded_m = false;
		sectorLength_m = 0;
		lastIndexStatus_m = false;
		indexCount_m = 0;
		stepUpdate_m = false;
		stepSettle_m = 0;
		missCount_m = 0;
		seekSpeed_m = 0;
		verifyTrack_m = false;
		multiple_m = false;
		delay_m = false;
		sideCmp_m = false;
		side_m = 0;
		deleteDAM_m = false;
		curCommand_m = Command.noneCmd;
		stepDirection_m = dir_out;
		// leave curPos_m alone, diskette is still spinning...
		sectorPos_m = -11;
	}

	protected int in(int addr) {
		int offset = addr - basePort_m;
		int val = 0;
		switch (offset) {
		case StatusPort_Offset_c:
			val = statusReg_m;
			lowerIntrq();
			break;

		case TrackPort_Offset_c:
			val = trackReg_m;
			break;

		case SectorPort_Offset_c:
			val = sectorReg_m;
			break;

		case DataPort_Offset_c:
			val = dataReg_m;
			dataReady_m = false;
			statusReg_m &= ~stat_DataRequest_c;
			lowerDrq();
			break;

		default:
			System.err.format("in(Unknown - 0x%02x)\n", addr);
			break;

		}
		return val;
	}

	public void out(int addr, int val) {
		int offset = addr - basePort_m;
		switch (offset) {
		case CommandPort_Offset_c:
			// MUST tolerate changing of selected disk *after* setting command...
			// timing is tricky...
			indexCount_m = 0;
			cmdReg_m = val;
			processCmd(val);
			break;

		case TrackPort_Offset_c:
			trackReg_m = val;
			break;

		case SectorPort_Offset_c:
			sectorReg_m = val;
			break;

		case DataPort_Offset_c:
			// unpredictable results if !dataReady_m... (data changed while being written).
			// other mechanisms detect lostData, which is different.
			dataReg_m = val;
			dataReady_m = true;
			lowerDrq();
			break;

		default:
			System.err.format("out(Unknown - 0x%02x): %d\n", addr, val);
			break;
		}
	}

	private void processCmd(int cmd) {
		// First check for the Force Interrupt command
		if ((cmd & cmd_Mask_c) == cmd_ForceInterrupt_c) {
			isTypeI = true;
			processCmdTypeIV(cmd);
			return;
		}

		// Make sure controller is not already busy. Documentation
		// did not specify what would happen.
		if ((statusReg_m & stat_Busy_c) == stat_Busy_c) {
			System.err.format("New command while still busy: %02x\n", cmd);
			// return;
		}
		// Can't reference drive here... it could be about to change.
		// GenericFloppyDrive drive = ctrl.getCurDrive();


		// set Busy flag
		statusReg_m |= stat_Busy_c;
		statusReg_m &= ~stat_LostData_c;// right?

		isTypeI = ((cmd & 0x80) == 0x00);
		if (isTypeI) {
			// Type I commands
			processCmdTypeI(cmd);
		} else if ((cmd & 0x40) == 0x00) {
			// Type II commands
			processCmdTypeII(cmd);
		} else {
			// must be Type III command
			processCmdTypeIII(cmd);
		}
	}

	private void processCmdTypeI(int cmd) {
		verifyTrack_m = ((cmd & cmdop_VerifyTrack_c) != 0);
		seekSpeed_m = speeds[cmd & cmdop_StepMask_c];
		lowerDrq();
		dataReady_m = false;

		if (ctrl.getClockPeriod() > 500) {
			seekSpeed_m *= 2;
		}
		loadHead((cmd & cmdop_HeadLoad_c) != 0);
		stepUpdate_m = false;

		// reset CRC Error, Seek Error, DRQ, INTRQ
		statusReg_m &= ~(stat_CRCError_c | stat_SeekError_c);
		lowerDrq();
		lowerIntrq();

		if ((cmd & 0xf0) == 0x00) {
			curCommand_m = Command.restoreCmd;
		} else if ((cmd & 0xf0) == 0x10) {
			curCommand_m = dataReg_m == 0 ? Command.restoreCmd : Command.seekCmd;
		} else {
			// One of the Step commands
			curCommand_m = Command.stepCmd;

			stepUpdate_m = (cmd & cmdop_TrackUpdate_c) != 0;

			// check for step in or step out.
			if ((cmd & 0x40) == 0x40) {
				if ((cmd & 0x20) == 0x20) {
					// Step Out
					stepDirection_m = dir_out;
				} else {
					// Step In
					stepDirection_m = dir_in;
				}
			}
		}

		// Drive selection might change, need to delay start of command...
		stepSettle_m = 50;

		// TODO: detect track0
		// statusReg_m |= stat_TrackZero_c;

	}

	private void processCmdTypeII(int cmd) {
		multiple_m = ((cmd & cmdop_MultipleRecord_c) != 0);
		delay_m = ((cmd & cmdop_Delay_15ms_c) != 0);
		sectorLength_m = 1;
		sideCmp_m = ((cmd & cmdop_CompareSide_c) != 0);
		if (sideCmp_m) {
			side_m = ((cmd & cmdop_SideSelect_c) >> cmdop_SideSelect_Shift_c);
		} else {
			side_m = -1;
		}
		loadHead(true);

		lowerDrq();
		dataReady_m = false;
		sectorPos_m = -11;
		statusReg_m &= ~(stat_DataRequest_c | stat_LostData_c |
			stat_CRCError_c | stat_RecordNotFound_c |
			stat_WriteFault_c | stat_WriteProtect_c);

		if ((cmd & 0x20) == 0x20) {
			// Write Sector
			deleteDAM_m = ((cmd & cmdop_DataAddressMark_c) != 0);

			if (deleteDAM_m) {
				System.err.format("Deleted Data Addr Mark not supported - ignored\n");
			}
			curCommand_m = Command.writeSectorCmd;
		} else {
			// Read Sector
			curCommand_m = Command.readSectorCmd;
		}
		stepSettle_m = 100;	// give host time to get ready...
	}

	private void processCmdTypeIII(int cmd) {
		delay_m = ((cmd & cmdop_Delay_15ms_c) != 0);
		sideCmp_m = ((cmd & cmdop_CompareSide_c) != 0);
		if (sideCmp_m) {
			side_m = ((cmd & cmdop_SideSelect_c) >> cmdop_SideSelect_Shift_c);
		} else {
			side_m = -1;
		}
		loadHead(true);
		lowerDrq();
		dataReady_m = false;
		sectorPos_m = -11;

		statusReg_m &= ~(stat_DataRequest_c | stat_LostData_c |
			stat_CRCError_c | stat_RecordNotFound_c |
			stat_WriteFault_c | stat_WriteProtect_c);

		if ((cmd & 0xf0) == 0xc0) {
			// Read Address
			curCommand_m = Command.readAddressCmd;

		} else if ((cmd & 0xf0) == 0xf0) {
			// write Track
			curCommand_m = Command.writeTrackCmd;
			raiseDrq();

		} else if ((cmd & 0xf0) == 0xe0) {
			// read Track
			curCommand_m = Command.readTrackCmd;

		} else {
			System.err.format("Invalid type-III cmd: %02x\n", cmd);
			statusReg_m &= ~stat_Busy_c;
			return;
		}

		stepSettle_m = 100;	// give host time to get ready...
	}

	// 'drive' might be NULL.
	private void processCmdTypeIV(int cmd) {
		loadHead(false);
		// we assume drive won't change for Type IV commands...
		GenericFloppyDrive drive = ctrl.getCurDrive();

		curCommand_m = Command.forceInterruptCmd;

		// check to see if previous command is still running
		if ((statusReg_m & stat_Busy_c) != 0) {
			// still running, abort command and reset busy
			abortCmd();
			statusReg_m &= ~stat_Busy_c;
		} else if (drive != null) {
			// no Command running, update status.
			statusReg_m = stat_Busy_c;
			updateReady(drive);
			if (drive.getTrackZero()) {
				statusReg_m |= stat_TrackZero_c;
			}
			if (drive.getIndexPulse()) {
				statusReg_m |= stat_IndexPulse_c;
			}
		} else {
			//System.err.format("Type IV with no drive\n");
			statusReg_m &= ~stat_SeekError_c;
			statusReg_m &= ~stat_CRCError_c;
		}

		if ((cmd & 0x0f) != 0x00) {
			// TODO: Fix this, must setup background "task" waiting for event...

			// at least one bit is set.
			if ((cmd & cmdop_NotReadyToReady_c) == cmdop_NotReadyToReady_c) {
			}
			if ((cmd & cmdop_ReadyToNotReady_c) == cmdop_ReadyToNotReady_c) {
			}
			if ((cmd & cmdop_IndexPulse_c) == cmdop_IndexPulse_c) {
			}
			if ((cmd & cmdop_ImmediateInterrupt_c) == cmdop_ImmediateInterrupt_c) {
				statusReg_m &= ~stat_Busy_c;
				raiseIntrq();
			}
		} else {
			statusReg_m &= ~stat_Busy_c;
			curCommand_m = Command.noneCmd;
		}
	}

	private void abortCmd() {
		//System.err.format("Done %02x/%02x %02x %02x %02x\n",
		//	statusReg_m, cmdReg_m, trackReg_m, sectorReg_m, dataReg_m);
		curCommand_m = Command.noneCmd;
	}

	protected void raiseIntrq() {
		intrqRaised_m = true;
		ctrl.raisedIntrq();
	}

	protected void raiseDrq() {
		drqRaised_m = true;
		ctrl.raisedDrq();
	}

	protected void lowerIntrq() {
		intrqRaised_m = false;
		ctrl.loweredIntrq();
	}

	protected void lowerDrq() {
		drqRaised_m = false;
		ctrl.loweredDrq();
	}

	protected void loadHead(boolean load) {
		headLoaded_m = load;
		ctrl.loadedHead(load);
	}

	private void transferData(int data) {
		if (dataReady_m) {
			statusReg_m |= stat_LostData_c;
		}
		dataReady_m = true;
		dataReg_m = data;
		statusReg_m |= stat_DataRequest_c;
		raiseDrq();
	}

	private boolean checkAddr(int[] addr) {
		return addr[0] == trackReg_m &&
			(!sideCmp_m || addr[1] == side_m) &&
			addr[2] == sectorReg_m;
	}

	private int sectorLen(int[] addr) {
		int x = addr[3] & 0x03;
		return sectorLengths[sectorLength_m][x];
	}

	private void commandCompleted() {
		statusReg_m &= ~stat_Busy_c;
		//System.err.format("Done %02x/%02x %02x %02x %02x\n",
		//	statusReg_m, cmdReg_m, trackReg_m, sectorReg_m, dataReg_m);
		raiseIntrq();
		curCommand_m = Command.noneCmd;
	}

	public void addTicks(int cycleCount, long clk) {
		long charPos = 0;
		boolean indexEdge = false;
		if (stepSettle_m > 0) {
			if (stepSettle_m > cycleCount) {
				stepSettle_m -= cycleCount;
				return;
			} else {
				stepSettle_m = 0;
				missCount_m = 0;
			}
		}
		GenericFloppyDrive drive = ctrl.getCurDrive();
		if (drive == null) {
			// TODO: set some status indicator.
			statusReg_m |= stat_NotReady_c;
			if (curCommand_m != Command.noneCmd) {
				abortCmd();
				// TODO: shouldn't abortCmd() do all this?
				raiseIntrq();
				statusReg_m &= ~stat_Busy_c;
			}
			return;
		}
		drive.addTicks(cycleCount, clk);
		statusReg_m &= ~stat_NotReady_c;
		// GenericFloppyDrive does not return INDEX if motor off.
		if (drive.getIndexPulse()) {
			if (!lastIndexStatus_m) {
				indexEdge = true;
				++indexCount_m;
				if (isTypeI) {
					statusReg_m |= stat_IndexPulse_c;
				}
			}
			lastIndexStatus_m = true;
		} else {
			if (lastIndexStatus_m) {
				if (isTypeI) {
					statusReg_m &= ~stat_IndexPulse_c;
				}
			}
			lastIndexStatus_m = false;
		}
		if (indexEdge) {
			// Someday we may need this event, again. e.g. to timeout commands.
			// 'indexEdge' is true once and only once per revolution.
			// 'getIndexPulse()' may be true for many (e.g. 100) clock cycles.
		}
		updateReady(drive);
		if (curCommand_m == Command.noneCmd) {
			return;
		}
		charPos = drive.getCharPos(ctrl.densityFactor());
		if (drive.isReady() && charPos == curPos_m) {
			// Position hasn't changed just return
			return;
		}
		curPos_m = charPos;

		// Process any active Type I command...
		switch (curCommand_m) {
		case restoreCmd:
		{
			if (!drive.getTrackZero()) {
				drive.step(false);
				stepSettle_m = 100;	// millisecToTicks(seekSpeed_m);
			} else {
				trackReg_m = 0;
				statusReg_m |= stat_TrackZero_c;
				commandCompleted();
			}

			break;
		}

		case seekCmd:
			if (dataReg_m != trackReg_m) {
				// NOTE: "hub" (in) is track <MAX-1>,
				// rim (out) is track 00.
				boolean dirIn = (dataReg_m > trackReg_m);
				drive.step(dirIn);
				trackReg_m += (dirIn ? 1 : -1);
				stepSettle_m = 100;	// millisecToTicks(seekSpeed_m);
			} else {
				if (verifyTrack_m) {
					// TODO: confirm this works...
					int[] adr = drive.readAddress();
					if (adr == null) {
						statusReg_m |= stat_CRCError_c;
					} else if (adr[0] != trackReg_m) {
						statusReg_m |= stat_SeekError_c;
					}
				}

				if (drive.getTrackZero()) {
					statusReg_m |= stat_TrackZero_c;
				} else {
					statusReg_m &= ~stat_TrackZero_c;
				}
				commandCompleted();
			}

			break;

		case stepCmd:
			if (stepDirection_m == dir_out) {
				// NOTE: "hub" (in) is track <MAX-1>,
				// rim (out) is track 00.
				if (!drive.getTrackZero()) {
					drive.step(false);

					if (drive.getTrackZero()) {
						statusReg_m |= stat_TrackZero_c;
					} else {
						statusReg_m &= ~stat_TrackZero_c;
					}
					stepSettle_m = 100;	// millisecToTicks(seekSpeed_m);

					if (stepUpdate_m)
						trackReg_m--;
				} else {
					statusReg_m |= stat_TrackZero_c;
				}
			} else if (stepDirection_m == dir_in) {

				drive.step(true);
				statusReg_m &= ~stat_TrackZero_c;
				stepSettle_m = 100;	// millisecToTicks(seekSpeed_m);

				if (stepUpdate_m) {
					trackReg_m++;
				}
			}

			curCommand_m = Command.stepDoneCmd;
			break;

		case stepDoneCmd:
			if (drive.getTrackZero()) {
				statusReg_m |= stat_TrackZero_c;
			} else {
				statusReg_m &= ~stat_TrackZero_c;
			}
			commandCompleted();
			break;

		default:
			break;
		}

		int data;
		int result;

		switch (curCommand_m) {
		case restoreCmd:
		case seekCmd:
		case stepCmd:
		case stepDoneCmd:
		case noneCmd:
			updateReady(drive);

			if (lastIndexStatus_m) {
				statusReg_m |= stat_IndexPulse_c;
			} else {
				statusReg_m &= ~stat_IndexPulse_c;
			}
			break;

		case readSectorCmd:

			// user may choose to ignore data... must not hang here!
			if (dataReady_m) {
				if ((statusReg_m & stat_LostData_c) == 0 && ++missCount_m < 4) {
					// wait a little for host to catch up
					break;
				}
				statusReg_m |= stat_LostData_c;
			}

			missCount_m = 0;
			data = drive.readData(ctrl.densityFactor(), trackReg_m, side_m, sectorReg_m,
									sectorPos_m);

			if (data == GenericFloppyFormat.NO_DATA) {
				// just wait for sector to come around..
			} else if (data == GenericFloppyFormat.INDEX_AM) {
				sectorPos_m = -1;
				statusReg_m |= stat_RecordNotFound_c;
				commandCompleted();
			} else if (data == GenericFloppyFormat.DATA_AM) {
				sectorPos_m = 0;
			} else if (data == GenericFloppyFormat.CRC) {
				sectorPos_m = -1;
				if (multiple_m) {
					++sectorReg_m;
					break; // keep going until INDEX_AM
				}
				commandCompleted();
			} else if (data < 0) {
				// probably ERROR
				sectorPos_m = -1;
				statusReg_m |= stat_CRCError_c;
				commandCompleted();
			} else {
				transferData(data);
				++sectorPos_m;
			}

			break;

		case readAddressCmd:

			// user may choose to ignore data... must not hang here!
			if (dataReady_m) {
				if ((statusReg_m & stat_LostData_c) == 0 && ++missCount_m < 4) {
					// wait a little for host to catch up
					break;
				}
				statusReg_m |= stat_LostData_c;
			}

			missCount_m = 0;
			// sector '0xfd' indicates a read address
			data = drive.readData(ctrl.densityFactor(), trackReg_m, side_m, 0xfd,
									sectorPos_m);

			if (data == GenericFloppyFormat.NO_DATA) {
				// just wait for sector to come around..
				// should never happen, as long as track was formatted.
			} else if (data == GenericFloppyFormat.ID_AM) {
				sectorPos_m = 0;
			} else if (data == GenericFloppyFormat.CRC) {
				sectorPos_m = -1;
				commandCompleted();
			} else if (data < 0) {
				// probably ERROR
				sectorPos_m = -1;
				statusReg_m |= stat_CRCError_c;
				commandCompleted();
			} else {
				if (sectorPos_m == 0) {
					sectorReg_m = data;
				}

				transferData(data);
				++sectorPos_m;
			}

			break;

		case writeSectorCmd:
			result = drive.writeData(ctrl.densityFactor(), trackReg_m, side_m, sectorReg_m,
						  sectorPos_m, dataReg_m, dataReady_m);

			if (result == GenericFloppyFormat.NO_DATA) {
				// out of paranoia, but must be careful
				if (sectorPos_m >= 0 && !drqRaised_m) {
					raiseDrq();
				}
				// just wait for sector to come around..
			} else if (result == GenericFloppyFormat.INDEX_AM) {
				sectorPos_m = -1;
				if (multiple_m) {
					statusReg_m |= stat_RecordNotFound_c;
					commandCompleted();
				} else {
					// Error?
				}
			} else if (result == GenericFloppyFormat.DATA_AM) {
				sectorPos_m = 0;
			} else if (result == GenericFloppyFormat.CRC) {
				sectorPos_m = -1;
				if (multiple_m) {
					++sectorReg_m;
					break; // keep going until INDEX_AM
				}
				commandCompleted();
			} else if (result < 0) {
				// other errors
				sectorPos_m = -1;
				statusReg_m |= stat_WriteFault_c;
				commandCompleted();
			} else {
				dataReady_m = false;
				++sectorPos_m;
				raiseDrq();
			}

			break;

		case readTrackCmd:

			// user may choose to ignore data... must not hang here!
			if (dataReady_m) {
				if ((statusReg_m & stat_LostData_c) == 0 && ++missCount_m < 4) {
					// wait a little for host to catch up
					break;
				}
				statusReg_m |= stat_LostData_c;
			}

			missCount_m = 0;
			data = drive.readData(ctrl.densityFactor(), trackReg_m, side_m, 0xff, sectorPos_m);

			if (data == GenericFloppyFormat.NO_DATA) {
				// just wait for index to come around..
			} else if (data == GenericFloppyFormat.INDEX_AM) {
				sectorPos_m = 0;
			} else if (data == GenericFloppyFormat.CRC) {
				sectorPos_m = -1;
				commandCompleted();
			} else if (data < 0) {
				// probably ERROR
				sectorPos_m = -1;
				statusReg_m |= stat_CRCError_c;
				commandCompleted();
			} else {
				transferData(data);
				++sectorPos_m;
			}

			break;

		case writeTrackCmd:
			result = drive.writeData(ctrl.densityFactor(), trackReg_m, side_m, 0xff,
						  sectorPos_m, dataReg_m, dataReady_m);

			if (result == GenericFloppyFormat.NO_DATA) {
				// out of paranoia, but must be careful
				if (sectorPos_m >= 0 && !drqRaised_m) {
					raiseDrq();
				}
				// just wait for sector to come around..
			} else if (result == GenericFloppyFormat.INDEX_AM) {
				sectorPos_m = 0;
			} else if (result == GenericFloppyFormat.CRC) {
				sectorPos_m = -1;
				commandCompleted();
			} else if (result < 0) {
				// other errors
				System.err.format("Error in write track\n");
				sectorPos_m = -1;
				statusReg_m |= stat_WriteFault_c;
				commandCompleted();
			} else {
				dataReady_m = false;
				++sectorPos_m;
				raiseDrq();
			}

			break;

		case forceInterruptCmd:
			// TODO: watch for event(s) and raise interrupt when seen...
			break;
		}

	}

	protected long millisecToTicks(long ms) {
		long tps = 500; // TODO: abstract this
		long ticks = (tps * ms) / 1000;

		return ticks;
	}
}
