DESTDIR = /usr
version = $(shell awk 'NR == 3 {print substr($$3, 2, length($$3)-2)}' Cargo.toml)
policykit = org.freedesktop.policykit.systemd-manager.policy

all:
	cargo build --release

install:
	install -Dm 755 target/release/systemd-manager "$(DESTDIR)/bin/systemd-manager"
	install -Dm 755 assets/systemd-manager-pkexec "$(DESTDIR)/bin/systemd-manager-pkexec"
	install -Dm 644 assets/systemd-manager.desktop "$(DESTDIR)/share/applications/systemd-manager.desktop"
	install -Dm 644 assets/$(policykit) "$(DESTDIR)/share/polkit-1/actions/$(policykit)"
	install -Dm 644 README.md "$(DESTDIR)/share/doc/systemd-manager/README"
	install -Dm 644 LICENSE "$(DESTDIR)/share/licenses/systemd-manager/COPYING"

uninstall:
	rm $(DESTDIR)/bin/systemd-manager
	rm $(DESTDIR)/bin/systemd-manager-pkexec
	rm $(DESTDIR)/share/applications/systemd-manager.desktop
	rm $(DESTDIR)/share/polkit-1/actions/$(policykit)

tar:
	install -Dm 755 target/release/systemd-manager systemd-manager/bin/systemd-manager
	install -Dm 755 assets/systemd-manager-pkexec systemd-manager/bin/systemd-manager-pkexec
	install -Dm 644 assets/systemd-manager.desktop systemd-manager/share/applications/systemd-manager.desktop
	install -Dm 644 assets/$(policykit) systemd-manager/share/polkit-1/actions/$(policykit)
	tar cf - "systemd-manager" | xz -zf > systemd-manager_$(version)_$(shell uname -m).tar.xz

deb:
	# Install libgtk-3-dev if it is not installed
	dpkg -s libgtk-3-dev >/dev/null 2>&1 || sudo apt install libgtk-3-dev -y
	# Compile systemd-manager
	cargo build --release
	# Set the version in the Debian control file that is in the Cargo.toml.
	sed "2s/.*/Version: $(version)/g" -i debian/DEBIAN/control
	# Set the architecture in the Debian control file based on what `dpkg` reports.
	sed "7s/.*/Architecture: $(shell dpkg --print-architecture)/g" -i debian/DEBIAN/control
	# Install the files into the debian directory.
	install -Dsm 755 target/release/systemd-manager debian/usr/bin/systemd-manager
	install -Dm 755 assets/systemd-manager-pkexec debian/usr/bin/systemd-manager-pkexec
	install -Dm 644 assets/systemd-manager.desktop debian/usr/share/applications/systemd-manager.desktop
	install -Dm 644 assets/$(policykit) debian/usr/share/polkit-1/actions/$(policykit)
	install -Dm 644 README.md debian/usr/share/doc/systemd-manager/README
	install -Dm 644 LICENSE debian/usr/share/licenses/systemd-manager/COPYING
	# Generate a Debian package from the debian directory.
	fakeroot dpkg-deb --build debian systemd-manager_$(version)_$(shell dpkg --print-architecture).deb
	# Install the debian package
	sudo dpkg -i systemd-manager_$(version)_$(shell dpkg --print-architecture).deb
