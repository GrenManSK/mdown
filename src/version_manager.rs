use semver::{ BuildMetadata, Prerelease, Version, VersionReq };
use std::{ fs::File, io::Write };

use crate::{ error::MdownError, getter::get_dat_path, metadata::Dat };

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
            version = match Version::parse(&get_current_version()) {
                Ok(version) => version,
                Err(_err) => {
                    Version {
                        major: 0,
                        minor: 0,
                        patch: 0,
                        pre: Prerelease::EMPTY,
                        build: BuildMetadata::EMPTY,
                    }
                }
            };

            let dat_path = match get_dat_path() {
                Ok(path) => path,
                Err(err) => {
                    return Err(err);
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
        } else {
            break;
        }
    }
    Ok(require_confirmation_from_user)
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
pub(crate) fn get_current_version() -> String {
    remove_prerelease(env!("CARGO_PKG_VERSION"))
}
