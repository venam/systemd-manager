DESTDIR = /usr

all:
	cargo build --release

install:
	cp target/release/systemd-manager $(DESTDIR)/bin/
	cp assets/systemd-manager-pkexec $(DESTDIR)/bin/
	cp assets/systemd-manager.desktop $(DESTDIR)/share/applications/
	cp assets/org.freedesktop.policykit.systemd-manager.policy $(DESTDIR)/share/polkit-1/actions/
	
uninstall:
	rm $(DESTDIR)/bin/systemd-manager
	rm $(DESTDIR)/bin/systemd-manager-pkexec
	rm $(DESTDIR)/share/applications/systemd-manager.desktop
	rm $(DESTDIR)/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy

ubuntu:
	sudo apt install libgtk-3-dev
	cargo build --release
	mkdir -p debian/usr/bin
	mkdir -p debian/usr/share/applications
	mkdir -p debian/usr/share/polkit-1/actions/
	cp target/release/systemd-manager debian/usr/bin
	cp assets/systemd-manager-pkexec debian/usr/bin/
	cp assets/systemd-manager.desktop debian/usr/share/applications/
	cp assets/org.freedesktop.policykit.systemd-manager.policy debian/usr/share/polkit-1/actions
	dpkg-deb --build debian systemd-manager.deb
	sudo dpkg -i systemd-manager.deb
