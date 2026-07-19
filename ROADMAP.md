# Sidestep Roadmap

## UBports full-DSL flash engine — DONE

Implemented: a `bootstrap` step DSL (`models/installer.rs` `BootstrapStep` +
`UbportsDownload`), an interpreter in `UbportsInstaller` (download/unpack →
ordered fastboot/Heimdall bootstrap → existing system-image recovery push), the
missing fastboot verbs, and 36 device configs generated from the authoritative
`ubports/installer-configs` (Ubuntu Touch section only). Samsung Exynos
(herolte/hero2lte) routes through download mode + Heimdall. A deserialization
test covers the simple/logical-partition/unpack/heimdall tiers. Legacy sargo
`firmware` path kept as a fallback. Not hardware-tested.

Original analysis (kept for reference):

**Goal:** wire ~35 more devices for Ubuntu Touch by executing the per-device
flash sequences from `ubports/installer-configs` (v2/devices/*.yml) — the
authoritative source (URLs + SHA256 sums published for most devices).

**Why it's big:** the configs are a rich flash DSL, not a firmware list. Per
device it can include (verbs seen across our 35): `fastboot:flash/format/erase`,
`set_active`, `delete_logical_partition`, `resize_logical_partition`,
`wipe_super`, `reboot_{recovery,bootloader,fastboot}`, `wait`, `assert_var`,
`continue`, `core:download/unpack/user_action`, `systemimage:channels/install`,
and `heimdall:flash` (Samsung). Examples: FP4 deletes 7 logical partitions +
resizes; beryllium flashes 30+ unpacked firmware partitions per A/B slot; surya
does `wipe_super`; herolte is Heimdall. Install *models* also differ (flash
rootfs directly vs push system-image via recovery).

**Phased plan:**
1. **DONE** — add missing fastboot verbs to `hardware/fastboot.rs`:
   `set_active`, `delete_logical_partition`, `resize_logical_partition`,
   `wipe_super`, `reboot_fastboot`.
2. Add a `bootstrap` step model to the ubports InstallerConfig (ordered typed
   ops referencing downloaded `firmware` files) + an interpreter in
   `UbportsInstaller` that runs them, replacing the hardcoded flash/format.
3. Support `core:unpack` (flash from `unpacked/...` firmware archives) and
   optional checksums (some devices publish none — download+hash or accept).
4. Handle per-device install models (recovery-push vs direct rootfs flash).
5. Wire Heimdall (`heimdall:flash`) for herolte/hero2lte via the existing
   `HeimdallInstaller` wrapper.
6. Generate + **verify** per-device bootstrap configs from installer-configs
   (a bulk auto-translate was tried and reverted — it silently dropped every
   no-checksum/complex device; needs per-tier verification, brick risk).

**Devices pending (in installer-configs):** pro1x, FP2/3/4/5, axolotl, x00td,
herolte, hero2lte, begonia, lavender, citrus, lancelot, merlin, jasmine_sprout,
sagit, miatoll, beryllium, violet, surya, enchilada, fajita, oneplus3, billie,
billie2, cheeseburger, dumpling, denniz, bacon, suzu, yggdrasil, yggdrasilx,
mimameid, vidofnir, algiz, mimir. Not in installer-configs: biloba, bonito.



## pmbootstrap backend — build-and-flash for source-only postmarketOS devices

**Goal:** make Sidestep a one-stop shop for Linux/Android flashing by supporting
postmarketOS devices that have **no pre-built image** (source-only in pmaports),
by driving a host-installed `pmbootstrap`.

**Why host-driven (not bundled):** pmbootstrap needs chroots, loop-mounted
images and user namespaces, which the flatpak sandbox blocks by design. It is
meant to run on the host. So Sidestep should orchestrate a host pmbootstrap
rather than run it inside the sandbox.

**Design (mirrors the Droidian `flash_all.sh` pattern, but aimed at the host):**
- Add `--talk-name=org.freedesktop.Flatpak` to the manifest → run host commands
  via `flatpak-spawn --host`.
- Detect host pmbootstrap; if absent, show an instruction step
  (`pipx install pmbootstrap`) instead of failing.
- Drive non-interactively: `pmbootstrap init` (device + UI preset) →
  `pmbootstrap install` → `pmbootstrap flasher flash_rootfs` / `flash_kernel`,
  streaming output to the terminal overlay and checking exit status.
- Gate behind `flash_method: "pmbootstrap"` in the distro config so it activates
  only for devices marked that way.

**Tradeoffs to weigh when building it:**
- `org.freedesktop.Flatpak` talk permission is broad (host command execution) —
  a real sandbox-security loosening.
- Depends on the user's host having pmbootstrap (detect + guide; can't bundle).
- It's a *build*, not a download: 10–60 min + several GB — needs different UX
  expectations than the current download-and-flash flow.

**Already in place:** the source-only devices still carry their postmarketOS
`distros.yml` entries, so enabling them later is mostly setting
`flash_method: pmbootstrap` per device — no re-research needed.

**Devices waiting on this backend (no `bpo` image as of 2026-07):**
fairphone/fp2, fairphone/fp3, asus/x00td, samsung/herolte (Exynos → also needs
Heimdall), xiaomi/begonia, xiaomi/lavender, xiaomi/lancelot,
xiaomi/jasmine_sprout, xiaomi/sagit, oneplus/bacon, oneplus/oneplus3,
oneplus/cheeseburger, oneplus/dumpling.
