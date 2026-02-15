// Copyright (c) 2016 Douglas Miller <durgadas311@gmail.com>
import java.util.Properties;
import java.awt.event.*;
import javax.swing.Timer;

public class CPNetDevice extends ServerDispatch implements IODevice, ActionListener {
	static final int serverId = 0;

	static final int BUFFER_OVERRUN = 512; // any value larger than buffer[]

	static final int dataPortOffset = 0;
	static final int statusPortOffset = 1;

	static final int sts_DataReady = 0x01;
	static final int sts_CmdOverrun = 0x02;
	static final int sts_RespUnderrun = 0x04;
	static final int sts_Error = 0x08;

	// includes MAGNet header...
	static final int ndosLen = NetworkServer.DAT;

	private int clientId;
	private String dir;
	private byte[] buffer = new byte[256 + ndosLen];
	private byte[] respBuf = null; // might point to 'buffer'
	private int bufIx;
	private int msgLen;
	private int respLen;
	private boolean initDev;

///
/// User (CP/M) prootocol:
///
/// Init/reset:
///     OUT ctrlPort    ; RESET device
///     IN  dataPort    ; get clientId (only valid after RESET)
///     STA myClientId  ; save for CP/Net
///     ; other initialization...
///
/// Send:
///     MVI C,dataPort
///     LXI H,msgbuf
///     MVI B,5         ; header length
///     OUTIR           ; send header
///     LDA msgbuf+4    ; msg size (-1)
///     INR A
///     MOV B,A
///     OUTIR           ; send message body
///     IN  statusPort
///     ANI 02h         ; cmd overrun bit
///     RZ              ; message accepted
///     ; error case
///
/// Receive:
///     IN statusPort
///     ANI 01h         ; data ready
///     JZ  Receive
///     MVI C,dataPort
///     LXI H,msgbuf
///     MVI B,5         ; header length
///     INIR            ; get header
///     LDA msgbuf+4    ; msg size (-1)
///     INR A
///     MOV B,A
///     INIR            ; get message body
///     IN  statusPort
///     ANI 04h         ; rcv overrun bit
///     RZ              ; message OK
///     ; error case
///
/// For reference, standard CP/Net message header is:
/// +0  format code (00 = CP/Net send, 01 = response)
/// +1  dest node ID (server or this client, depending on direction)
/// +2  src node ID (this client or server, '')
/// +3  CP/Net, MP/M, CP/M BDOS function number
/// +4  msg size - 1 (00 = 1, FF = 256)
/// +5...   message body
///

	private int base = -1;
	private String name;
	private LED led;
	javax.swing.Timer timer;

	public CPNetDevice(Properties props, LEDHandler lh, Interruptor intr) {
		super();
		int cid = 0xfe;		// OK default if we have no network connections
		int port = 0x38;	// default port outside Kaypro I/O devs
		String s;
		int no = 0;

		s = props.getProperty("cpnetdevice_port");
		if (s != null) {
			try {
				int p = Integer.decode(s);
				if (p >= 0 && p <= 255) {
					port = p;
				} else {
					System.err.format("Invalid port %02x, using default\n", p);
				}
			} catch (Exception ee) {
				System.err.format( "Invalid port number \"%s\", using default\n", s);
			}
		}
		s = props.getProperty("cpnetdevice_clientid");
		if (s != null) {
			try {
				int c = Integer.decode(s);
				if (c > 0x00 && c < 0xff) {
					cid = c;
				} else {
					System.err.format("Invalid CP/Net client ID \"%s\"\n", s);
				}
			} catch (Exception ee) {
				System.err.format("Invalid CP/Net client ID \"%s\"\n", s);
			}
		}
		System.err.format("Creating CPNetDevice device at port %02x, client ID %02x\n",
			port, cid);
		name = String.format("CP/NET-%02x", cid);
		led = lh.registerLED(getDeviceName());
		timer = new javax.swing.Timer(500, this);

		base = port;
		clientId = cid;
		bufIx = 0;
		msgLen = 0;
		respLen = 0;
		initDev = false;
		install_ServerDispatch(props, "cpnetdevice_", cid, no, 255, null);
	}

	static public boolean isConfigured(Properties props) {
		// Without this property, no CP/Net device is created.
		return (props.getProperty("cpnetdevice_clientid") != null);
	}

	public int getBaseAddress() { return base; }
	public int getNumPorts() { return 2; }

	public void reset() {
		initDev = false;
		bufIx = 0;
		msgLen = 0;
		respLen = 0;
	}

	private boolean ledState = false;

