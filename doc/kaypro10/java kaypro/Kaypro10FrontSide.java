// Copyright (c) 2017 Douglas Miller
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;
import java.util.Properties;
import java.util.Vector;
import java.util.Map;
import java.util.HashMap;
import javax.swing.border.*;

class LabeledRectLED extends LEDPane {
	LED led;
	public LabeledRectLED(int wd, int ht, Color bg,
			Font ft, String lbl, LED.Colors clr) {
		super(bg);
		setPreferredSize(new Dimension(wd, ht));
		setFont(ft);
		setForeground(Color.black);
		JPanel pn = new JPanel();
		pn.setPreferredSize(new Dimension(wd, 60));
		pn.setOpaque(false);
		add(pn);
		led = new VRectLED(clr);
		add(led);
		JLabel lb = new JLabel(lbl);
		lb.setPreferredSize(new Dimension(wd, 15));
		lb.setHorizontalAlignment(SwingConstants.CENTER);
		add(lb);
	}
	public LED getLED() { return led; }
}

class LabeledRoundLED extends LEDPane {
	LED led;
	public LabeledRoundLED(int wd, int ht, Color bg,
			Font ft, String[] lbl, LED.Colors clr) {
		super(bg);
		setPreferredSize(new Dimension(wd, ht));
		setForeground(Color.white);
		JPanel pn = new JPanel();
		pn.setPreferredSize(new Dimension(wd, 15));
		pn.setOpaque(false);
		add(pn);
		led = new RoundLED(clr);
		add(led);
		for (int x = 0; x < lbl.length; ++x) {
			JLabel lb = new JLabel(lbl[x]);
			lb.setFont(ft);
			lb.setForeground(Color.white);
			lb.setPreferredSize(new Dimension(wd, ft.getSize() - 2));
			lb.setHorizontalAlignment(SwingConstants.CENTER);
			add(lb);
		}
	}
	public LED getLED() { return led; }
}

class K10LEDPanel extends JPanel {
	LEDPane[] panes;
	LED[] leds;
	int nLeds;
	public static final Font font = new Font("SansSerif", Font.BOLD, 10);
	// From photo, BG = new Color(141, 156, 163);
	// From photo, LN = new Color(193, 206, 214);
	Color bg;
	Color ln;
	int wd;
	int ht;
	int dh;
	JPanel ledPn;

	public K10LEDPanel(int w, int h, int rows, Color bg, Color bh) {
		super();
		wd = w;
		ht = h;
		dh = ht / 12;
		this.bg = bg;
		ln = bg.brighter();
		panes = new LEDPane[rows];
		leds = new LED[rows];
		GridBagLayout gb = new GridBagLayout();
		GridBagConstraints gc = new GridBagConstraints();
		gc.fill = GridBagConstraints.NONE;
		gc.gridx = 0;
		gc.gridy = 0;
		gc.weightx = 0;
		gc.weighty = 0;
		gc.gridwidth = 1;
		gc.gridheight = 1;
		setLayout(gb);
		setOpaque(false); // background painted by this, not JPanel
		setPreferredSize(new Dimension(w, h));
		LabeledRoundLED kp = new LabeledRoundLED(w / 3, h, bh, font,
			new String[]{"POWER"}, LED.Colors.RED);
		kp.getLED().set(true);
		gb.setConstraints(kp, gc);
		add(kp);
		++gc.gridx;
		LabeledRoundLED led1 = new LabeledRoundLED(w / 3, h, bh, font,
			new String[]{"10 MB", "READY"}, LED.Colors.RED);
		gb.setConstraints(led1, gc);
		add(led1);
		ledPn = led1;
		++gc.gridx;
		LabeledRectLED led2 = new LabeledRectLED(w / 3, h, bh, font, "floppy", LED.Colors.RED);
		gb.setConstraints(led2, gc);
		add(led2);
		++gc.gridx;
		nLeds = 0;
		panes[nLeds] = led1;
		leds[nLeds] = led1.getLED();
		++nLeds;
		panes[nLeds] = led2;
		leds[nLeds] = led2.getLED();
		++nLeds;
	}

	public void paint(Graphics g) {
		Graphics2D g2d = (Graphics2D)g;
		g2d.setColor(bg);
		g2d.fillRect(0, 0, wd, ht);
		g2d.setColor(ln);
		for (int y = 32; y < ht; y += dh) {
			g2d.drawLine(0, y, wd, y);
		}
		super.paint(g);
	}

	public LED getLED(int row) {
		return leds[row];
	}

	public LEDPane getPane(int row) {
		if (row < panes.length) {
			return panes[row]; // should not be null
		} else {
			return null;
		}
	}

	public void newPane(String name, LED[] leds) {
		JPanel p = new JPanel();
		p.setPreferredSize(new Dimension(wd, 20));
		p.setOpaque(false);
		ledPn.add(p);
		p = new JPanel();
		p.setOpaque(false);
		for (int y = 0; y < leds.length; ++y) {
			p.add(leds[y]);
		}
		ledPn.add(p);
		ledPn.add(new JLabel(name));
	}

	public void putMap(String drive, JMenuItem mi) {
		for (LEDPane pn : panes) {
			if (pn == null) continue;
			if (drive.equals(pn.getName())) {
				pn.setMenuItem(mi);
			}
		}
	}

