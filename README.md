# AIO Cooler Screen

A fork of [Thnorty/aio-cooler-screen](https://github.com/Thnorty/aio-cooler-screen), with additional improvements maintained by **SefaMeert**. Designed for compatible Monster AIO cooler LCD screens (USB HID `1D6B:0130`).

[**Türkçe README**](TURKCE-README.md) · [Original project](https://github.com/Thnorty/aio-cooler-screen) · [Build instructions](FORK-BUILD-README.md)

> **Project status:** Actively tested fork. The Windows build and installer have been built and launched on a test computer. Full compatibility with all hardware, fonts and LCD configurations has not been verified. **Release binaries are currently unsigned.** Windows Smart App Control may block them; do not disable system security protections just to install this application.

![Screen designs](docs/designs.png)

## Improvements in this fork

- **Extended hardware sensors:** Additional LibreHardwareMonitor readings, including fan/pump RPM, voltage, power, current and clocks where available.
- **Clearer sensor selection:** Sensor type/category labels while aiming to preserve identifiers used by existing designs.
- **English and Turkish UI:** English by default, with a saved language preference. Hardware sensor names may remain in their original language.
- **More fonts:** Google Sans, Google Sans Flex, Inter, Rajdhani, Orbitron, JetBrains Mono, Roboto Condensed and Oswald alongside existing choices. See [FONT-README.md](FONT-README.md).
- **Windows build workflow:** GitHub Actions creates a full Windows installer and a standalone executable for advanced use.

## Features inherited from the original project

- Built-in and custom image, video and GIF backgrounds for the 480×480 LCD.
- Preset layouts, a custom editor, overlays, gauges, text and progress bars.
- Temperature/load readings and integrations with HWiNFO and LibreHardwareMonitor.
- A Windows background service for updating the LCD.

## Installation

Requires 64-bit Windows 10/11. For a new installation use the **full installer**; the standalone `aio-screen.exe` does not install the background service or supporting components. Close the original Monster LCD app before using this one to avoid device conflicts.

**Windows security:** Unsigned installers may be blocked by Smart App Control. Source visibility and checksums help with auditing but are not proof of binary safety. Verify the release source and keep Windows security enabled.

[Build and artifact instructions](FORK-BUILD-README.md)

## Hardware sensors

Run LibreHardwareMonitor as administrator, enable **Options → Remote Web Server → Run**, and configure port `8085` in AIO Cooler Screen. Sensors are available only if your hardware and monitoring software expose them. Displaying fan RPM does **not** mean fan-speed control.

## Credits and licences

This repository is a fork of **Thnorty's original AIO Cooler Screen**. Original copyright and MIT licence terms are preserved in [LICENSE](LICENSE). Additional components and fonts have their own terms: see [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) and [FONT-README.md](FONT-README.md).

This is an independent fork, not an official release from the original author or Monster Notebook. Not affiliated with or endorsed by Monster Notebook or Google. Their trademarks belong to their respective owners.
