use bytes::BytesMut;
use chrono::{ NaiveDateTime, Local };
use semver::{ BuildMetadata, Prerelease, Version, VersionReq };
use std::{ fs::{ File, write }, io::Write, process::Command };
use sha2::{ Digest, Sha256 };

use crate::{
    db,
    download,
    debug,
    error::MdownError,
    getter::{ get_dat_path, get_exe_path, get_exe_file_path, get_exe_name },
    metadata::Dat,
};

pub const DB_VERSION: &str = "0000";

/// Checks and updates the version in the provided `Dat` object.
///
/// This function compares the given `version` with the `current_version` of the application
/// and updates the `Dat` object if the version needs to be changed.
/// It uses semantic versioning and checks if the provided `version` is less than the current version.
/// If the version is out of date, it updates the `Dat` object and writes the changes to disk.
///
/// # Arguments
/// * `dat` - A mutable reference to the `Dat` object that stores version information.
/// * `version` - The version of the data that is currently being checked.
/// * `current_version` - The current version of the application.
///
/// # Returns
/// * `Result<bool, MdownError>` - Returns `Ok(false)` if no user confirmation is required after the version change,
///   or an error if something goes wrong (such as file I/O or JSON serialization).
///
/// # Errors
/// * `MdownError::IoError` - If there is an issue reading or writing to the file.
/// * `MdownError::JsonError` - If JSON serialization of the `Dat` object fails.
///
/// # Example
/// ```rust
/// let mut dat = Dat { version: "1.2.3".to_string() };
/// let result = check_ver(&mut dat, Version::parse("1.0.0").unwrap(), Version::parse("2.0.0").unwrap());
/// match result {
///     Ok(false) => println!("Version updated without user confirmation."),
///     Ok(true) => println!("User confirmation required."),
///     Err(e) => eprintln!("Error: {:?}", e),
/// }
/// ```
///
/// # Panics
/// * The function will panic if the version parsing with `VersionReq::parse` fails,
///   though this should not occur with valid version strings.
pub(crate) fn check_ver(
    dat: &mut Dat,
    mut version: Version,
    current_version: Version
) -> Result<bool, MdownError> {
    let req_ver_text = format!("<{}", get_current_version());
    let req1 = VersionReq::parse(&req_ver_text).unwrap();

    let require_confirmation_from_user = false;

    loop {
        if req1.matches(&version) {
            let version_to_change_to = remove_prerelease(&current_version.to_string());
            println!("Changing to version: {}", version_to_change_to);
            dat.version = version_to_change_to.clone();

            let dat_path = match get_dat_path() {
                Ok(path) => path,
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 11621));
                }
            };

            let mut file = match File::create(&dat_path) {
                Ok(path) => path,
                Err(err) => {
                    return Err(MdownError::IoError(err, dat_path, 11600));
                }
            };

            let json = match serde_json::to_value::<Dat>(dat.clone()) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string(), 11601));
                }
            };

            let json_string = match serde_json::to_string_pretty(&json) {
                Ok(value) => value,
                Err(err) => {
                    return Err(MdownError::JsonError(err.to_string(), 11602));
                }
            };

            if let Err(err) = writeln!(file, "{}", json_string) {
                return Err(MdownError::IoError(err, dat_path, 11603));
            }

            version = match Version::parse(&get_current_version()) {
                Ok(version) => version,
                Err(_err) => version_new(),
            };
        } else {
            break;
        }
    }
    Ok(require_confirmation_from_user)
}

fn version_new() -> Version {
    Version {
        major: 0,
        minor: 0,
        patch: 0,
        pre: Prerelease::EMPTY,
        build: BuildMetadata::EMPTY,
    }
}

pub(crate) fn check_app_ver() -> Result<bool, MdownError> {
    let req_ver_text = format!("<{}", get_current_version());
    let req1 = VersionReq::parse(&req_ver_text).unwrap();

    let require_confirmation_from_user = false;

    let current_version = match Version::parse(&get_current_version()) {
        Ok(version) => version,
        Err(_err) => version_new(),
    };

    let mut version = match db::read_resource_lone(DB_VERSION) {
        Ok(Some(version)) => {
            debug!("Current version from database {}", version);
            match Version::parse(&version) {
                Ok(version) => version,
                Err(_err) => version_new(),
            }
        }
        Ok(None) => {
            debug!("Writing to database version");
            return match
                db::write_resource_lone(DB_VERSION, get_current_version().as_bytes(), false)
            {
                Ok(_) => Ok(false),
                Err(err) => Err(MdownError::ChainedError(Box::new(err), 11633)),
            };
        }
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 11622));
        }
    };

    loop {
        if req1.matches(&version) {
            let version_to_change_to = current_version.to_string();
            println!("Changing to version: {}", version_to_change_to);
            match db::write_resource_lone(DB_VERSION, get_current_version().as_bytes(), false) {
                Ok(_) => (),
                Err(err) => {
                    return Err(MdownError::ChainedError(Box::new(err), 11623));
                }
            }
            version = current_version.clone();
        } else {
            break;
        }
    }
    Ok(require_confirmation_from_user)
}

pub(crate) async fn app_update() -> Result<bool, MdownError> {
    debug!("app_update");

    let (current_version, latest_version, data, client) = match version_preparation().await {
        Ok(t) => t,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 11624));
        }
    };
    if latest_version > current_version {
        debug!("New version available: {}", latest_version);
        let target_files = ["mdown.exe", "mdown_min.exe"];
        let current_name = match get_exe_name() {
            Ok(name) => name,
            Err(err) => {
                return Err(MdownError::ChainedError(Box::new(err), 11625));
            }
        };

        let asset_url = match search_url(&data, &current_name) {
            Ok(value) => value,
            Err(err) => {
                return Err(MdownError::ChainedError(Box::new(err), 11626));
            }
        };

        let target_file = if target_files.contains(&current_name.as_str()) {
            current_name.as_str()
        } else {
            "mdown.exe"
        };
        let body = match data["body"].as_str() {
            Some(s) => s,
            None => {
                return Err(
                    MdownError::ConversionError(
                        String::from("Body could not be converted to string"),
                        11609
                    )
                );
            }
        };
        let checksum = match
            body
                .lines()
                .skip_while(|line| !line.contains("## SHA256"))
                .skip_while(|line| !line.contains(target_file))
                .nth(2)
                .map(str::trim)
                .ok_or("Checksum not found")
        {
            Ok(checksum) => checksum,
            Err(err) => {
                return Err(MdownError::NotFoundError(err.to_string(), 11610));
            }
        };
        let checksum = &checksum[1..checksum.len() - 1];

        debug!("Checksum for {}: {}", target_file, checksum);
        debug!("Downloading from {}", asset_url);

        let mut binary_data = BytesMut::new();

        let mut response = match download::get_response_from_client(&asset_url, &client).await {
            Ok(response) => response,
            Err(err) => {
                return Err(MdownError::ChainedError(Box::new(err), 11627));
            }
        };

        let (total_size, final_size_string) = download::get_size(&response);
        let (mut downloaded, mut last_size) = (0, 0);
        let interval = std::time::Duration::from_millis(250);
        let mut last_check_time = std::time::Instant::now();

        while
            //prettier-ignore
            let Some(chunk) = match response.chunk().await {
                Ok(Some(chunk)) => Some(chunk),
                Ok(None) => None,
                Err(err) => {
                    return Err(MdownError::NetworkError(err, 11619));
                }
            }
        {
            binary_data.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            let current_time = std::time::Instant::now();
            if current_time.duration_since(last_check_time) >= interval {
                last_check_time = current_time;
                let percentage = (100.0 / (total_size as f32)) * (downloaded as f32);
                let perc_string = download::get_perc(percentage);
                let current_mbs = bytefmt::format(downloaded - last_size);
                let current_mb = bytefmt::format(downloaded);
                println!(
                    "Downloading {} {}% - {} of {} [{}/s]",
                    target_file,
                    perc_string,
                    current_mb,
                    final_size_string,
                    current_mbs
                );
                last_size = downloaded;
            }
        }

        let mut hasher = Sha256::new();
        debug!("Calculating checksum");
        hasher.update(&binary_data);
        let calculated_hash = format!("{:x}", hasher.finalize());

        debug!("Checksum for downloaded file: {}", calculated_hash);
        if calculated_hash != checksum {
            return Err(
                MdownError::CustomError(
                    String::from("Checksum verification failed"),
                    String::from(""),
                    11614
                )
            );
        }

        let current_exe = match get_exe_path() {
            Ok(path) => path,
            Err(err) => {
                return Err(MdownError::ChainedError(Box::new(err), 11628));
            }
        };

        let temp_dir = std::env::temp_dir();
        let temp_exe = match temp_dir.join("mdown.exe").to_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(
                    MdownError::ConversionError(
                        String::from("Temp directory path could not be converted to string"),
                        11613
                    )
                );
            }
        };
        match write(&temp_exe, binary_data) {
            Ok(_) => (),
            Err(err) => {
                return Err(MdownError::IoError(err, temp_exe, 11612));
            }
        }
        let batch_script = format!(
            "@echo off\n\
             timeout /t 1 /nobreak >nul\n\
             move \"{}\" \"{}\" >nul\n\
             >nul 2>nul del \"%~f0\" & exit\n",
            temp_exe,
            current_exe
        );

        let script_path = match temp_dir.join("mdown.update.bat").to_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(
                    MdownError::ConversionError(
                        String::from("Temp directory path could not be converted to string"),
                        11618
                    )
                );
            }
        };
        match write(&script_path, batch_script) {
            Ok(_) => (),
            Err(err) => {
                return Err(MdownError::IoError(err, temp_exe, 11617));
            }
        }

        // Launch the script and exit
        Command::new("cmd")
            .args(["/c", &script_path])
            .spawn()
            .map_err(|err| MdownError::IoError(err, script_path, 11616))?;

        println!("Update successful! Quiting...");
        Ok(true)
    } else {
        println!("Already up to date!");
        Ok(false)
    }
}

fn search_url<'a>(data: &'a serde_json::Value, target_file: &str) -> Result<&'a str, MdownError> {
    let items = data["assets"]
        .as_array()
        .ok_or_else(|| {
            MdownError::ConversionError(String::from("Expected 'assets' to be an array"), 11615)
        })?;

    for item in items {
        if let Some(url) = item["browser_download_url"].as_str() {
            if url.ends_with(target_file) {
                return Ok(url);
            }
        }
    }

    Err(MdownError::NotFoundError(String::from("No matching URL found"), 11620))
}

async fn version_preparation() -> Result<
    (Version, Version, serde_json::Value, reqwest::Client),
    MdownError
> {
    let current_version = match Version::parse(&get_current_version()) {
        Ok(version) => version,
        Err(_err) => version_new(),
    };
    debug!("Current version: {}", current_version);
    let repo = "GrenManSK/mdown";
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let client = match download::get_client() {
        Ok(client) => client,
        Err(err) => {
            return Err(MdownError::NetworkError(err, 11604));
        }
    };
    let response = match download::get_response_from_client(&url, &client).await {
        Ok(res) => res,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 11629));
        }
    };
    let data = match response.json::<serde_json::Value>().await {
        Ok(json) => json,
        Err(err) => {
            return Err(MdownError::JsonError(err.to_string(), 11605));
        }
    };

    let latest_version = match
        Version::parse(
            &(
                match data["tag_name"].as_str() {
                    Some(s) => s,
                    None => {
                        return Err(
                            MdownError::ConversionError(
                                String::from("Tag name could not be converted to string"),
                                11606
                            )
                        );
                    }
                }
            )[1..]
        )
    {
        Ok(version) => version,
        Err(_err) => {
            return Err(
                MdownError::ConversionError(String::from("Unable to parse latest version"), 11607)
            );
        }
    };
    Ok((current_version, latest_version, data, client))
}
pub(crate) async fn check_update() -> Result<bool, MdownError> {
    debug!("check_update");

    match db::get_update_time() {
        Ok(Some(time)) => {
            if let Ok(parsed_time) = NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S") {
                let current_time = Local::now().naive_local();
                let difference = current_time.signed_duration_since(parsed_time);
                if difference < chrono::Duration::days(1) {
                    debug!("No update needed (last check: {})\n", time);
                    return Ok(false);
                }
            }
        }
        _ => (),
    }

    let (current_version, latest_version, _, _) = match version_preparation().await {
        Ok(t) => t,
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 11630));
        }
    };

    let now = Local::now(); // Get the current local time
    let formatted_time = now.format("%Y-%m-%d %H:%M:%S").to_string();

    match db::set_update_time(&formatted_time) {
        Ok(()) => (),
        Err(err) => {
            return Err(MdownError::ChainedError(Box::new(err), 11631));
        }
    }

    if latest_version > current_version {
        debug!("New version available: {}", latest_version);
        let exe_path = match get_exe_file_path() {
            Ok(exe) => exe,
            Err(err) => {
                return Err(MdownError::ChainedError(Box::new(err), 11632));
            }
        };
        println!("Update of mdown is available");
        println!("mdown: {} => {}", current_version, latest_version);
        println!("Run {} app --update", exe_path);

        Ok(true)
    } else {
        debug!("Already up to date!\n");
        Ok(false)
    }
}

/// Removes the pre-release suffix from a version string.
///
/// This function takes a version string, splits it at the hyphen (`-`),
/// and returns the core version, excluding any pre-release identifiers.
/// For example, `1.2.3-beta` will be transformed to `1.2.3`.
///
/// # Arguments
/// * `version` - A version string that may contain a pre-release suffix (e.g., `1.2.3-beta`).
///
/// # Returns
/// * `String` - A string representing the core version without the pre-release suffix.
///
/// # Example
/// ```
/// let version = remove_prerelease("1.2.3-beta");
/// assert_eq!(version, "1.2.3");
/// ```
///
/// # Panics
/// * This function does not explicitly panic.
fn remove_prerelease(version: &str) -> String {
    let core_version: Vec<&str> = version.split('-').collect();
    core_version[0].to_string()
}

/// Returns the current package version without the pre-release suffix.
///
/// This function retrieves the current package version using the `CARGO_PKG_VERSION` environment variable
/// and removes any pre-release suffix (if present) using the `remove_prerelease` function.
///
/// # Returns
/// * `String` - A string representing the current version of the package without the pre-release suffix.
///
/// # Example
/// ```
/// let version = get_current_version();
/// println!("Current version: {}", version);
/// ```
///
/// # Panics
/// * This function does not explicitly panic.
#[inline]
pub(crate) fn get_current_version() -> String {
    remove_prerelease(env!("CARGO_PKG_VERSION"))
}
