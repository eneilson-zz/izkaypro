// Copyright (c) 2020 Douglas Miller <durgadas311@gmail.com>
import java.awt.*;
import javax.swing.*;
import java.io.*;
import java.util.Properties;
import java.lang.reflect.Field;
import java.util.Properties;

public class VirtualKaypro {
	private static JFrame front_end;

	public static void main(String[] args) {
		Properties props = new Properties();
		String config = null;
		if (args.length > 0) {
			File f = new File(args[0]);
			if (f.exists()) {
				config = f.getAbsolutePath();
			} else {
				props.setProperty("kaypro_model", args[0]);
			}
		} else {
			config = System.getenv("KAYPRO_CONFIG");
			if (config == null) {
				config = "vkayprorc";
				File f = new File(config);
				if (f.exists()) {
					config = f.getAbsolutePath();
				} else {
					config = System.getProperty("user.home") + "/." + config;
				}
			}
		}
		if (config != null) {
			try {
				FileInputStream cfg = new FileInputStream(config);
				props.setProperty("configuration", config);
				props.load(cfg);
				cfg.close();
			} catch(Exception ee) {
				config = null;
			}
		}
		Interruptor.Model model = Kaypro.setModel(props);
		if (model == Interruptor.Model.UNKNOWN) {
			System.exit(1);
		}
		String title = "Virtual Kaypro " + model.name().substring(1) + " Computer";

		KayproCrt crt = null;
		if (model == Interruptor.Model.K2 ||
				model == Interruptor.Model.K4) {
			crt = new Kaypro83Crt(props);
		}
		// TODO: configure CRT based on model...
		// For now, all supported models use compatible CRT.
		if (crt == null) {
			crt = new Kaypro84Crt(props);
		}
		front_end = new JFrame(title);
		front_end.getContentPane().setName("Kaypro Emulator");
		front_end.setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
		front_end.getContentPane().setBackground(new Color(25, 25, 25));
		// This allows TAB to be sent
		front_end.setFocusTraversalKeysEnabled(false);
		JPanel pn;
		LEDHandler lh;
		Kaypro10FrontSide front;
		if (Kaypro.hasWin()) {
			Kaypro10FrontSide k10 = new Kaypro10FrontSide(front_end, crt, props);
			lh = k10;
			pn = k10;
		} else {
			KayproFrontSide fs = new KayproFrontSide(front_end, crt, props);
			lh = fs;
			pn = fs;
		}

		Kaypro kaypro = new Kaypro(props, lh, (KayproCrt)crt);
		// All LEDs should be registered now...
		KayproOperator op = new KayproOperator(front_end, props, crt, lh, model);
		op.setCommander(kaypro.getCommander());

		front_end.addKeyListener(kaypro.getKeyboard());
		crt.setPasteListener(kaypro.getKeyboard());
		// TODO:
		// kbd.addResetListener(term);
		// kbd.addResetListener(op);

		if (false) {
			front_end.add(crt);
		} else {
			front_end.add(pn);
		}

		front_end.pack();
		front_end.setVisible(true);

		kaypro.start(); // spawns its own thread... returns immediately
	}
}
