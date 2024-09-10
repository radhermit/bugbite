use std::env;
use std::fs::{self, File};
use std::io::Write;

/// Bundle all services into a single file that is statically injected.
fn bundle_services(path: &str) {
    let out_path = format!("{}/services.toml", env::var("OUT_DIR").unwrap());
    let mut f = File::create(&out_path).unwrap();
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let data = fs::read(&path).unwrap();
        let name = path.file_stem().unwrap().to_str().unwrap();
        writeln!(f, "[{name}]").unwrap();
        f.write_all(&data).unwrap();
    }
}

fn main() {
    let services_path = format!("{}/services", env!("CARGO_MANIFEST_DIR"));
    bundle_services(&services_path);
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed={services_path}");
}
