// Copyright (c) 2020 Douglas Miller <durgadas311@gmail.com>

public interface AuxMemory {
	int base();	// base address of this memory
	int end();	// end of memory (+1)
	int read(int adr);
	void write(int adr, int val);
}
