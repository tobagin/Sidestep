use std::env;
use std::path::PathBuf;

fn main() {
    // For meson builds, gresources are compiled by meson in data/meson.build
    // This build.rs is kept minimal - just rerun triggers
    
    // Only compile gresources if building outside meson (e.g., cargo build directly)
    // Check if we're in a meson build by looking for the meson build directory marker
    let is_meson_build = env::var("MESON_BUILD_ROOT").is_ok() 
        || PathBuf::from("_flatpak_build").exists();
    
    if !is_meson_build {
        // For standalone cargo builds, we skip gresource compilation
        // since it requires blueprint-compiler and compiled UI files
        println!("cargo:warning=Building outside meson - gresources not compiled");
    }
    
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=data/resources.gresource.xml");
    println!("cargo:rerun-if-changed=data/ui/");
}
