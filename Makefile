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
