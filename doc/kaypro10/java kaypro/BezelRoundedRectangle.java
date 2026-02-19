// Copyright (c) 2017 Douglas Miller

import java.awt.*;
import java.awt.event.*;
import javax.swing.*;

class BezelRoundedRectangle extends JPanel {
	//static final long serialVersionUID = 198900000003L;
	java.awt.geom.RoundRectangle2D.Float shape;
	Color color;
	Color bezel;
	Color shadow;
	Color highlt;
	int _sy, _sx;
	int _y, _x;
	float _w;
	Font _font;
	int _arch, _arcw;
	Polygon lpoly;
	int _ox, _oy, _ex, _ey;

	public BezelRoundedRectangle(Color bg, Color bez, boolean reverse,
			float x, float y, float w, float h,
			float arcw, float arch) {
		color = bg;
		bezel = bez;
		if (reverse) {
			highlt = bez.darker();
			shadow = bez.brighter();
		} else {
			shadow = bez.darker();
			highlt = bez.brighter();
		}
		_arch = Math.round(arch / 2);
		_arcw = Math.round(arcw);
		_ox = Math.round(x);
		_oy = Math.round(y);
		_ex = Math.round(w);
		_ey = Math.round(h);
		int gap = Math.round(w / 100);
		if (gap < 3) gap = 3;
		_x = gap;
		_y = gap;
		_sx = _ex - 2 * _x;
		_sy = _ey - 2 * _y;
		setPreferredSize(new Dimension(_ex, _ey));
		setOpaque(false);
		lpoly = new Polygon();
		lpoly.addPoint(_ex, 0);
		lpoly.addPoint(_ex, _ey);
		lpoly.addPoint(0, _ey);
		lpoly.addPoint(0 + _arcw, _ey - _arcw);
		lpoly.addPoint(_ex - _arcw, 0 + _arcw);
	}

	public void paint(Graphics g) {
		Graphics2D g2d = (Graphics2D)g;
		g2d.addRenderingHints(new RenderingHints(
			RenderingHints.KEY_ANTIALIASING,
			RenderingHints.VALUE_ANTIALIAS_ON));
		g2d.setColor(highlt);
		g2d.fillRect(_ox, _oy, _ex - _ox, _ey - _oy);
		g2d.setColor(shadow);
		g2d.fillPolygon(lpoly);
		g2d.setColor(bezel);
		g2d.drawLine(_ox, _oy, _ox + _arcw, _oy + _arcw);
		g2d.drawLine(_ex, _ey, _ex - _arcw, _ey - _arcw);
		g2d.setColor(color);
		g2d.fillRoundRect(_x, _y, _sx, _sy, _arcw, _arcw);
		super.paint(g2d);
	}
}
