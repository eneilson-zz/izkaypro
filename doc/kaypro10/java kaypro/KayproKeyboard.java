// Copyright (c) 2017 Douglas Miller
import java.awt.event.*;
import java.io.*;
import java.util.Map;
import java.util.HashMap;
import java.util.Vector;
import java.util.Properties;
import javax.sound.sampled.*;

public class KayproKeyboard implements PasteListener, KeyListener, Runnable {
	VirtualUART _uart;
	java.util.concurrent.LinkedBlockingDeque<String> fifo;
	int paste_delay = 0;	// mS, 1000/cps
	int cr_delay = 0;	// mS
	KeyboardBeep _kbd;
	static final Map<Integer, Integer> altKeys;
	static {
		altKeys = new HashMap<Integer, Integer>();
		altKeys.put((int)'0', 0xb1);
		altKeys.put((int)'1', 0xc0);
		altKeys.put((int)'2', 0xc1);
		altKeys.put((int)'3', 0xc2);
		altKeys.put((int)'4', 0xd0);
		altKeys.put((int)'5', 0xd1);
		altKeys.put((int)'6', 0xd2);
		altKeys.put((int)'7', 0xe1);
		altKeys.put((int)'8', 0xe2);
		altKeys.put((int)'9', 0xe3);
		altKeys.put((int)'-', 0xe4);
		altKeys.put((int)',', 0xd3); // TODO: not on modern keyboard!
		altKeys.put((int)'+', 0xd3); // same relative position as comma...
		altKeys.put((int)'\n', 0xc3);
		altKeys.put((int)'.', 0xb2);
	}

	class KeyboardBeep implements SerialDevice {
		Clip beep;

		public KeyboardBeep(Properties props, VirtualUART uart) {
			setupBeep(props);
			uart.attachDevice(this);
		}

		private void setupBeep(Properties props) {
			beep = null;
			String s = props.getProperty("kaypro_beep");
			if (s == null) {
				s = "kpbeep.wav";
			} else if (s.length() == 0) {
				return;
			}
			String beep_wav = s;
			try {
				AudioInputStream wav =
					AudioSystem.getAudioInputStream(
						new BufferedInputStream(
							this.getClass().getResourceAsStream(beep_wav)));
				AudioFormat format = wav.getFormat();
				DataLine.Info info = new DataLine.Info(Clip.class, format);
				beep = (Clip)AudioSystem.getLine(info);
				beep.open(wav);
				//beep.setLoopPoints(0, loop);
			} catch (Exception e) {
				//e.printStackTrace();
				beep = null;
				return;
			}
			int volume = 50;
			s = props.getProperty("kaypro_beep_volume");
			if (s != null) {
				volume = Integer.valueOf(s);
				if (volume < 0) volume = 0;
				if (volume > 100) volume = 100;
			}
			FloatControl vol = null;
			if (beep.isControlSupported(FloatControl.Type.MASTER_GAIN)) {
				vol = (FloatControl)beep.getControl(FloatControl.Type.MASTER_GAIN);
			} else if (beep.isControlSupported(FloatControl.Type.VOLUME)) {
				vol = (FloatControl)beep.getControl(FloatControl.Type.VOLUME);
			}
			if (vol != null) {
				float min = vol.getMinimum();
				float max = vol.getMaximum();
				float gain = (float)(min + ((max - min) * (volume / 100.0)));
				vol.setValue(gain);
			}
		}

		private void beep() {
			if (beep != null) {
				beep.setFramePosition(0);
				beep.loop(0);
			}
		}

		// SerialDevice interface
		public void write(int b) {	// CPU is writing the serial data port
			if (b == 0x04) {
				beep();
			}
		}
		public int read() { return 0; }
		public int available() { return 0; }
		public void rewind() {}
		public void modemChange(VirtualUART me, int mdm) {}
		public int dir() { return SerialDevice.DIR_OUT; }
		public String dumpDebug() {
			return "";
		}
	}

	public KayproKeyboard(Properties props, Vector<String> argv,
			VirtualUART uart) {
		_uart = uart;
		_kbd = new KeyboardBeep(props, uart);
		fifo = new java.util.concurrent.LinkedBlockingDeque<String>();
		String s = props.getProperty("kaypro_paste_rate");
		int cps = 0;
		int crd = 0;
		if (s != null) {
			try {
				cps = Integer.valueOf(s);
			} catch (Exception ee) {
				cps = 33;
				System.err.format("Bad paste rate \"%s\", using %d\n",
									s, cps);
			}
		}
		s = props.getProperty("kaypro_paste_cr_delay");
		if (s != null) {
			try {
				crd = Integer.valueOf(s);
			} catch (Exception ee) {
				crd = cps > 0 ? 3 * (1000 / cps) : 100;
				System.err.format("Bad paste CR delay \"%s\", using %d\n",
									s, crd);
			}
		}
		if (cps > 0 && crd <= 0) {
			crd = 3 * (1000 / cps);
		}
		setPasteRate(cps, crd);
		Thread t = new Thread(this);
		t.start();
		uart.attach(this);
	}

	public void setPasteRate(int cps, int cr) {
		if (cps > 0) {
			if (cps < 1) {
				cps = 1;
			}
			int dly = 1000 / cps;
			if (dly <= 0) {
				dly = 1;
			}
			paste_delay = dly;
		}
		if (cr > 0) {
			cr_delay = cr;
		}
	}

	public void paste(String s) {
		// TODO: what is the right translation here?
		fifo.add(s.replaceAll("\n", "\r"));
	}

        public void keyTyped(KeyEvent e) { }

        public void keyPressed(KeyEvent e) {
		int s = -1;
                char c = e.getKeyChar();
                int k = e.getKeyCode();
		int m = e.getModifiers();
		int l = e.getKeyLocation();
		if (l == KeyEvent.KEY_LOCATION_NUMPAD &&
				c != KeyEvent.CHAR_UNDEFINED &&
				altKeys.containsKey((int)c)) {
			s = altKeys.get((int)c);
		} else if (k == KeyEvent.VK_ENTER &&
				(m & InputEvent.CTRL_MASK) == 0) {
			// Assume if CTRL is down, must be ^J not ENTER...
			s = 0x0d;
		} else if (k == KeyEvent.VK_BACK_SPACE &&
				(m & InputEvent.SHIFT_MASK) != 0) {
			s = 0x7f;
		} else if (k == KeyEvent.VK_UP) {
			s = 0xf1;
		} else if (k == KeyEvent.VK_DOWN) {
			s = 0xf2;
		} else if (k == KeyEvent.VK_LEFT) {
			s = 0xf3;
		} else if (k == KeyEvent.VK_RIGHT) {
			s = 0xf4;
		}
		if (s < 0 && c < 0x80) {
			s = c;
		}
		// TODO: do we ever NOT consume event?
		if (s >= 0) {
			// We cannot sleep here, so risk overrun...
			_uart.put(s, false);
			e.consume();
		}
        }

        public void keyReleased(KeyEvent e) {
        }

	public void run() {
		// everything on 'fifo' is pasted text, throttle it.
		while (true) {
			String s = null;
			try {
				s = fifo.take();
			} catch (Exception ee) { }
			if (s == null) {
				continue;
			}
			for (byte b : s.getBytes()) {
				_uart.put(b & 0xff, true);
				if (paste_delay > 0) try {
					if (b == '\r') {
						Thread.sleep(cr_delay);
					} else {
						Thread.sleep(paste_delay);
					}
				} catch (Exception ee) {}
			}
		}
	}
}
