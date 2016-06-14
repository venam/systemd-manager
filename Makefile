DESTDIR = /usr
version = $(shell awk 'NR == 3 {print substr($$3, 2, length($$3)-2)}' Cargo.toml)
policykit_src = "assets/org.freedesktop.policykit.systemd-manager.policy"

all:
	cargo build --release

install:
	install -Dm 755 target/release/systemd-manager "$(DESTDIR)/bin/systemd-manager"
	install -Dm 755 assets/systemd-manager-pkexec "$(DESTDIR)/bin/systemd-manager-pkexec"
	install -Dm 644 assets/systemd-manager.desktop "$(DESTDIR)/share/applications/systemd-manager.desktop"
	install -Dm 644 $(policykit_src) "$(DESTDIR)/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy"

uninstall:
	rm $(DESTDIR)/bin/systemd-manager
	rm $(DESTDIR)/bin/systemd-manager-pkexec
	rm $(DESTDIR)/share/applications/systemd-manager.desktop
	rm $(DESTDIR)/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy

tar:
	install -Dm 755 target/release/systemd-manager systemd-manager/bin/systemd-manager
	install -Dm 755 assets/systemd-manager-pkexec systemd-manager/bin/systemd-manager-pkexec
	install -Dm 644 assets/systemd-manager.desktop systemd-manager/share/applications/systemd-manager.desktop
	install -Dm 644 $(policykit_src) systemd-manager/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy
	tar cf - "systemd-manager" | xz -zf > systemd-manager_$(version).tar.xz

deb:
	dpkg -s libgtk-3-dev >/dev/null 2>&1 || sudo apt install libgtk-3-dev -y
	cargo build --release
	sed "2s/.*/Version: $(version)/g" -i debian/DEBIAN/control
	sed "7s/.*/Architecture: $(shell dpkg --print-architecture)/g" -i debian/DEBIAN/control
	install -Dsm 755 target/release/systemd-manager debian/usr/binsystemd-manager
	install -Dm 755 assets/systemd-manager-pkexec debian/usr/bin/systemd-manager-pkexec
	install -Dm 644 assets/systemd-manager.desktop debian/usr/share/applications/systemd-manager.desktop
	install -Dm 644 $(policykit_src) debian/usr/share/polkit-1/actions/org.freedesktop.policykit.systemd-manager.policy
	fakeroot dpkg-deb --build debian systemd-manager_$(version)_$(shell dpkg --print-architecture).deb
	sudo dpkg -i systemd-manager_$(version)_$(shell dpkg --print-architecture).deb
