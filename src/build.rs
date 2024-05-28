use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        if let Err(err) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", err);
        }
    }

    for directory in ["resources/server", "resources/web", "resources/combined", "resources/database"] {
        setup(directory);
    }
}

fn setup(directory_path: &str) {
    if let Ok(entries) = fs::read_dir(directory_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_path = entry.path();
                let file_name = entry.file_name();

                if file_path.is_file() {
                    let mut file = File::open(&file_path).expect("Failed to open file");
                    let mut binary_data = Vec::new();
                    file.read_to_end(&mut binary_data).expect("Failed to read file");

                    let out_dir = match std::env::var("OUT_DIR") {
                        Ok(out_dir) => out_dir,
                        Err(err) => {
                            eprintln!("Failed to get OUT_DIR: {}", err);
                            return;
                        }
                    };
                    let file_stem = file_name.to_string_lossy().replace(".", "_");
                    let dest_path = Path::new(&out_dir).join(
                        format!("{}.rs", file_stem.to_lowercase())
                    );
                    let mut dest_file = File::create(&dest_path).expect("Failed to create file");

                    let data = binary_data
                        .iter()
                        .map(|byte| byte.to_string())
                        .collect::<Vec<_>>()
                        .join(",");

                    write!(
                        &mut dest_file,
                        "pub(crate) const {}: &[u8] = &[{}];",
                        file_stem.to_uppercase(),
                        data
                    ).expect("Failed to write to file");

                    println!("cargo:rerun-if-changed={}", file_path.to_string_lossy());
                }
            }
        }
    }
}
