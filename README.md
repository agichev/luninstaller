# luninstaller (GNOME Application Uninstaller)

A simple, lightweight, and modern graphical application built with **Rust + GTK3** that lets you search and uninstall any installed application visible in the GNOME application menu in just a few clicks.

![App Icon](resources/luninstaller.svg)

## Features
- 🔍 **Instant Search**: real-time filtering by application name, description, and package type.
- 📦 **Multi-format Support**:
  - **Flatpak** (`flatpak uninstall <id>`)
  - **Snap** (`pkexec snap remove <name>`)
  - **APT / Dpkg** system deb packages (`pkexec apt-get purge <pkg>`)
  - **Local & Custom Applications** (from `~/.local/share/applications` and standalone shortcuts)
- 🎨 **Modern GNOME Look**: HeaderBar, native GTK theme compliance (automatic dark/light mode), system icons, and clean package type badges.
- ⚡ **High Performance**: Instant startup without process spawning bottlenecks, asynchronous background uninstallation without freezing the UI or triggering "Application not responding" alerts.
- 🛡️ **Safe**: Modal confirmation dialog before removal. Runs as a regular user and only invokes standard Polkit (`pkexec`) for administrative tasks.

## Installation (.deb package)

To install the pre-built `.deb` package on your system:
```bash
sudo apt install ./luninstaller_0.1.0_amd64.deb
```
or via `dpkg`:
```bash
sudo dpkg -i luninstaller_0.1.0_amd64.deb
```

Once installed, find **"Application Uninstaller"** in your GNOME application menu or run `luninstaller` in the terminal.

## Building from Source

### Prerequisites
- Rust & Cargo (1.70+)
- GTK3 development headers: `libgtk-3-dev`
- `dpkg-deb` (for building the `.deb` package)

### Build and Run
```bash
# Run directly in development mode
cargo run

# Build optimized release binary
cargo build --release

# Build complete Debian package (.deb)
./build_deb.sh
```

## License
[GPL-3.0](LICENSE)
