CARGO ?= cargo
INSTALL ?= install
SYSTEMCTL ?= systemctl

BINARY := zsh-history-backup
PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
CONFIG_DIR ?= $(HOME)/.config/$(BINARY)
CONFIG_FILE ?= $(CONFIG_DIR)/config.toml
SYSTEMD_USER_DIR ?= $(HOME)/.config/systemd/user
AUTO_BACKUP_DIR ?= $(HOME)/zsh/backup/auto
MANUAL_BACKUP_DIR ?= $(HOME)/zsh/backup/manual

.PHONY: all build install install-binary install-config install-dirs install-systemd enable-timer disable-timer status help

all: build

build:
	$(CARGO) build --release --locked

install: install-binary install-config install-systemd
	$(SYSTEMCTL) --user enable --now $(BINARY).timer
	@echo "Installed and enabled $(BINARY).timer."

install-binary: build
	$(INSTALL) -Dm755 target/release/$(BINARY) "$(BINDIR)/$(BINARY)"

install-config:
	$(INSTALL) -d -m700 "$(CONFIG_DIR)"
	@if [ -e "$(CONFIG_FILE)" ]; then \
		echo "Keeping existing $(CONFIG_FILE)"; \
	else \
		$(INSTALL) -m600 config.toml "$(CONFIG_FILE)"; \
	fi

install-dirs:
	$(INSTALL) -d -m700 "$(AUTO_BACKUP_DIR)"
	$(INSTALL) -d -m700 "$(MANUAL_BACKUP_DIR)"

install-systemd: install-dirs
	$(INSTALL) -d "$(SYSTEMD_USER_DIR)"
	$(INSTALL) -m644 systemd/$(BINARY).service "$(SYSTEMD_USER_DIR)/$(BINARY).service"
	$(INSTALL) -m644 systemd/$(BINARY).timer "$(SYSTEMD_USER_DIR)/$(BINARY).timer"
	$(SYSTEMCTL) --user daemon-reload

enable-timer: install

disable-timer:
	$(SYSTEMCTL) --user disable --now $(BINARY).timer

status:
	$(SYSTEMCTL) --user status $(BINARY).timer --no-pager

help:
	@echo "make build          Build the release binary"
	@echo "make install        Install binary, config, directories, and systemd units"
	@echo "make enable-timer   Install and enable the midnight timer"
	@echo "make disable-timer  Disable and stop the timer"
	@echo "make status         Show timer status"
