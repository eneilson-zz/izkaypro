// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.awt.*;
import javax.swing.*;

public class VRectLED extends LED {
	public VRectLED(Colors color) {
		super(color);
		setBackground(off);
		setPreferredSize(new Dimension(8, 16));
	}

	public static Dimension getDim() { return new Dimension(8, 16); }

	public void set(boolean onf) {
		if (onf) {
			setBackground(on);
		} else {
			setBackground(off);
		}
		repaint();
	}
}
