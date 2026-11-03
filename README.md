# AIO Cooler Screen — v1.2.4-Fork

[**Türkçe**](TURKCE-README.md) | [Original project](https://github.com/Thnorty/aio-cooler-screen) | [Build and setup](FORK-BUILD-README.md) | [Sensor guide](SENSOR-GUIDE.md)

A Windows app for compatible Monster AIO cooler LCDs (USB HID `1D6B:0130`). This is an independent fork of **Thnorty/aio-cooler-screen**, maintained by SefaMeert. It is not affiliated with Monster.

## What it does

- 480 × 480 LCD backgrounds: images, animated GIFs, and videos (optional FFmpeg).
- Ready-made layouts and a custom editor with text, values, gauges, overlays, and progress indicators.
- Live CPU/GPU information and a background Windows service.
- **Built-in hardware monitoring** via a bundled .NET / LibreHardwareMonitor helper, including available clocks, voltages, fans/pump RPM, and temperatures — without an external LHM web server.
- Friendly sensor labels that preserve stable sensor identifiers in saved designs.
- Optional **external LibreHardwareMonitor HTTP (default port 8085)** and **HWiNFO shared-memory** sources for other devices.
- **English and Turkish** application UI, tray menu, and installer; additional fonts and Windows build workflow.

Hardware availability varies by system; unsupported sensors may be missing. Fan/pump RPM readings do not imply fan-speed control.

## Screenshots

### Background
![Background](docs/screenshots/en/background.png)

### Design
![Design](docs/screenshots/en/design.png)

### Sensors
![Sensors](docs/screenshots/en/sensors.png)

### Settings
![Settings](docs/screenshots/en/settings.png)

## Install

Use the full **Windows x64 setup** on Windows 10/11. It installs the background service and internal sensor helper. The standalone EXE is not a replacement for the complete setup. Close the original Monster screen software to avoid USB device conflicts.

**Unsigned build:** Smart App Control or enterprise application-control policies may block the installer or sensor helper. Do **not** disable Windows security to make it run. A test build passing on one computer does not guarantee compatibility elsewhere.

## More information

[Detailed hardware sensor setup](SENSOR-GUIDE.md) · [Building and packaging](FORK-BUILD-README.md) · [Fonts](FONT-README.md) · [Third-party licences](THIRD-PARTY-NOTICES.md)

The original author's MIT licence is retained in [LICENSE](LICENSE). See third-party notices for the bundled LibreHardwareMonitor library and other components.
