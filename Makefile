tool_name=anon

src_dir=/usr/share
trigger_dir=/usr/bin

install:

	install -m 755 anon $(trigger_dir)
	mkdir -p $(src_dir)/$(tool_name)/backups
	cp -vr assets $(src_dir)/$(tool_name)
	mkdir -p /usr/share/kali-menu/applications/ || true\n\tcp -vr  $(src_dir)/$(tool_name)/assets/anon.desktop /usr/share/kali-menu/applications/ || true
	mkdir -p /usr/share/applications/ || true\n\tmv -v  $(src_dir)/$(tool_name)/assets/anon.desktop /usr/share/applications/ || true


uninstall:

	rm -Rf $(trigger_dir)/$(tool_name)
	rm -Rf $(src_dir)/$(tool_name)
	rm -Rf $(src_dir)/kali-menu/applications/anon.desktop
	rm -Rf $(src_dir)/applications/anon.desktop
	
reinstall:

	rm -Rf $(trigger_dir)/$(tool_name)
	rm -Rf $(src_dir)/$(tool_name)
	rm -Rf $(src_dir)/kali-menu/applications/anon.desktop
	rm -Rf $(src_dir)/applications/anon.desktop
	install -m 755 anon $(trigger_dir)
	mkdir -p $(src_dir)/$(tool_name)/backups
	cp -vr assets $(src_dir)/$(tool_name)
	mkdir -p /usr/share/kali-menu/applications/ || true\n\tcp -vr  $(src_dir)/$(tool_name)/assets/anon.desktop /usr/share/kali-menu/applications/ || true
	mv -vr  $(src_dir)/$(tool_name)/assets/anon.desktop /usr/share/applications/
