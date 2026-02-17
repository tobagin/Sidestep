// Device database - loads device data from YAML files
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::{Device, Distro, UnlockingStep};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::models::PartitionImage;

/// In-memory database of supported devices
pub struct DeviceDatabase {
    devices: HashMap<String, Device>,
    unlocking_steps: HashMap<String, Vec<UnlockingStep>>,
    distros: HashMap<String, Vec<Distro>>,
    data_dir: PathBuf,
}

impl DeviceDatabase {
    pub fn new() -> Self {
        // Default data directory (bundled with app)
        let data_dir = std::env::var("SIDESTEP_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("devices"));

        let mut db = Self {
            devices: HashMap::new(),
            unlocking_steps: HashMap::new(),
            distros: HashMap::new(),
            data_dir,
        };

        // Load hardcoded devices for MVP
        db.load_hardcoded_devices();

        db
    }

    /// Load hardcoded device data (MVP approach)
    fn load_hardcoded_devices(&mut self) {
        // Pixel 3a (sargo)
        self.devices.insert("sargo".to_string(), Device {
            codename: "sargo".to_string(),
            name: "Google Pixel 3a".to_string(),
            maker: "Google".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Pixel 3a".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus 6 (enchilada)
        self.devices.insert("enchilada".to_string(), Device {
            codename: "enchilada".to_string(),
            name: "OnePlus 6".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["OnePlus6".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Motorola Edge 30 (dubai)
        self.devices.insert("dubai".to_string(), Device {
            codename: "dubai".to_string(),
            name: "Motorola Edge 30".to_string(),
            maker: "Motorola".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["moto edge 30".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Surface Duo (zeta)
        self.devices.insert("zeta".to_string(), Device {
            codename: "zeta".to_string(),
            name: "Microsoft Surface Duo".to_string(),
            maker: "Microsoft".to_string(),
            experimental: true,
            battery_min: 60,
            warnings: vec![
                "Experimental device - proceed with caution".to_string(),
                "Dual screen functionality may be limited".to_string(),
            ],
            aliases: vec!["surfaceduo".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Xiaomi POCO M2 Pro / Redmi Note 9 Pro (miatoll)
        self.devices.insert("miatoll".to_string(), Device {
            codename: "miatoll".to_string(),
            name: "POCO M2 Pro".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![
                "Redmi Note 9S".to_string(),
                "Redmi Note 9 Pro".to_string(),
                "Redmi Note 9 Pro Max".to_string(),
                "Redmi Note 10 Lite".to_string(),
            ],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // F(x)tec Pro1 (pro1)
        self.devices.insert("pro1".to_string(), Device {
            codename: "pro1".to_string(),
            name: "F(x)tec Pro1".to_string(),
            maker: "F(x)tec".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Pro1".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // F(x)tec Pro1 X (pro1x)
        self.devices.insert("pro1x".to_string(), Device {
            codename: "pro1x".to_string(),
            name: "F(x)tec Pro1 X".to_string(),
            maker: "F(x)tec".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Pro1X".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Sony Xperia 5 (bahamut)
        self.devices.insert("bahamut".to_string(), Device {
            codename: "bahamut".to_string(),
            name: "Sony Xperia 5".to_string(),
            maker: "Sony".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "Sony DRM keys will be permanently lost after unlocking".to_string(),
            ],
            aliases: vec!["Xperia 5".to_string(), "J9210".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Volla Phone (yggdrasil)
        self.devices.insert("yggdrasil".to_string(), Device {
            codename: "yggdrasil".to_string(),
            name: "Volla Phone".to_string(),
            maker: "Volla".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["GS290".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Volla Phone X (yggdrasilx)
        self.devices.insert("yggdrasilx".to_string(), Device {
            codename: "yggdrasilx".to_string(),
            name: "Volla Phone X".to_string(),
            maker: "Volla".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Volla Phone 22 (mimameid)
        self.devices.insert("mimameid".to_string(), Device {
            codename: "mimameid".to_string(),
            name: "Volla Phone 22".to_string(),
            maker: "Volla".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Volla Phone X23 (vidofnir)
        self.devices.insert("vidofnir".to_string(), Device {
            codename: "vidofnir".to_string(),
            name: "Volla Phone X23".to_string(),
            maker: "Volla".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Volla Phone Quintus (algiz)
        self.devices.insert("algiz".to_string(), Device {
            codename: "algiz".to_string(),
            name: "Volla Phone Quintus".to_string(),
            maker: "Volla".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Volla 5".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Lenovo ThinkPhone by Motorola (bronco)
        self.devices.insert("bronco".to_string(), Device {
            codename: "bronco".to_string(),
            name: "Lenovo ThinkPhone".to_string(),
            maker: "Lenovo".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["ThinkPhone".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Volla Tablet (mimir)
        self.devices.insert("mimir".to_string(), Device {
            codename: "mimir".to_string(),
            name: "Volla Tablet".to_string(),
            maker: "Volla".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus 6T (fajita)
        self.devices.insert("fajita".to_string(), Device {
            codename: "fajita".to_string(),
            name: "OnePlus 6T".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["OnePlus6T".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Xiaomi Pocophone F1 (beryllium)
        self.devices.insert("beryllium".to_string(), Device {
            codename: "beryllium".to_string(),
            name: "Pocophone F1".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["POCO F1".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Fairphone 4 (FP4)
        self.devices.insert("FP4".to_string(), Device {
            codename: "FP4".to_string(),
            name: "Fairphone 4".to_string(),
            maker: "Fairphone".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["fp4".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Fairphone 5 (FP5)
        self.devices.insert("FP5".to_string(), Device {
            codename: "FP5".to_string(),
            name: "Fairphone 5".to_string(),
            maker: "Fairphone".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["fp5".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // SHIFT6mq (axolotl)
        self.devices.insert("axolotl".to_string(), Device {
            codename: "axolotl".to_string(),
            name: "SHIFT6mq".to_string(),
            maker: "SHIFT".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["SHIFT 6mq".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Motorola Moto Z (griffin)
        self.devices.insert("griffin".to_string(), Device {
            codename: "griffin".to_string(),
            name: "Motorola Moto Z".to_string(),
            maker: "Motorola".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "Verizon Droid Edition may have bootloader unlock restrictions".to_string(),
            ],
            aliases: vec!["moto z".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus 3 (oneplus3)
        self.devices.insert("oneplus3".to_string(), Device {
            codename: "oneplus3".to_string(),
            name: "OnePlus 3".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["OnePlus3".to_string()],
            variants: vec![
                "OnePlus 3T".to_string(),
            ],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Xiaomi Redmi Note 7 Pro (violet)
        self.devices.insert("violet".to_string(), Device {
            codename: "violet".to_string(),
            name: "Redmi Note 7 Pro".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Redmi Note 7 Pro".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Sony Xperia 5 II (pdx206)
        self.devices.insert("pdx206".to_string(), Device {
            codename: "pdx206".to_string(),
            name: "Sony Xperia 5 II".to_string(),
            maker: "Sony".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "Sony DRM keys will be permanently lost after unlocking".to_string(),
            ],
            aliases: vec!["Xperia 5 II".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Fairphone 3/3+ (fp3)
        self.devices.insert("fp3".to_string(), Device {
            codename: "fp3".to_string(),
            name: "Fairphone 3/3+".to_string(),
            maker: "Fairphone".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["FP3".to_string(), "Fairphone 3".to_string()],
            variants: vec![
                "Fairphone 3+".to_string(),
            ],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Fairphone 2 (fp2)
        self.devices.insert("fp2".to_string(), Device {
            codename: "fp2".to_string(),
            name: "Fairphone 2".to_string(),
            maker: "Fairphone".to_string(),
            experimental: true,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "Older device with limited Linux distribution support".to_string(),
            ],
            aliases: vec!["FP2".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Google Pixel 3a XL (bonito)
        self.devices.insert("bonito".to_string(), Device {
            codename: "bonito".to_string(),
            name: "Google Pixel 3a XL".to_string(),
            maker: "Google".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Pixel 3a XL".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus Nord N10 5G (billie)
        self.devices.insert("billie".to_string(), Device {
            codename: "billie".to_string(),
            name: "OnePlus Nord N10 5G".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["Nord N10".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus Nord N100 (billie2)
        self.devices.insert("billie2".to_string(), Device {
            codename: "billie2".to_string(),
            name: "OnePlus Nord N100".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["Nord N100".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus 5 (cheeseburger)
        self.devices.insert("cheeseburger".to_string(), Device {
            codename: "cheeseburger".to_string(),
            name: "OnePlus 5".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["OnePlus5".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus 5T (dumpling)
        self.devices.insert("dumpling".to_string(), Device {
            codename: "dumpling".to_string(),
            name: "OnePlus 5T".to_string(),
            maker: "OnePlus".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["OnePlus5T".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus Nord 2 5G (denniz)
        self.devices.insert("denniz".to_string(), Device {
            codename: "denniz".to_string(),
            name: "OnePlus Nord 2 5G".to_string(),
            maker: "OnePlus".to_string(),
            experimental: true,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "OxygenOS backup recommended before proceeding".to_string(),
            ],
            aliases: vec!["Nord 2".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // OnePlus One (bacon)
        self.devices.insert("bacon".to_string(), Device {
            codename: "bacon".to_string(),
            name: "OnePlus One".to_string(),
            maker: "OnePlus".to_string(),
            experimental: true,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "Older device with limited Linux distribution support".to_string(),
            ],
            aliases: vec!["OnePlus1".to_string(), "A0001".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // POCO X3 NFC (surya)
        self.devices.insert("surya".to_string(), Device {
            codename: "surya".to_string(),
            name: "POCO X3 NFC".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["POCO X3".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Redmi Note 8 Pro (begonia)
        self.devices.insert("begonia".to_string(), Device {
            codename: "begonia".to_string(),
            name: "Redmi Note 8 Pro".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Redmi Note 7 (lavender)
        self.devices.insert("lavender".to_string(), Device {
            codename: "lavender".to_string(),
            name: "Redmi Note 7".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // POCO M3 (citrus)
        self.devices.insert("citrus".to_string(), Device {
            codename: "citrus".to_string(),
            name: "POCO M3".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Redmi 9 / 9 Prime (lancelot)
        self.devices.insert("lancelot".to_string(), Device {
            codename: "lancelot".to_string(),
            name: "Redmi 9".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![
                "Redmi 9 Prime".to_string(),
            ],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Redmi Note 9 (merlin)
        self.devices.insert("merlin".to_string(), Device {
            codename: "merlin".to_string(),
            name: "Redmi Note 9".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Mi A2 (jasmine_sprout)
        self.devices.insert("jasmine_sprout".to_string(), Device {
            codename: "jasmine_sprout".to_string(),
            name: "Mi A2".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["Mi 6X".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Mi 6 (sagit)
        self.devices.insert("sagit".to_string(), Device {
            codename: "sagit".to_string(),
            name: "Mi 6".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: true,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Redmi Note 8 (2021) (biloba)
        self.devices.insert("biloba".to_string(), Device {
            codename: "biloba".to_string(),
            name: "Redmi Note 8 (2021)".to_string(),
            maker: "Xiaomi".to_string(),
            experimental: true,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec![],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // ZenFone Max Pro M1 (x00td)
        self.devices.insert("x00td".to_string(), Device {
            codename: "x00td".to_string(),
            name: "ZenFone Max Pro M1".to_string(),
            maker: "ASUS".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
            ],
            aliases: vec!["ZB601KL".to_string(), "ZB602KL".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Galaxy S7 Exynos (herolte)
        self.devices.insert("herolte".to_string(), Device {
            codename: "herolte".to_string(),
            name: "Galaxy S7 (Exynos)".to_string(),
            maker: "Samsung".to_string(),
            experimental: true,
            battery_min: 50,
            warnings: vec![
                "Exynos variant only — Snapdragon variant is not supported".to_string(),
                "Samsung devices use Odin/Heimdall for flashing, not standard fastboot".to_string(),
            ],
            aliases: vec!["SM-G930F".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Galaxy S7 Edge Exynos (hero2lte)
        self.devices.insert("hero2lte".to_string(), Device {
            codename: "hero2lte".to_string(),
            name: "Galaxy S7 Edge (Exynos)".to_string(),
            maker: "Samsung".to_string(),
            experimental: true,
            battery_min: 50,
            warnings: vec![
                "Exynos variant only — Snapdragon variant is not supported".to_string(),
                "Samsung devices use Odin/Heimdall for flashing, not standard fastboot".to_string(),
            ],
            aliases: vec!["SM-G935F".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Sony Xperia X (suzu)
        self.devices.insert("suzu".to_string(), Device {
            codename: "suzu".to_string(),
            name: "Sony Xperia X".to_string(),
            maker: "Sony".to_string(),
            experimental: false,
            battery_min: 50,
            warnings: vec![
                "Unlocking will factory reset the device".to_string(),
                "Sony DRM keys will be permanently lost after unlocking".to_string(),
            ],
            aliases: vec!["Xperia X".to_string(), "F5121".to_string()],
            variants: vec![],
            is_locked: None,
            serial: None,
            android_version: None,
            build_id: None,
            battery_level: None,
        });

        // Load unlocking steps for each device
        self.load_unlocking_steps();

        // Load distros for each device
        self.load_distros();
    }

    fn load_unlocking_steps(&mut self) {
        // Common unlocking steps for Pixel 3a
        self.unlocking_steps.insert("sargo".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus 6 steps (similar but with some differences)
        self.unlocking_steps.insert("enchilada".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Motorola Edge 30 steps
        self.unlocking_steps.insert("dubai".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'. Note: this option may take up to a week to appear after first setup.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("OEM Unlock may take up to one week to become available".to_string()),
            },
            UnlockingStep {
                order: 3,
                title: "Get Unlock Code from Motorola".to_string(),
                description: "Visit the Motorola bootloader unlock page to request an unlock code for your device".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command with the code from Motorola".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock <code>".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Surface Duo steps
        self.unlocking_steps.insert("zeta".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Microsoft may detect this and void warranty".to_string()),
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Hold Power + Volume Down when the device is off".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Xiaomi POCO M2 Pro (miatoll) — Xiaomi requires Mi Unlock tool
        self.unlocking_steps.insert("miatoll".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // F(x)tec Pro1 (pro1) — standard OEM unlock
        self.unlocking_steps.insert("pro1".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // F(x)tec Pro1 X (pro1x) — standard OEM unlock
        self.unlocking_steps.insert("pro1x".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Sony Xperia 5 (bahamut) — Sony requires unlock code from website
        self.unlocking_steps.insert("bahamut".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Get Unlock Code from Sony".to_string(),
                description: "Visit Sony's bootloader unlock page to request an unlock code for your device. You will need your IMEI number.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Unlocking will permanently void DRM keys (camera quality may be affected)".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command with the code from Sony".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock 0x<code>".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Volla Phone (yggdrasil) — MediaTek, standard OEM unlock
        self.unlocking_steps.insert("yggdrasil".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Volla Phone X (yggdrasilx) — same as Volla Phone
        self.unlocking_steps.insert("yggdrasilx".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Volla Phone 22 (mimameid)
        self.unlocking_steps.insert("mimameid".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Volla Phone X23 (vidofnir)
        self.unlocking_steps.insert("vidofnir".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Volla Phone Quintus (algiz)
        self.unlocking_steps.insert("algiz".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Lenovo ThinkPhone (bronco) — Motorola unlock process
        self.unlocking_steps.insert("bronco".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'. Note: this option may take up to a week to appear after first setup.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("OEM Unlock may take up to one week to become available".to_string()),
            },
            UnlockingStep {
                order: 3,
                title: "Get Unlock Code from Motorola".to_string(),
                description: "Visit the Motorola bootloader unlock page to request an unlock code for your device".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command with the code from Motorola".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock <code>".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Volla Tablet (mimir)
        self.unlocking_steps.insert("mimir".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus 6T (fajita) — same as enchilada
        self.unlocking_steps.insert("fajita".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Xiaomi Pocophone F1 (beryllium) — Xiaomi Mi Unlock pattern
        self.unlocking_steps.insert("beryllium".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Fairphone 4 (FP4) — standard fastboot flashing unlock
        self.unlocking_steps.insert("FP4".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Fairphone 5 (FP5) — standard fastboot flashing unlock
        self.unlocking_steps.insert("FP5".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // SHIFT6mq (axolotl) — standard fastboot flashing unlock
        self.unlocking_steps.insert("axolotl".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Motorola Moto Z (griffin) — Motorola unlock code
        self.unlocking_steps.insert("griffin".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'. Note: this option may take up to a week to appear after first setup.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("OEM Unlock may take up to one week to become available".to_string()),
            },
            UnlockingStep {
                order: 3,
                title: "Get Unlock Code from Motorola".to_string(),
                description: "Visit the Motorola bootloader unlock page to request an unlock code for your device".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command with the code from Motorola".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock <code>".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus 3 (oneplus3) — standard OEM unlock
        self.unlocking_steps.insert("oneplus3".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Xiaomi Redmi Note 7 Pro (violet) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("violet".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Sony Xperia 5 II (pdx206) — Sony unlock code
        self.unlocking_steps.insert("pdx206".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Get Unlock Code from Sony".to_string(),
                description: "Visit Sony's bootloader unlock page to request an unlock code for your device. You will need your IMEI number.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Unlocking will permanently void DRM keys (camera quality may be affected)".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command with the code from Sony".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock 0x<code>".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Fairphone 3/3+ (fp3) — standard fastboot flashing unlock
        self.unlocking_steps.insert("fp3".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Fairphone 2 (fp2) — standard fastboot flashing unlock
        self.unlocking_steps.insert("fp2".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Google Pixel 3a XL (bonito) — same as sargo
        self.unlocking_steps.insert("bonito".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus Nord N10 5G (billie) — standard OEM unlock
        self.unlocking_steps.insert("billie".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus Nord N100 (billie2) — standard OEM unlock
        self.unlocking_steps.insert("billie2".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus 5 (cheeseburger) — standard OEM unlock
        self.unlocking_steps.insert("cheeseburger".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus 5T (dumpling) — standard OEM unlock
        self.unlocking_steps.insert("dumpling".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus Nord 2 5G (denniz) — standard OEM unlock
        self.unlocking_steps.insert("denniz".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // OnePlus One (bacon) — standard OEM unlock
        self.unlocking_steps.insert("bacon".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command. Use volume keys to confirm on device.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // POCO X3 NFC (surya) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("surya".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Redmi Note 8 Pro (begonia) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("begonia".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Redmi Note 7 (lavender) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("lavender".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // POCO M3 (citrus) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("citrus".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Redmi 9 (lancelot) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("lancelot".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Redmi Note 9 (merlin) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("merlin".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Mi A2 (jasmine_sprout) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("jasmine_sprout".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Mi 6 (sagit) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("sagit".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Redmi Note 8 2021 (biloba) — Xiaomi Mi Unlock
        self.unlocking_steps.insert("biloba".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'MIUI version' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Link Mi Account".to_string(),
                description: "Go to Settings > Additional Settings > Developer Options > Mi Unlock status and link your Mi account. Wait for the required unlock period (up to 168 hours).".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Xiaomi enforces a waiting period before unlocking".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run unlock command via fastboot".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot flashing unlock".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // ZenFone Max Pro M1 (x00td) — ASUS unlock-go
        self.unlocking_steps.insert("x00td".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 4,
                title: "Unlock Bootloader".to_string(),
                description: "Run ASUS unlock command. Confirm on device screen.".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock-go".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);

        // Galaxy S7 Exynos (herolte) — Samsung OEM toggle only
        self.unlocking_steps.insert("herolte".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'. This is sufficient for Samsung Exynos devices.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Samsung Exynos devices use Odin/Heimdall, not standard fastboot".to_string()),
            },
            UnlockingStep {
                order: 3,
                title: "Enter Download Mode".to_string(),
                description: "Power off the device, then hold Volume Down + Home + Power to enter Download Mode. Press Volume Up to confirm.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
        ]);

        // Galaxy S7 Edge Exynos (hero2lte) — Samsung OEM toggle only
        self.unlocking_steps.insert("hero2lte".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'. This is sufficient for Samsung Exynos devices.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Samsung Exynos devices use Odin/Heimdall, not standard fastboot".to_string()),
            },
            UnlockingStep {
                order: 3,
                title: "Enter Download Mode".to_string(),
                description: "Power off the device, then hold Volume Down + Home + Power to enter Download Mode. Press Volume Up to confirm.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
        ]);

        // Sony Xperia X (suzu) — Sony unlock code
        self.unlocking_steps.insert("suzu".to_string(), vec![
            UnlockingStep {
                order: 1,
                title: "Enable Developer Options".to_string(),
                description: "Go to Settings > About Phone and tap 'Build Number' 7 times".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 2,
                title: "Enable OEM Unlocking".to_string(),
                description: "Go to Settings > System > Developer Options and enable 'OEM unlocking'".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 3,
                title: "Get Unlock Code from Sony".to_string(),
                description: "Visit Sony's bootloader unlock page to request an unlock code for your device. You will need your IMEI number.".to_string(),
                step_type: crate::models::StepType::Manual,
                command: None,
                duration_secs: None,
                optional: false,
                warning: Some("Unlocking will permanently void DRM keys (camera quality may be affected)".to_string()),
            },
            UnlockingStep {
                order: 4,
                title: "Reboot to Bootloader".to_string(),
                description: "Reboot the device into fastboot mode".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("adb reboot bootloader".to_string()),
                duration_secs: Some(10),
                optional: false,
                warning: None,
            },
            UnlockingStep {
                order: 5,
                title: "Unlock Bootloader".to_string(),
                description: "Run OEM unlock command with the code from Sony".to_string(),
                step_type: crate::models::StepType::Automated,
                command: Some("fastboot oem unlock 0x<code>".to_string()),
                duration_secs: Some(30),
                optional: false,
                warning: Some("This will factory reset your device!".to_string()),
            },
        ]);
    }

    fn load_distros(&mut self) {
        // postmarketOS for Pixel 3a
        self.distros.insert("sargo".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/google-sargo/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/google-sargo/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-google-sargo.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(500_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
            Distro {
                name: "/e/OS".to_string(),
                version: "3.4".to_string(),
                description: "De-Googled Android-based mobile OS focused on privacy".to_string(),
                download_base_url: "https://images.ecloud.global/community/sargo/".to_string(),
                checksum_url: None,
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "recovery-e-community-sargo.img".to_string(),
                        erase_first: false,
                    },
                ],
                homepage: Some("https://e.foundation".to_string()),
                download_size_bytes: Some(1_500_000_000),
                requires_unlock: true,
                post_install_notes: Some("Flash recovery, then sideload ROM zip via adb sideload".to_string()),
            },
        ]);

        // postmarketOS for OnePlus 6
        self.distros.insert("enchilada".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/oneplus-enchilada/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/oneplus-enchilada/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-oneplus-enchilada.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // OnePlus 6T (fajita) - postmarketOS
        self.distros.insert("fajita".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/oneplus-fajita/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/oneplus-fajita/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-oneplus-fajita.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Pocophone F1 (beryllium) - postmarketOS
        self.distros.insert("beryllium".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/xiaomi-beryllium/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/xiaomi-beryllium/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-xiaomi-beryllium.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Fairphone 4 (FP4) - postmarketOS
        self.distros.insert("FP4".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/fairphone-fp4/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/fairphone-fp4/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-fairphone-fp4.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Fairphone 5 (FP5) - postmarketOS
        self.distros.insert("FP5".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/fairphone-fp5/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/fairphone-fp5/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-fairphone-fp5.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // SHIFT6mq (axolotl) - postmarketOS
        self.distros.insert("axolotl".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/shift-axolotl/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/shift-axolotl/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-shift-axolotl.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // postmarketOS for Surface Duo
        self.distros.insert("zeta".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux (experimental)".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/microsoft-zeta/".to_string(),
                checksum_url: None,
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(400_000_000),
                requires_unlock: true,
                post_install_notes: Some("Dual screen support is experimental".to_string()),
            },
        ]);

        // OnePlus 3 (oneplus3) - postmarketOS
        self.distros.insert("oneplus3".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/oneplus-oneplus3/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/oneplus-oneplus3/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-oneplus-oneplus3.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(500_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Motorola Edge 30 (dubai) - /e/OS
        self.distros.insert("dubai".to_string(), vec![
            Distro {
                name: "/e/OS".to_string(),
                version: "3.4".to_string(),
                description: "De-Googled Android-based mobile OS focused on privacy".to_string(),
                download_base_url: "https://images.ecloud.global/community/dubai/".to_string(),
                checksum_url: None,
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "recovery-IMG-e-community-dubai.zip".to_string(),
                        erase_first: false,
                    },
                ],
                homepage: Some("https://e.foundation".to_string()),
                download_size_bytes: Some(2_000_000_000),
                requires_unlock: true,
                post_install_notes: Some("Flash recovery (dtbo + vendor_boot), then sideload ROM zip via adb sideload".to_string()),
            },
        ]);

        // Fairphone 3/3+ (fp3) - postmarketOS
        self.distros.insert("fp3".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/fairphone-fp3/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/fairphone-fp3/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-fairphone-fp3.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Fairphone 2 (fp2) - postmarketOS
        self.distros.insert("fp2".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/fairphone-fp2/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/fairphone-fp2/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-fairphone-fp2.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(500_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Google Pixel 3a XL (bonito) - postmarketOS
        self.distros.insert("bonito".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/google-bonito/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/google-bonito/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-google-bonito.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(500_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // OnePlus Nord N10 5G (billie) — UBports only in load_distros
        self.distros.insert("billie".to_string(), vec![]);

        // OnePlus Nord N100 (billie2) — UBports only
        self.distros.insert("billie2".to_string(), vec![]);

        // OnePlus 5 (cheeseburger) - postmarketOS
        self.distros.insert("cheeseburger".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/oneplus-cheeseburger/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/oneplus-cheeseburger/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-oneplus-cheeseburger.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // OnePlus 5T (dumpling) - postmarketOS
        self.distros.insert("dumpling".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/oneplus-dumpling/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/oneplus-dumpling/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-oneplus-dumpling.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // OnePlus Nord 2 5G (denniz) — UBports only
        self.distros.insert("denniz".to_string(), vec![]);

        // OnePlus One (bacon) - postmarketOS
        self.distros.insert("bacon".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/oneplus-bacon/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/oneplus-bacon/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-oneplus-bacon.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(500_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // POCO X3 NFC (surya) - postmarketOS
        self.distros.insert("surya".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "24.06".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/v24.06/xiaomi-surya/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/v24.06/xiaomi-surya/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-xiaomi-surya.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Redmi Note 8 Pro (begonia) - postmarketOS
        self.distros.insert("begonia".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/xiaomi-begonia/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/xiaomi-begonia/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-xiaomi-begonia.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Redmi Note 7 (lavender) - postmarketOS
        self.distros.insert("lavender".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/xiaomi-lavender/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/xiaomi-lavender/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-xiaomi-lavender.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // POCO M3 (citrus) — UBports only
        self.distros.insert("citrus".to_string(), vec![]);

        // Redmi 9 (lancelot) - postmarketOS
        self.distros.insert("lancelot".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/xiaomi-lancelot/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/xiaomi-lancelot/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-xiaomi-lancelot.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Redmi Note 9 (merlin) — UBports only
        self.distros.insert("merlin".to_string(), vec![]);

        // Mi A2 (jasmine_sprout) - postmarketOS
        self.distros.insert("jasmine_sprout".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/xiaomi-jasmine_sprout/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/xiaomi-jasmine_sprout/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-xiaomi-jasmine_sprout.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Mi 6 (sagit) - postmarketOS
        self.distros.insert("sagit".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/xiaomi-sagit/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/xiaomi-sagit/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-xiaomi-sagit.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Redmi Note 8 2021 (biloba) — UBports only
        self.distros.insert("biloba".to_string(), vec![]);

        // ZenFone Max Pro M1 (x00td) - postmarketOS
        self.distros.insert("x00td".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux distribution".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/asus-x00td/".to_string(),
                checksum_url: Some("https://images.postmarketos.org/bpo/edge/asus-x00td/SHA256SUMS".to_string()),
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                    PartitionImage {
                        partition: "system".to_string(),
                        image: "rootfs-asus-x00td.img.xz".to_string(),
                        erase_first: true,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(550_000_000),
                requires_unlock: true,
                post_install_notes: Some("First boot may take several minutes".to_string()),
            },
        ]);

        // Galaxy S7 Exynos (herolte) - postmarketOS
        self.distros.insert("herolte".to_string(), vec![
            Distro {
                name: "postmarketOS".to_string(),
                version: "edge".to_string(),
                description: "Alpine-based mobile Linux (experimental)".to_string(),
                download_base_url: "https://images.postmarketos.org/bpo/edge/samsung-herolte/".to_string(),
                checksum_url: None,
                partitions: vec![
                    PartitionImage {
                        partition: "boot".to_string(),
                        image: "boot.img".to_string(),
                        erase_first: false,
                    },
                ],
                homepage: Some("https://postmarketos.org".to_string()),
                download_size_bytes: Some(400_000_000),
                requires_unlock: true,
                post_install_notes: Some("Exynos platform — experimental support".to_string()),
            },
        ]);

        // Galaxy S7 Edge Exynos (hero2lte) — UBports only
        self.distros.insert("hero2lte".to_string(), vec![]);

        // Sony Xperia X (suzu) — UBports only (no pOS stable)
        self.distros.insert("suzu".to_string(), vec![]);
    }

    /// Find a device by its codename
    pub fn find_by_codename(&self, codename: &str) -> Option<Device> {
        // Direct lookup
        if let Some(device) = self.devices.get(codename) {
            return Some(device.clone());
        }

        // Check aliases
        for device in self.devices.values() {
            if device.matches_codename(codename) {
                return Some(device.clone());
            }
        }

        None
    }

    /// Get unlocking steps for a device
    pub fn get_unlocking_steps(&self, codename: &str) -> Vec<UnlockingStep> {
        self.unlocking_steps
            .get(codename)
            .cloned()
            .unwrap_or_default()
    }

    /// Get available distros for a device
    pub fn get_distros(&self, codename: &str) -> Vec<Distro> {
        self.distros.get(codename).cloned().unwrap_or_default()
    }

    /// Get all supported device codenames
    pub fn all_codenames(&self) -> Vec<String> {
        self.devices.keys().cloned().collect()
    }

    /// Get all supported devices
    pub fn all_devices(&self) -> Vec<Device> {
        self.devices.values().cloned().collect()
    }
}

impl Default for DeviceDatabase {
    fn default() -> Self {
        Self::new()
    }
}
