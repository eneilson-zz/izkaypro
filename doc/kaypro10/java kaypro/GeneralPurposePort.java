// Copyright (c) 2020 Douglas Miller <durgadas311@gmail.com>

public interface GeneralPurposePort {
        public int get();
        public void addGppProvider(GppProvider inp);
        public void addGppListener(GppListener lstn);
        public String dumpDebug();
}
