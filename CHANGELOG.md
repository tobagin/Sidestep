# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-02-16

### Added

- **22 New UBports Devices**: Added all Ubuntu Touch Noble (24.04) and Focal (20.04) devices with full cross-distro coverage:
  - **ASUS**: ZenFone Max Pro M1 (x00td) — new manufacturer
  - **Fairphone**: Fairphone 2 (fp2), Fairphone 3/3+ (fp3)
  - **Google**: Pixel 3a XL (bonito)
  - **OnePlus**: OnePlus One (bacon), OnePlus 5 (cheeseburger), OnePlus 5T (dumpling), Nord N10 5G (billie), Nord N100 (billie2), Nord 2 5G (denniz)
  - **Samsung**: Galaxy S7 Exynos (herolte), Galaxy S7 Edge Exynos (hero2lte) — new manufacturer
  - **Sony**: Xperia X (suzu)
  - **Xiaomi**: Mi 6 (sagit), Mi A2 (jasmine_sprout), POCO M3 (citrus), POCO X3 NFC (surya), Redmi 9 (lancelot), Redmi Note 7 (lavender), Redmi Note 8 Pro (begonia), Redmi Note 8 2021 (biloba), Redmi Note 9 (merlin)
- **Cross-Distro Coverage**: Each device includes applicable distros from postmarketOS, LineageOS, and /e/OS in addition to Ubuntu Touch.
- **Previous Devices**: OnePlus 6T (fajita), Pocophone F1 (beryllium), Fairphone 4 (FP4), Fairphone 5 (FP5), SHIFT6mq (axolotl), Moto Z (griffin), OnePlus 3/3T (oneplus3), Redmi Note 7 Pro (violet), Xperia 5 II (pdx206).
- **Droidian Community Devices**: Added griffin, oneplus3, violet, pdx206 with Droidian community support.
- **Mobian Support**: Added Mobian as a distro option for all Qualcomm-based devices (sdm845, sm6350, sc7280 chipsets).
- **/e/OS Support**: Added /e/OS community builds for multiple devices with Android 13/14/15 channels.
- **OnePlus 6 Data**: Created missing data directory for enchilada with full distro coverage.
- **New Manufacturers**: ASUS, Samsung, Fairphone, and SHIFT added as supported manufacturers.

### Changed

- **Version Source of Truth**: User-agent strings in flashing modules now use `crate::config::VERSION` instead of hardcoded version strings.
- **Device Count**: 15 → 47 supported devices.

## [0.2.0] - 2026-02-16

### Added

- **Device Detection**: Automatic USB device identification via ADB and Fastboot polling.
- **Device Browser**: Browse all supported devices, specs, and available distributions without connecting a phone.
- **Device Variants**: Devices with multiple marketing names (e.g. miatoll: POCO M2 Pro, Redmi Note 9S, Redmi Note 9 Pro) appear as separate rows in the browser, all linking to the same device.
- **Multi-Distro Support**: Install Droidian, Mobian, Ubuntu Touch, postmarketOS, and other distributions.
- **Full Install Pipeline**: Download, decompress (XZ/GZIP), verify (SHA256), and flash images via Fastboot.
- **Ubuntu Touch Installer**: Dedicated installer implementation for Ubuntu Touch with channel and interface selection.
- **Android Firmware Flashing**: Flash stock Android firmware as a pre-requisite when required by a distribution.
- **Bootloader Unlock Wizard**: Device-specific step-by-step unlocking guidance (Google, OnePlus, Xiaomi, Sony, Motorola, F(x)tec, Volla, Lenovo).
- **Bootloader Lock Detection**: Automatic check of bootloader lock status via Fastboot.
- **Pre-requisite Checks**: Firmware version validation and battery level checks before installation.
- **Interface Selection**: Choose between Phosh, Plasma Mobile, and other desktop shells where supported.
- **Channel Selection**: Pick stable, development, or edge release channels per distribution.
- **Real-Time Progress**: Live progress bars for download, extraction, and flashing stages.
- **Device Info Page**: Read-only device specs page (SoC, display, connectivity, cameras) loaded from YAML.
- **Distro Detail Page**: Per-distro detail view with channels, interfaces, and hardware compatibility info.
- **15 Supported Devices**: Google Pixel 3a, OnePlus 6, Xiaomi miatoll family, Microsoft Surface Duo, Motorola Edge 30, F(x)tec Pro1/Pro1 X, Sony Xperia 5, Volla Phone/X/22/X23/Quintus/Tablet, Lenovo ThinkPhone.
- **Flatpak Packaging**: Development and production Flatpak builds with sandboxed USB access.
