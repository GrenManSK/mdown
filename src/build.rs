use std::{ fs::{ self, File }, io::prelude::*, path::Path };

/// The build script for configuring and processing project resources.
///
/// This script performs different tasks based on the target OS and resource directories. It handles resource compilation for Windows, processes files in specified resource directories, and generates corresponding Rust source files with binary data. It also handles conditional compilation flags related to music resources.
fn main() {
    // For Windows OS, compile Windows-specific resources such as icons.
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        if let Err(err) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", err);
        }
    }

    // Iterate over predefined directories and process each one.
    for directory in [
        "resources/combined",
        "resources/database",
        "resources/music",
        "resources/server",
        "resources/web",
    ] {
        setup(directory);
    }
}

/// Processes files in a given directory and generates corresponding Rust source files.
///
/// # Parameters
///
/// - `directory_path: &str`: The path to the directory containing files to be processed.
///
/// This function reads each file in the specified directory, converts its content into binary data, and writes the binary data into a new Rust source file in the `OUT_DIR`. The new file contains a constant array of bytes representing the file's content. Additionally, it sets up cargo rerun-if-changed triggers for the processed files.
///
/// The function also handles conditional compilation flags for music-related resources if the "music" feature is enabled.
fn setup(directory_path: &str) {
    // Read the contents of the directory.
    if let Ok(entries) = fs::read_dir(directory_path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            let file_name = entry.file_name();

            // Skip non-file entries.
            if !file_path.is_file() {
                continue;
            }

            // Open the file and read its contents.
            let mut file = match File::open(&file_path) {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("Failed to open file {}: {}", file_path.display(), err);
                    continue;
                }
            };
            let mut binary_data = Vec::new();
            match file.read_to_end(&mut binary_data) {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Failed to read file {}: {}", file_path.display(), err);
                    continue;
                }
            }

            // Determine the output directory for generated Rust files.
            let out_dir = match std::env::var("OUT_DIR") {
                Ok(out_dir) => out_dir,
                Err(err) => {
                    eprintln!("Failed to get OUT_DIR: {}", err);
                    return;
                }
            };
            let file_stem = file_name.to_string_lossy().replace(".", "_");
            let dest_path = Path::new(&out_dir).join(format!("{}.rs", file_stem.to_lowercase()));
            let mut dest_file = match File::create(&dest_path) {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("Failed to create file: {}", err);
                    continue;
                }
            };

            // Write the binary data as a Rust constant.
            let data = binary_data
                .iter()
                .map(|byte| byte.to_string())
                .collect::<Vec<_>>()
                .join(",");

            match
                write!(
                    &mut dest_file,
                    "pub(crate) const {}: &[u8] = &[{}];",
                    file_stem.to_uppercase(),
                    data
                )
            {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Failed to write to file: {}", err);
                    continue;
                }
            }

            // Set up cargo to re-run this build script if the file changes.
            println!("cargo:rerun-if-changed={}", file_path.to_string_lossy());
        }

        // If the "music" feature is enabled, conditionally set compilation flags based on the presence of certain files.
        #[cfg(feature = "music")]
        if directory_path == "resources/music" {
            println!("cargo::rustc-check-cfg=cfg(music_m1)");
            println!("cargo::rustc-check-cfg=cfg(music_m2)");
            println!("cargo::rustc-check-cfg=cfg(music_m3)");
            println!("cargo::rustc-check-cfg=cfg(music_m4)");
            println!("cargo::rustc-check-cfg=cfg(music_m5)");
            let out_dir = match std::env::var("OUT_DIR") {
                Ok(out_dir) => out_dir,
                Err(err) => {
                    eprintln!("Failed to get OUT_DIR: {}", err);
                    return;
                }
            };
            if
                Path::new(&format!("{}/m1_combat_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m1_end_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m1_start_c_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m1_stealth_mp3.rs", out_dir)).exists()
            {
                println!("cargo:rustc-cfg=music_m1");
            }
            if
                Path::new(&format!("{}/m2_combat_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m2_end_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m2_start_c_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m2_stealth_mp3.rs", out_dir)).exists()
            {
                println!("cargo:rustc-cfg=music_m2");
            }
            if
                Path::new(&format!("{}/m3_combat_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m3_end_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m3_start_c_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m3_stealth_mp3.rs", out_dir)).exists()
            {
                println!("cargo:rustc-cfg=music_m3");
            }
            if
                Path::new(&format!("{}/m4_combat_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m4_end_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m4_start_c_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m4_stealth_mp3.rs", out_dir)).exists()
            {
                println!("cargo:rustc-cfg=music_m4");
            }
            if
                Path::new(&format!("{}/m5_combat_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m5_end_mp3.rs", out_dir)).exists() &&
                Path::new(&format!("{}/m5_start_c_mp3.rs", out_dir)).exists()
            {
                println!("cargo:rustc-cfg=music_m5");
            }
        }
    }
}
