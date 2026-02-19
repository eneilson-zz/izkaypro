// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

import java.awt.*;
import javax.swing.*;

public abstract class KayproCrt extends JPanel
		implements IODevice, ScreenDumper {
	abstract void setPasteListener(PasteListener lstn);
	abstract void showSleep(boolean sleeping);
}
