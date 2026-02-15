// Copyright (c) 2017 Douglas Miller
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;
import java.util.Properties;
import java.util.Vector;
import java.util.Map;
import java.util.HashMap;
import javax.swing.border.*;

class KayproPowerLED extends JPanel {
	int height;
	int width;
	int vcenter;
	LED led;
	public KayproPowerLED(int wd, int ht, Font ft) {
		super();
		setPreferredSize(new Dimension(wd, ht));
		setFont(ft);
		setOpaque(false);
		setForeground(Color.white);
		width = wd;
		height = ht;
		vcenter = ht / 2;
		led = new RoundLED(LED.Colors.RED);
		led.set(true);
	}
	public void paint(Graphics g) {
		super.paint(g);
		Graphics2D g2d = (Graphics2D)g;
		g2d.addRenderingHints(new RenderingHints(
			RenderingHints.KEY_ANTIALIASING,
			RenderingHints.VALUE_ANTIALIAS_ON));
		g2d.drawString("~", 20, vcenter - 20);
		g2d.drawString("POWER", 0, vcenter + 20);
		g2d.translate(18, vcenter - 16);
		led.paint(g2d);
	}
}

class LEDPanel extends JPanel {
	LEDPane[] panes;
	public static final Font font = new Font("Monospaced", Font.BOLD, 16);

	public LEDPanel(int w, int h, int rows, Color bg) {
		super();
		panes = new LEDPane[rows];
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
		setOpaque(false);
		setPreferredSize(new Dimension(w, h));
		for (int y = 0; y < rows; ++y) {
			LEDPane pn = new LEDPane(bg);
			if (y == 0) {
				pn.setPreferredSize(new Dimension(w, h / 3));
				gb.setConstraints(pn, gc);
				add(pn);
				++gc.gridy;
				KayproPowerLED kp = new KayproPowerLED(w, h / 3, font);
				gb.setConstraints(kp, gc);
				add(kp);
			} else {
				pn.setPreferredSize(new Dimension(w, h / 3 / (rows - 1)));
				gb.setConstraints(pn, gc);
				add(pn);
			}
			panes[y] = pn;
			++gc.gridy;
		}
	}

	public LEDPane getPane(int row) {
		if (row < panes.length) {
			return panes[row]; // should not be null
		} else {
			return null;
		}
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
			lb.setText(String.format(fmt, lb.getText()));
		}
	}

	public void rePad(int num) {
		++num; // one component is the label...
		for (LEDPane pn : panes) {
			if (pn == null) continue;
			int n = pn.getComponentCount();
			if (n <= 0 || n >= num) continue;
			for (int x = n; x < num; ++x) {
				JPanel p = new JPanel();
				p.setPreferredSize(RectLED.getDim());
				pn.add(p);
			}
		}
	}
}

public class KayproFrontSide extends JPanel
		implements LEDHandler {
	static final long serialVersionUID = 198900000004L;

	BezelRoundedRectangle _crtshape;
	LEDPanel _ledspace;
	int _ledy;
	int _ledmax = 1;
	int _ledwid = 5;
	String _ledfmt = "%5s";
	int gaps;
	float rounding;
	int width, height;
	int offset;

	public KayproFrontSide(JFrame main, JPanel crt,
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
		gc.gridwidth = 5;
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
		_ledspace = new LEDPanel(nmwid - 20, dim.height, 4,
					getBackground().brighter().brighter());
		_ledy = 0;
		gc.gridx = 3;
		gridbag.setConstraints(_ledspace, gc);
		add(_ledspace);

		pan = new JPanel();
		pan.setPreferredSize(new Dimension(gaps, gaps));
		pan.setOpaque(false);
		gc.gridx = 4;
		gridbag.setConstraints(pan, gc);
		add(pan);

		++gc.gridy;
		gc.gridx = 0;

		pan = new JPanel();
		pan.setPreferredSize(new Dimension(gaps, gaps));
		pan.setOpaque(false);
		gc.gridwidth = 5;
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

	public LED[] registerLEDs(String drive, int num, LED.Colors[] colors) {
		LED[] leds = new LED[num];
		if (_ledmax < num) {
			_ledmax = num;
			_ledspace.rePad(num);
		}
		if (drive.length() > 20) {
			drive = drive.substring(0, 20);
		}
		if (drive.length() > _ledwid) {
			_ledwid = drive.length();
			_ledfmt = String.format("%%%ds", _ledwid);
			_ledspace.reFmt(_ledfmt);
		}
		LEDPane pn = _ledspace.getPane(_ledy++);
		if (pn != null) {
			pn.setName(drive);
			JLabel lb = new JLabel(String.format(_ledfmt, drive));
			lb.setForeground(Color.white);
			lb.setFont(LEDPanel.font);
			pn.add(lb);
		}
		for (int x = 0; x < num; ++x) {
			leds[x] = new RectLED(colors[x]);
			if (pn != null) {
				pn.add(leds[x]);
			}
		}
		if (pn != null) {
			for (int x = num; x < _ledmax; ++x) {
				JPanel p = new JPanel();
				p.setPreferredSize(RectLED.getDim());
				pn.add(p);
			}
		}
		return leds;
	}
}
