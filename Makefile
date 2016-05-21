DESTDIR = /usr

all:
	cargo build --release

install:
	install -d $(DESTDIR)/bin/
	install -d $(DESTDIR)/share/applications/
	install -d $(DESTDIR)/share/polkit-1/actions/
	install -m 755 target/release/systemd-manager $(DESTDIR)/bin/
	install -m 755 assets/systemd-manager-pkexec $(DESTDIR)/bin/
	install -m 644 assets/systemd-manager.desktop $(DESTDIR)/share/applications/
	install -m 644 assets/org.freedesktop.policykit.systemd-manager.policy $(DESTDIR)/share/polkit-1/actions/

uninstall:
	rm $(DESTDIR)/bin/systemd-manager
	rm $(DESTDIR)/bin/systemd-manager-pkexec
	rm $(DESTDIR)/share/applications/systemd-manager.desktop
	rm $(DESTDIR)/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy

ubuntu:
	sudo apt install libgtk-3-dev
	cargo build --release
	strip target/release/systemd-manager
	install -d debian/usr/bin
	install -d debian/usr/share/applications
	install -d debian/usr/share/polkit-1/actions/
	install -m 755 target/release/systemd-manager debian/usr/bin
	install -m 755 assets/systemd-manager-pkexec debian/usr/bin/
	install -m 644 assets/systemd-manager.desktop debian/usr/share/applications/
	install -m 644 assets/org.freedesktop.policykit.systemd-manager.policy debian/usr/share/polkit-1/actions
	dpkg-deb --build debian systemd-manager.deb
	sudo dpkg -i systemd-manager.deb
