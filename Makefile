all:
	cargo build --release

install:
	cp target/release/systemd-manager /usr/bin/
	cp assets/systemd-manager-pkexec /usr/bin/
	cp assets/systemd-manager.desktop /usr/share/applications/
	cp assets/org.freedesktop.policykit.systemd-manager.policy /usr/share/polkit-1/actions/
	
uninstall:
	rm /usr/bin/systemd-manager
	rm /usr/bin/systemd-manager-pkexec
	rm /usr/share/applications/systemd-manager.desktop
	rm /usr/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy

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
