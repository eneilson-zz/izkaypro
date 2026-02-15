// Copyright (c) 2017 Douglas Miller <durgadas311@gmail.com>

public interface GppListener {
	int interestedBits();
	void gppNewValue(int gpio);
}
