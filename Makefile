prefix = /usr

all:
	cargo build --release

install:
	cp target/release/systemd-manager $(prefix)/bin/
	cp assets/systemd-manager-pkexec $(prefix)/bin/
	cp assets/systemd-manager.desktop $(prefix)/share/applications/
	cp assets/org.freedesktop.policykit.systemd-manager.policy $(prefix)/share/polkit-1/actions/
	
uninstall:
	rm $(prefix)/bin/systemd-manager
	rm $(prefix)/bin/systemd-manager-pkexec
	rm $(prefix)/share/applications/systemd-manager.desktop
	rm $(prefix)/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy

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
