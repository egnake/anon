tool_name=anon
DESTDIR ?=
src_dir=/usr/share
trigger_dir=/usr/bin

.PHONY: install uninstall reinstall lint test

install:
	install -d $(DESTDIR)$(trigger_dir)
	install -m 755 anon $(DESTDIR)$(trigger_dir)/$(tool_name)
	mkdir -p $(DESTDIR)$(src_dir)/$(tool_name)/backups
	cp -a assets $(DESTDIR)$(src_dir)/$(tool_name)/
	rm -f $(DESTDIR)$(src_dir)/$(tool_name)/assets/sources/bridges.txt
	chmod 755 $(DESTDIR)$(src_dir)/$(tool_name)/assets/scripts/*
	chmod 644 $(DESTDIR)$(src_dir)/$(tool_name)/assets/sources/config \
		$(DESTDIR)$(src_dir)/$(tool_name)/assets/sources/helpers \
		$(DESTDIR)$(src_dir)/$(tool_name)/assets/sources/anon.js \
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
	bash -n anon setup.sh test_suite.sh assets/sources/config assets/sources/helpers assets/scripts/*
	shellcheck -S warning -e SC1090,SC1091,SC2034,SC2086 \
		anon setup.sh test_suite.sh assets/sources/config assets/sources/helpers assets/scripts/*

test:
	chmod +x test_suite.sh
	./test_suite.sh