	private void startSend() {
		if (!ledState && led != null) {
			led.set(true);
			ledState = true;
		}
		timer.removeActionListener(this);
		timer.addActionListener(this);
		timer.restart();
	}

	public void actionPerformed(ActionEvent e) {
		if (e.getSource() == timer) {
			timer.removeActionListener(this);
			if (led != null) {
				led.set(false);
			}
			ledState = false;
		}
	}

	public int in(int adr) {
		int off = adr - base;
		int val = 0x00;

		if (off == statusPortOffset) {
			initDev = false;

			if (respLen > 0) {
				val |= sts_DataReady;
			}
			if (respLen < 0) {
				val |= sts_RespUnderrun;
				respLen = 0;
			}
			if (msgLen == BUFFER_OVERRUN) {
				val |= sts_CmdOverrun;
			}
			if (val != 0) {
				// any of these conditions mean the client must do work.
				return val;
			}
			// must be waiting for a response, check and see...
			// Should never get here for synchronous server handlers
			// (e.g. HostFileBdos).
			respBuf = checkRecvMsg((byte)clientId);
			// 'null' is not an error, just absence of message.
			// checkRecvMsg() must create an error packet if needed.
			if (respBuf == null) {
				return val;
			}
			// This includes any failures or errors, must be returned as CP/Net errors.
			int len = getBC(respBuf);
			respLen = len;
			val |= sts_DataReady;
			return val;
		}
		if (off == dataPortOffset) {
			if (initDev) {
				val = clientId;
				initDev = false;
				return val;
			}
			initDev = false;
			// works for 'respLen == 0' (no data) case also.
			if (bufIx < respLen) {
				val = respBuf[bufIx++] & 0xff;
				if (bufIx >= respLen) {
					// Response finished
					respBuf = null; // in case allocated
					respLen = 0;
					bufIx = 0;
				}
			} else {
				// error condition: reading bytes that don't exist.
				respLen = -1;
				bufIx = 0;
			}
			return val;
		}
		System.err.format("Invalid port address %02x\n", adr);
		return val;
	}

	public void out(int adr, int val) {
		int off = adr - base;

		initDev = false;
		if (off == statusPortOffset) {
			// reset / resync. other functions needed?
			initDev = true;	// send clientId on next input data port.
			bufIx = 0;
			msgLen = 0;
			respLen = 0;
			return;
		}
		if (off == dataPortOffset) {
			if (respLen > 0) {
				// response was not yet consumed.
				// force error?
				msgLen = BUFFER_OVERRUN;
				return;
			}
			if (bufIx < ndosLen) {
				buffer[bufIx++] = (byte)val;

				if (bufIx >= ndosLen) {
					msgLen = ndosLen + (buffer[NetworkServer.SIZ] & 0xff) + 1;
					// Header received, ready for message
				}
				return;
			}
			if (bufIx < msgLen) {
				buffer[bufIx++] = (byte)val;
				if (bufIx >= msgLen) {
					// we have something to do...
					startSend();
					respBuf = sendMsg(buffer, msgLen);
					// 'null' indicates async, not error...
					// sendMsg must have prepared error packet.
					bufIx = 0;
					msgLen = 0;
					if (respBuf == null) {
						// indicates async message sent - wait for recv later.
						return;
					}
					respLen = ndosLen + (respBuf[NetworkServer.SIZ] & 0xff) + 1;
					// have response...
				}
				// don't do anything, just wait for receiver to execute command?
				return;
			}
			// too many bytes - probably some lost/aborted message.
			// need to remember error condition for later.
			System.err.format("Command buffer overrun %d/%d\n", bufIx, msgLen);
			msgLen = BUFFER_OVERRUN;
			return;
		}
		System.err.format("Invalid port address %02x\n", adr);
	}

	public String getDeviceName() { return name; }

	public String dumpDebug() {
		String str = String.format("Status: %s%s%s [%d]\n",
			msgLen == BUFFER_OVERRUN ? "CmdOverrun " : "-",
			respLen < 0 ? "RespUnderrun " : "-",
			respLen > 0 ? "DataReady " : "-",
			bufIx);
		str += String.format("%02x %04x %04x %04x\n",
			getCode(buffer), getBC(buffer), getDE(buffer), getHL(buffer));
		str += String.format("%02x %02x %02x %02x %02x : %02x %02x\n",
			buffer[0] & 0xff, buffer[1] & 0xff, buffer[2] & 0xff,
			buffer[3] & 0xff, buffer[4] & 0xff,
			buffer[5] & 0xff, buffer[6] & 0xff);
		return str;
	}
}
