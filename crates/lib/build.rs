use std::env;
use std::fs::{self, File};
use std::io::Write;

/// Bundle all services into a single file that is statically injected.
fn bundle_services() {
    let services_path = format!("{}/../../services", env!("CARGO_MANIFEST_DIR"));
    let out_path = format!("{}/services.toml", env::var("OUT_DIR").unwrap());
    let mut f = File::create(&out_path).unwrap();
    for entry in fs::read_dir(&services_path).unwrap() {
        let entry = entry.unwrap();
        let data = fs::read(entry.path()).unwrap();
        f.write_all(&data).unwrap();
    }
}

fn main() {
    bundle_services();
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=services");
}
