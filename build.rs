use std::env;
use std::path::PathBuf;
// use std::fs;

fn main() {
    let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let darts_inc_dir: PathBuf = [&project_root, "darts-clone", "include"].iter().collect();
    let bridge_inc_dir: PathBuf = [&project_root, "src"].iter().collect();

    println!("{:?}", darts_inc_dir);
    println!("{:?}", bridge_inc_dir);

    cxx_build::bridge("src/lib.rs")  // returns a cc::Build
        .file("src/bridge.cpp")
        .flag_if_supported("-std=c++14")
        .flag_if_supported("-Wno-ignored-qualifiers")
        .include(&bridge_inc_dir)
        .include(&darts_inc_dir)
        .compile("darts-clone-demo");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/bridge.cpp");
    println!("cargo:rerun-if-changed=src/bridge.h");
    println!("cargo:rerun-if-changed=darts-clone/include/darts.h");
}
