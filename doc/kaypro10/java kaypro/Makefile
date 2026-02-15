CLASS_PATH = .:z80cpu/z80core.jar:diablo630/Diablo630Embedded.jar

JAVAS = $(wildcard *.java)
CLASSES = $(subst .java,.class,$(JAVAS))
DEST = durgadas.com:/http/durgadas.com/sims/kaypro
SEBHC = durgadas.com:/http/durgadas.com/sebhc/kaypro

all: z80cpu/z80core.jar diablo630/Diablo630Embedded.jar $(CLASSES)

%.class: %.java
	javac -cp $(CLASS_PATH) $?

TtySerial.class: TtySerial.java jSerialComm-2.6.2.jar
	javac -cp .:jSerialComm-2.6.2.jar $<

jSerialComm-2.6.2.jar:
	ln -s ~/Downloads/jSerialComm-2.6.2.jar .

run:
	java -cp $(CLASS_PATH) VirtualKaypro

VirtualKaypro.jar: all temp
	echo "Main-Class: VirtualKaypro" >Manifest.txt
	jar -cmf Manifest.txt VirtualKaypro.jar *.class \
		81-*.rom *.ttf *.wav \
		-C temp .
	rm -rf temp

temp: __FRC__
	mkdir -p temp
	rm -rf temp/*
	cp -r docs temp/
	./mkAbout >temp/docs/About.html
	cd temp; jar -xf ../z80cpu/z80core.jar
	rm -rf temp/META-INF
	cd temp; jar -xf ../diablo630/Diablo630Embedded.jar
	rm -rf temp/META-INF

z80cpu/z80core.jar:
	make -C z80cpu z80core.jar

diablo630/Diablo630Embedded.jar:
	make -C diablo630 Diablo630Embedded.jar

jar: VirtualKaypro.jar

src:
	cd ..; tar -czf kaypro84/vkaypro.tgz kaypro84/Makefile kaypro84/mkAbout \
		kaypro84/LICENSE \
		kaypro84/*.java \
		kaypro84/*.rom \
		kaypro84/*.ttf \
		kaypro84/*.wav \
		kaypro84/docs/*

www: __FRC__
	rsync -Wuv ../kicad/*.pdf $(DEST)
	rsync -Wuv www/*.* $(DEST)
	rsync -rWuv www/downloads/. $(SEBHC)/.

ship:
	rsync -Wuv LICENSE *.jar vkaypro.tgz z80cpu/z80cpu.tgz diablo630/diablo630.tgz \
		$(SEBHC)
	rsync -rWuv --exclude='.[a-z]*' \
		../disks/. $(SEBHC)/disks/.
	rsync -rWuv ../configs/. $(SEBHC)/configs/.

__FRC__:
