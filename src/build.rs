use std::{ fs::{ self, File }, io::prelude::*, path::Path };

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        if let Err(err) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", err);
        }
    }

    for directory in [
        "resources/server",
        "resources/web",
        "resources/combined",
        "resources/database",
        "resources/music",
    ] {
        setup(directory);
    }
}

fn setup(directory_path: &str) {
    if let Ok(entries) = fs::read_dir(directory_path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            let file_name = entry.file_name();

            if !file_path.is_file() {
                continue;
            }
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

            println!("cargo:rerun-if-changed={}", file_path.to_string_lossy());
        }
        #[cfg(feature = "music")]
        if directory_path == "resources/music" {
            println!("cargo::rustc-check-cfg=cfg(music_m1)");
            println!("cargo::rustc-check-cfg=cfg(music_m2)");
            println!("cargo::rustc-check-cfg=cfg(music_m3)");
            println!("cargo::rustc-check-cfg=cfg(music_m4)");
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
                Path::new(&format!("{}/m3_start_c_mp3.rs", out_dir)).exists()
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
        }
    }
}
