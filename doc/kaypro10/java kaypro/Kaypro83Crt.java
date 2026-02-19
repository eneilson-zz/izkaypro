// Copyright (c) 2020 Douglas Miller <durgadas311@gmail.com>
import java.util.Arrays;
import java.util.Properties;
import java.io.*;
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;
import java.awt.datatransfer.Transferable;
import java.awt.datatransfer.DataFlavor;
import java.awt.datatransfer.StringSelection;

public class Kaypro83Crt extends KayproCrt
		implements AuxMemory, ActionListener, MouseListener, MouseMotionListener {
	static final long serialVersionUID = 198900000001L;

	static final int base = 0x1c;
	static final int vccmd = base;
	static final int vcstat = base;
	static final int vcrdat = base + 1;
	static final int vcdata = base + 3;

	static final int sts_Update = 0x80;
	static final int sts_LtPen = 0x40;
	static final int sts_VBlnk = 0x20;

	byte[] ram = new byte[2048]; // chars only
	int status;
	private FontMetrics _fm;
	private int _fa; //, _fd;
	private int _fw, _fh;
	float _fz;
	int curs_x;
	int curs_y;
	int curs_s;
	int curs_e;
	int curs_a;
	boolean curs_on = true;
	boolean curs_rev = true;
	int blink = 0;
	boolean asleep = false;
	javax.swing.Timer timer;
	Dimension _dim;
	int bd_width;
	boolean drag = false;
	Point dragStart;
	Point dragStop;
	int dragCount = 0;
	static final Color highlight = new Color(100, 100, 120);
	PasteListener paster = null;

	private int curReg;
	private int data;

	public Dimension getNormSize() {
		return _dim;
	}

	public Kaypro83Crt(Properties props) {
		String f = "Kaypro2.ttf";
		float fz = 16f;
		Color fc = Color.green;
		String s = props.getProperty("kaypro_font");
		if (s != null) {
			f = s;
		}
		s = props.getProperty("kaypro_font_size");
		if (s != null) {
			fz = Float.valueOf(s);
		}
		s = props.getProperty("kaypro_font_color");
		if (s != null) {
			fc = new Color(Integer.valueOf(s, 16));
		}
		_fz = fz;
		timer = new javax.swing.Timer(500, this);
		timer.start();
		setBackground(new Color(50,50,50, 255));
		setOpaque(true);
		Font font = null;
		if (f.endsWith(".ttf")) {
			try {
				File ff = new File(f);
				java.io.InputStream ttf;
				if (ff.exists()) {
					ttf = new FileInputStream(ff);
				} else {
					ttf = VirtualKaypro.class.getResourceAsStream(f);
				}
				if (ttf != null) {
					font = Font.createFont(Font.TRUETYPE_FONT, ttf);
					font = font.deriveFont(fz);
				}
			} catch (Exception ee) {
				font = null;
			}
		} else {
			font = new Font(f, Font.PLAIN, (int)fz);
		}
		if (font == null) {
			System.err.println("Missing font \"" +
					f + "\", using default");
			font = new Font("Monospaced", Font.PLAIN, 10);
		}
		bd_width = 3;
		setupFont(font);
		setForeground(fc);
		addMouseListener(this);
		reset();
	}

	private void setupFont(Font f) {
		super.setFont(f);
		_fm = getFontMetrics(f);
		_fa = _fm.getAscent();
		//_fd = _fm.getDescent();
		_fw = _fm.charWidth('M');
		_fh = _fm.getHeight();

		_fh = (int)Math.floor(_fz);
		// leave room for borders...
		_dim = new Dimension(_fw * 80 + 2 * bd_width, _fh * 24 + 2 * bd_width);
		//System.err.format("%s ascent=%d width=%d height=%d dim=(%d,%d)\n",
		//	f.getName(), _fa, _fw, _fh, _dim.width, _dim.height);
		super.setPreferredSize(_dim);
	}

	public void setPasteListener(PasteListener lstn) {
		paster = lstn;
	}

	public void showSleep(boolean sleeping) {
		asleep = sleeping;
		repaint();
	}

	// This simulates the funky address translation done in hardware,
	// converting a 12-bit "virtual address" into an 11-bit physical.
	// This allows accessing each line on a 128-byte boundary, both
	// in software and also video refresh hardware, while packing
	// the display characters into 2K RAM.
	private int getAddr(int adr) {
		int pa = (adr & 0x00f) | ((adr & 0x380) >> 1);
		if ((adr & 0x040) != 0) {
			pa |= ((adr & 0xc00) >> 6) | 0x600;
		} else {
			pa |= (adr & 0x030) | ((adr & 0xc00) >> 1);
		}
		return pa;
	}

	// AuxMemory - CRT RAM mapped with ROM
	public int base() { return 0x3000; }	// base address of this memory
	public int end() { return 0x4000; }	// end of memory
	public int read(int adr) {
		int pa = getAddr(adr);
		return ram[pa] & 0xff;
	}
	public void write(int adr, int val) {
		int pa = getAddr(adr);
		ram[pa] = (byte)val;
		// update screen...
		repaint();
	}

	// No actual IODevice, but KayproCrt requires something.
	public void reset() {}
	public int getBaseAddress() { return 0; }
	public int getNumPorts() { return 0; }
	public int in(int port) { return 0; }
	public void out(int port, int value) {}
	public String getDeviceName() { return "NONE"; }
	public String dumpDebug() { return ""; }

	private void paintHighlight(Graphics2D g2d) {
		int x0 = (int)dragStart.getX();
		int y0 = (int)dragStart.getY();
		int x1 = 80;
		int y1 = (int)dragStop.getY();
		g2d.setColor(highlight);
		for (int y = y0; y < y1; ++y) {
			if (y + 1 == y1) {
				x1 = (int)dragStop.getX();
			}
			g2d.fillRect(x0 * _fw + bd_width, y * _fh + bd_width,
					(x1 - x0) * _fw, _fh);
			x0 = 0;
		}
		g2d.setColor(getForeground());
	}

	private char[] b = new char[1];

	public void paint(Graphics g) {
		super.paint(g);
		int blnk = blink;
		if (asleep) {
			return;
		}
		Graphics2D g2d = (Graphics2D)g;
		if (drag) {
			paintHighlight(g2d);
		}
		int gx = bd_width;
		int gy = _fa + bd_width;
		int va = 0;
		int pa;
		for (int y = 0; y < 24; ++y) {
			for (int x = 0; x < 80; ++x) {
				pa = getAddr(va);
				if ((ram[pa] & 0x80) != 0 && (blnk & 0x01) == 0) {
					b[0] = ' ';
				} else {
					b[0] = (char)(ram[pa] & 0x7f);
					if (b[0] < ' ') b[0] += 0x100;
				}
				g2d.drawChars(b, 0, 1, gx, gy);
				gx += _fw;
				++va;
			}
			va += 48;	// 128 bytes per line, 80 displayable...
			gx = bd_width;
			gy += _fh;
		}
	}

	private int getAddress(Point p) {
		int sa = 0x0000;
		int x = (int)p.getX();
		int y = (int)p.getY();
		sa += (y * 128 + x);
		sa &= 0x07ff;
		return sa;
	}

	// Note, p1 points to *next* line, in order to draw
	// proper highlight. But here we need to avoid copy
	// of an extra line.
	private String getRegion(Point p0, Point p1) {
		String s = "";
		int x = (int)p0.getX();
		int a0 = getAddress(p0);
		int a1 = getAddress(p1) - 128;
		while (a0 != a1) {
			int c = ram[getAddr(a0)] & 0x07f;
			if (c > '~' || c < ' ') {
				c = '.';
			}
			s += (char)c;
			if (++x >= 80) {
				s += '\n';
				x = 0;
				a0 += 48; // 128 bytes, 80 displayed
			}
			++a0;
			a0 &= 0x0fff;
		}
		return s.replaceAll(" +\n", "\n");
	}

	// TODO: does this need to reconstruct graphics/reverse-video
	// ESC sequences?
	public String getScreen(int line) {
		String str = "";
		int a = line * 128;
		if (a >= ram.length) {
			return str;
		}
		int b = 0;
		int e = 23;
		if (line >= 0) {
			b = e = line;
		}
		// TODO: extract character and format into lines...
		return str;
	}

	public String dumpScreen(int line) {
		String str = "";
		int b = 0;
		int e = 23;
		if (line >= 0) {
			b = e = line;
		}
		int a = b * 128;
		if (a >= ram.length) {
			return str;
		}
		// TODO: extract character and format into lines...
		for (; b < e; ++b) {
			for (int x = 0; x < 80; ++x) {
				int c = ram[getAddr(a)] & 0x07f;
				if (c > '~' || c < ' ') {
					c = '.';
				}
				str += (char)c;
				++a;
			}
			str += '\n';
			a += 48; // 128 bytes, 80 displayed
		}
		return str;
	}

	public void actionPerformed(ActionEvent e) {
		if (e.getSource() == timer) {
			if (asleep) {
				return;
			}
			blink = (blink + 1) & 0x03;
			if (dragCount > 0) {
				--dragCount;
				if (dragCount == 0) {
					drag = false;
				}
			}
			repaint();
			return;
		}
	}

	private Point charStart(Point mp) {
		Point p = new Point();
		p.x = (int)Math.floor((mp.getX() - bd_width) / _fw);
		p.y = (int)Math.floor((mp.getY() - bd_width) / _fh);
		return p;
	}

	private Point charEnd(Point mp) {
		Point p = new Point();
		p.x = (int)Math.ceil((mp.getX() - bd_width) / _fw);
		p.y = (int)Math.ceil((mp.getY() - bd_width) / _fh);
		return p;
	}

	public void mouseClicked(MouseEvent e) {
		if (e.getButton() != MouseEvent.BUTTON2) {
			return;
		}
		if (paster == null) {
			return;
		}
		Transferable t = Toolkit.getDefaultToolkit().
				getSystemClipboard().getContents(null);
		try {
			if (t != null && t.isDataFlavorSupported(DataFlavor.stringFlavor)) {
				String text = (String)t.getTransferData(DataFlavor.stringFlavor);
				paster.paste(text);
			}
		} catch (Exception ee) {
		}
	}
	public void mouseEntered(MouseEvent e) { }
	public void mouseExited(MouseEvent e) { }
	public void mousePressed(MouseEvent e) {
		if (e.getButton() != MouseEvent.BUTTON1) {
			return;
		}
		drag = true;
		dragCount = 0;
		dragStop = dragStart = charStart(e.getPoint());
		addMouseMotionListener(this);
	}
	public void mouseReleased(MouseEvent e) {
		if (e.getButton() != MouseEvent.BUTTON1) {
			return;
		}
		dragCount = 10;
		repaint();
		removeMouseMotionListener(this);
		if (dragStart.equals(dragStop)) {
			return;
		}
		StringSelection ss = new StringSelection(getRegion(dragStart, dragStop));
		Toolkit.getDefaultToolkit().getSystemClipboard().setContents(ss, null);
	}

	public void mouseDragged(MouseEvent e) {
		dragStop = charEnd(e.getPoint());
		repaint();
	}
	public void mouseMoved(MouseEvent e) { }
}