	public void reFmt(String fmt) {
		for (LEDPane pn : panes) {
			if (pn == null) continue;
			// Component #0 is always the label...
			// But, it might not be there yet...
			int n = pn.getComponentCount();
			if (n < 1) {
				continue;
			}
			JLabel lb = (JLabel)pn.getComponent(0);
			//lb.setText(String.format(fmt, lb.getText()));
		}
	}

	public void rePad(int num) {
		++num; // one component is the label...
		for (LEDPane pn : panes) {
			if (pn == null) continue;
			int n = pn.getComponentCount();
			if (n <= 0 || n >= num) continue;
			for (int x = n; x < num; ++x) {
				//pn.add(LED.blank());
			}
		}
	}
}

public class Kaypro10FrontSide extends JPanel
		implements LEDHandler {
	static final long serialVersionUID = 198900000004L;

	BezelRoundedRectangle _crtshape;
	K10LEDPanel _ledspace;
	int _ledy;
	int _ledmax = 1;
	int _ledwid = 5;
	String _ledfmt = "%5s";
	int gaps;
	float rounding;
	int width, height;
	int offset;

	public Kaypro10FrontSide(JFrame main, JPanel crt,
			Properties props) {
		super();
		if (props == null) {}
		setBackground(main.getContentPane().getBackground());
		setOpaque(true);
		Dimension dim = crt.getPreferredSize();
		gaps = dim.width / 50;
		if (gaps < 5) gaps = 5;
		offset = dim.width / 40;
		rounding = offset * 2;
		width = Math.round(dim.width + 2 * offset);
		height = Math.round(dim.height + 2 * offset);

		GridBagLayout gridbag = new GridBagLayout();
		setLayout(gridbag);
		GridBagConstraints gc = new GridBagConstraints();
		gc.fill = GridBagConstraints.NONE;
		gc.gridx = 0;
		gc.gridy = 0;
		gc.weightx = 0;
		gc.weighty = 0;
		gc.gridwidth = 1;
		gc.gridheight = 1;
		gc.anchor = GridBagConstraints.NORTH;
		JPanel pan;
		pan = new JPanel();
		pan.setPreferredSize(new Dimension(gaps, gaps));
		pan.setOpaque(false);
		gc.gridx = 0;
		gc.gridwidth = 3;
		gridbag.setConstraints(pan, gc);
		gc.gridwidth = 1;
		add(pan);
		++gc.gridy;

		pan = new JPanel();
		pan.setPreferredSize(new Dimension(gaps, gaps));
		pan.setOpaque(false);
		gc.gridx = 0;
		gridbag.setConstraints(pan, gc);
		add(pan);

		_crtshape = new BezelRoundedRectangle(crt.getBackground(),
			main.getContentPane().getBackground(), true,
			0f, 0f, width, height,
			rounding, rounding);
		GridBagLayout gb = new GridBagLayout();
		_crtshape.setLayout(gb);
		_crtshape.setOpaque(false);
		gridbag.setConstraints(crt, gc);
		_crtshape.add(crt);
		gc.gridx = 1;
		gridbag.setConstraints(_crtshape, gc);
		add(_crtshape);

		pan = new JPanel();
		pan.setPreferredSize(new Dimension(gaps, gaps));
		pan.setOpaque(false);
		gc.gridx = 2;
		gridbag.setConstraints(pan, gc);
		add(pan);

		int nmwid = width / 3;
		int nmhgh = height;

		
		// TODO: scan properties and count number of drives?
		_ledspace = new K10LEDPanel(nmwid, dim.height + 2 * (gaps + offset), 8,
					new Color(141, 156, 163),
					new Color(161, 176, 183, 128));
		_ledy = 0;
		gc.gridheight = 3;
		gc.gridx = 3;
		gc.gridy = 0;
		gridbag.setConstraints(_ledspace, gc);
		add(_ledspace);
		gc.gridy = 1;
		gc.gridheight = 1;

		++gc.gridy;
		gc.gridx = 0;

		pan = new JPanel();
		pan.setPreferredSize(new Dimension(gaps, gaps));
		pan.setOpaque(false);
		gc.gridwidth = 3;
		gridbag.setConstraints(pan, gc);
		gc.gridwidth = 1;
		add(pan);
	}

	public void setMenuItem(String drive, JMenuItem mi) {
		_ledspace.putMap(drive, mi);
	}

	public LED registerLED(String drive, LED.Colors color) {
		LED led[] = registerLEDs(drive, 1, new LED.Colors[]{color});
		return led[0];
	}

	public LED registerLED(String drive) {
		return registerLED(drive, LED.Colors.RED);
	}

	// TODO: how to allow more LEDs, per-device and per-system
	public LED[] registerLEDs(String drive, int num, LED.Colors[] colors) {
		LED[] leds = new LED[num];
		int x = _ledy++;
		LEDPane pn = _ledspace.getPane(x);
		if (pn != null) {
			pn.setName(drive);
			leds[0] = _ledspace.getLED(x);
		} else {
			for (int y = 0; y < num; ++y) {
				leds[y] = new RectLED(colors[y]);
			}
			_ledspace.newPane(drive, leds);
		}
		// TODO: might return zero LEDs, a problem for drevices.
		return leds;
	}
}
