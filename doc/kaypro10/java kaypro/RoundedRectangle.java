// Copyright (c) 2017 Douglas Miller
import java.awt.*;
import java.awt.event.*;
import javax.swing.*;

class RoundedRectangle extends JPanel {
	//static final long serialVersionUID = 198900000003L;

	Color color;
	Color shadow;
	Color highlt;
	int _y, _x;
	float _w;
	Font _font;
	int _arch, _arcw;

	public RoundedRectangle(Color bg, boolean reverse,
			float x, float y, float w, float h,
			float arcw, float arch) {
		if (x == y) {}
		color = bg;
		if (reverse) {
			highlt = bg.darker();
			shadow = bg.brighter();
		} else {
			shadow = bg.darker();
			highlt = bg.brighter();
		}
		_arch = Math.round(arch / 2);
		_arcw = Math.round(arcw);
		_x = Math.round(w);
		_y = Math.round(h);
		setPreferredSize(new Dimension(_x, _y));
		setOpaque(false);
	}

	public void paint(Graphics g) {
		Graphics2D g2d = (Graphics2D)g;
		g2d.addRenderingHints(new RenderingHints(
			RenderingHints.KEY_ANTIALIASING,
			RenderingHints.VALUE_ANTIALIAS_ON));
		g2d.setColor(color);
		g2d.fillRoundRect(0, 0, _x, _y, _arcw, _arcw);
		g2d.setStroke(new BasicStroke(3f, BasicStroke.CAP_BUTT, BasicStroke.JOIN_MITER));
		g2d.setColor(highlt);
		g2d.drawLine(0, _arch, 0, _y - _arch);
		g2d.drawLine(_arch, 0, _x - _arch, 0);
		g2d.drawArc(0, 0, _arcw, _arcw, 180, -90);
		g2d.drawArc(_x - _arcw - 1, 0, _arcw, _arcw, 90, -45);
		g2d.drawArc(0, _y - _arcw - 1, _arcw, _arcw, 180, 45);
		g2d.setColor(shadow);
		g2d.drawLine(_x - 1, _arch, _x - 1, _y - _arch);
		g2d.drawLine(_arch, _y - 1, _x - _arch, _y - 1);
		g2d.drawArc(_x - _arcw - 1, _y - _arcw - 1, _arcw, _arcw, 0, -90);
		g2d.drawArc(_x - _arcw - 1, 0, _arcw, _arcw, 0, 45);
		g2d.drawArc(0, _y - _arcw - 1, _arcw, _arcw, -90, -45);
		super.paint(g2d);
	}
}
