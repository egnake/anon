tool_name=anon
DESTDIR ?=
src_dir=/usr/share
trigger_dir=/usr/bin
CARGO ?= cargo
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: all build install uninstall reinstall lint test clean

all: build

build:
	RUSTFLAGS="-D warnings" $(CARGO) build --release

clean:
	$(CARGO) clean

install: build
	install -d $(DESTDIR)$(trigger_dir)
	install -m 755 target/release/$(tool_name) $(DESTDIR)$(trigger_dir)/$(tool_name)
	mkdir -p $(DESTDIR)$(src_dir)/$(tool_name)/backups
	cp -a assets $(DESTDIR)$(src_dir)/$(tool_name)/
	rm -f $(DESTDIR)$(src_dir)/$(tool_name)/assets/sources/bridges.txt
	chmod 644 $(DESTDIR)$(src_dir)/$(tool_name)/assets/sources/anon.js \
		$(DESTDIR)$(src_dir)/$(tool_name)/assets/sources/bridges.default.txt
	mkdir -p $(DESTDIR)/usr/share/kali-menu/applications/ || true
	cp -f $(DESTDIR)$(src_dir)/$(tool_name)/assets/anon.desktop $(DESTDIR)/usr/share/kali-menu/applications/ || true
	mkdir -p $(DESTDIR)/usr/share/applications/ || true
	cp -f $(DESTDIR)$(src_dir)/$(tool_name)/assets/anon.desktop $(DESTDIR)/usr/share/applications/ || true

uninstall:
	rm -f $(DESTDIR)$(trigger_dir)/$(tool_name)
	rm -rf $(DESTDIR)$(src_dir)/$(tool_name)
	rm -f $(DESTDIR)/usr/share/kali-menu/applications/anon.desktop
	rm -f $(DESTDIR)/usr/share/applications/anon.desktop
	rm -f $(DESTDIR)/etc/NetworkManager/conf.d/anon-dns-lock.conf
	rm -f $(DESTDIR)/etc/apparmor.d/usr.bin.tor.anon
	rm -f $(DESTDIR)/usr/local/bin/*.anon-sandbox

reinstall:
	rm -f $(DESTDIR)$(trigger_dir)/$(tool_name)
	rm -rf $(DESTDIR)$(src_dir)/$(tool_name)/assets
	rm -f $(DESTDIR)/usr/share/kali-menu/applications/anon.desktop
	rm -f $(DESTDIR)/usr/share/applications/anon.desktop
	$(MAKE) install DESTDIR=$(DESTDIR)

lint:
	RUSTFLAGS="-D warnings" $(CARGO) check --all-targets
	bash -n setup.sh test_suite.sh
	shellcheck -S warning -e SC1090,SC1091,SC2034 setup.sh test_suite.sh

test:
	RUSTFLAGS="-D warnings" $(CARGO) test
	chmod +x test_suite.sh
	./test_suite.sh
