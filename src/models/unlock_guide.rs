// Bootloader unlock guide
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Generates a manufacturer-aware list of unlocking steps. Most fastboot-based
// devices share a common flow (`fastboot flashing unlock`), while several
// vendors require their own portals or tooling (Sony/Motorola unlock codes,
// Xiaomi's Mi Unlock, Samsung's Download mode).

use crate::models::device::Device;
use crate::models::unlocking_step::{StepType, UnlockingStep};

/// Build the ordered list of unlocking steps for a device.
pub fn unlock_steps(device: &Device) -> Vec<UnlockingStep> {
    let maker = normalize_maker(&device.maker);

    let mut steps = common_preamble();

    match maker.as_str() {
        "samsung" => steps.extend(samsung_steps()),
        "xiaomi" | "redmi" | "poco" => steps.extend(xiaomi_steps()),
        "sony" => steps.extend(sony_steps()),
        "motorola" | "lenovo" => steps.extend(motorola_steps()),
        _ => steps.extend(generic_fastboot_steps()),
    }

    // Renumber so `order` is always a clean 1-based sequence regardless of
    // how the manufacturer-specific blocks were composed.
    for (idx, step) in steps.iter_mut().enumerate() {
        step.order = (idx + 1) as u8;
    }

    steps
}

/// Lowercase the maker and strip non-alphanumeric characters so values like
/// "F(x)tec" or "Redmi (Xiaomi)" normalize predictably.
fn normalize_maker(maker: &str) -> String {
    maker
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn manual(title: &str, description: &str) -> UnlockingStep {
    UnlockingStep {
        order: 0,
        title: title.to_string(),
        description: description.to_string(),
        step_type: StepType::Manual,
        command: None,
        duration_secs: None,
        optional: false,
        warning: None,
        link: None,
        link_label: None,
    }
}

fn automated(title: &str, description: &str, command: &str, duration_secs: u32) -> UnlockingStep {
    UnlockingStep {
        order: 0,
        title: title.to_string(),
        description: description.to_string(),
        step_type: StepType::Automated,
        command: Some(command.to_string()),
        duration_secs: Some(duration_secs),
        optional: false,
        warning: None,
        link: None,
        link_label: None,
    }
}

/// Steps every device shares: enabling developer options and backing up data.
fn common_preamble() -> Vec<UnlockingStep> {
    let mut enable_dev = manual(
        "Enable Developer Options",
        "On your phone, open Settings → About phone and tap \"Build number\" seven times until you see \"You are now a developer\".",
    );
    enable_dev.duration_secs = None;

    let mut enable_oem = manual(
        "Enable OEM Unlocking & USB Debugging",
        "In Settings → System → Developer options, turn on both \"OEM unlocking\" and \"USB debugging\". Accept the prompt on the phone when you connect it.",
    );
    enable_oem.warning = Some(
        "If \"OEM unlocking\" is greyed out, connect to the internet and wait a few minutes, or remove any active SIM lock.".to_string(),
    );

    let mut backup = manual(
        "Back Up Your Data",
        "Unlocking the bootloader erases everything on the device. Copy any photos, files, and accounts you want to keep before continuing.",
    );
    backup.warning = Some("This step wipes all user data. There is no way to recover it afterwards.".to_string());

    vec![enable_dev, enable_oem, backup]
}

/// The standard fastboot unlock used by Google, Fairphone, OnePlus, and most others.
fn generic_fastboot_steps() -> Vec<UnlockingStep> {
    let mut reboot = automated(
        "Reboot to Bootloader",
        "Reboots the phone into fastboot/bootloader mode. The screen will show the fastboot/bootloader menu when ready.",
        "adb reboot bootloader",
        12,
    );
    reboot.duration_secs = Some(12);

    let mut unlock = automated(
        "Unlock the Bootloader",
        "Runs the unlock command. On the phone, use the volume keys to highlight \"Unlock the bootloader\" and press the power button to confirm. The device will then factory-reset.",
        "fastboot flashing unlock",
        30,
    );
    unlock.warning =
        Some("You must confirm on the device itself. This wipes all data and reboots.".to_string());

    vec![reboot, unlock]
}

/// Sony devices need a per-device unlock code from Sony's developer portal.
fn sony_steps() -> Vec<UnlockingStep> {
    let mut get_code = manual(
        "Request Your Unlock Code",
        "Sony issues a unique unlock code per device. Find your IMEI (dial *#06#), then register on Sony's Open Devices portal to receive your code.",
    );
    get_code.link = Some(
        "https://developer.sony.com/open-source/aosp-on-xperia-open-devices/get-started/unlock-bootloader/".to_string(),
    );
    get_code.link_label = Some("Open Sony Portal".to_string());

    let reboot = automated(
        "Reboot to Bootloader",
        "Reboots the phone into fastboot/bootloader mode so the unlock code can be applied.",
        "adb reboot bootloader",
        12,
    );

    let mut unlock = manual(
        "Unlock With Your Code",
        "In a terminal, run `fastboot oem unlock 0xKEY`, replacing KEY with the code Sony gave you. This wipes the device. Then return here and continue.",
    );
    unlock.warning = Some("This wipes all data. Use the exact code Sony provided for your device.".to_string());

    vec![get_code, reboot, unlock]
}

/// Motorola/Lenovo devices need a code derived from `fastboot oem get_unlock_data`.
fn motorola_steps() -> Vec<UnlockingStep> {
    let reboot = automated(
        "Reboot to Bootloader",
        "Reboots the phone into fastboot/bootloader mode.",
        "adb reboot bootloader",
        12,
    );

    let mut get_data = automated(
        "Read Your Unlock Data",
        "Reads the device-specific unlock data. Copy the full string from the terminal output — you'll paste it into Motorola's website in the next step.",
        "fastboot oem get_unlock_data",
        8,
    );
    get_data.duration_secs = Some(8);

    let mut request = manual(
        "Request Your Unlock Code",
        "Paste your unlock data into Motorola's bootloader unlock page to receive an unlock code by email.",
    );
    request.link =
        Some("https://en-us.support.motorola.com/app/standalone/bootloader/unlock-your-device-b".to_string());
    request.link_label = Some("Open Motorola Portal".to_string());

    let mut unlock = manual(
        "Unlock With Your Code",
        "In a terminal, run `fastboot oem unlock CODE`, replacing CODE with the code Motorola emailed you. This wipes the device, then return here and continue.",
    );
    unlock.warning = Some("This wipes all data. Use the exact code from Motorola's email.".to_string());

    vec![reboot, get_data, request, unlock]
}

/// Xiaomi/Redmi/POCO require the Mi Unlock tool, a bound account, and a waiting period.
fn xiaomi_steps() -> Vec<UnlockingStep> {
    let mut bind = manual(
        "Add & Bind Your Mi Account",
        "In Settings → Developer options → \"Mi Unlock status\", bind your Mi account to this device. Make sure a SIM/mobile data is active during binding.",
    );
    bind.warning = Some(
        "Xiaomi enforces a waiting period (often 7+ days) between binding and unlocking. Bind as early as possible.".to_string(),
    );

    let mut mi_unlock = manual(
        "Use the Mi Unlock Tool",
        "Download Xiaomi's official Mi Unlock tool (Windows) and sign in with the same Mi account. Put the phone in fastboot mode, connect it, and click Unlock. Xiaomi does not allow unlocking with plain fastboot.",
    );
    mi_unlock.link = Some("https://en.miui.com/unlock/".to_string());
    mi_unlock.link_label = Some("Open Mi Unlock".to_string());
    mi_unlock.warning = Some("This wipes all data once the waiting period has elapsed.".to_string());

    vec![bind, mi_unlock]
}

/// Samsung devices don't use fastboot; they unlock via Download mode.
fn samsung_steps() -> Vec<UnlockingStep> {
    let power_off = manual(
        "Power Off the Device",
        "Fully power off the phone and unplug the USB cable. Samsung devices unlock through Download mode rather than fastboot.",
    );

    let download_mode = manual(
        "Boot Into Download Mode",
        "Hold both Volume Up and Volume Down together, then plug in the USB cable while still holding. Keep holding until the warning screen appears.",
    );

    let mut unlock = manual(
        "Unlock the Bootloader",
        "Press and hold Volume Up when prompted to enter the unlock screen, then press Volume Up again to confirm. The device factory-resets and reboots.",
    );
    unlock.warning = Some(
        "This wipes all data. After it reboots, complete setup and re-enable OEM unlocking before installing.".to_string(),
    );

    vec![power_off, download_mode, unlock]
}
