use std::{path::PathBuf, process::Command};

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let shader = "src/shaders/shader.slang";
    let shader_out = out_dir.join("shader.spv");

    println!("cargo:rerun-if-changed={shader}");

    let status = Command::new("slangc")
        .args([
            shader,
            "-target",
            "spirv",
            "-profile",
            "spirv_1_6",
            "-emit-spirv-directly",
            "-fvk-use-entrypoint-name",
            "-entry",
            "vertMain",
            "-entry",
            "fragMain",
            "-o",
            shader_out.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run slangc");

    assert!(status.success(), "slangc compilation failed");
}
