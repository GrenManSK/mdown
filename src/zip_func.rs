use std::{ fs::File, io::{ Read, Write, Seek }, path::Path };
use walkdir::{ DirEntry, WalkDir };
use zip::{ result::ZipError, write::FileOptions };
use tracing::info;

use crate::{ string, utils::progress_bar_preparation, MAXPOINTS, ARGS };

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &str,
    writer: T,
    method: zip::CompressionMethod
) -> zip::result::ZipResult<()>
    where T: Write + Seek
{
    let walkdir = WalkDir::new(prefix);
    let it_temp = &mut walkdir.into_iter().filter_map(|e| e.ok());
    let dir_entries_vec: Vec<DirEntry> = it_temp.collect();
    let total_items = dir_entries_vec.len();
    let start = MAXPOINTS.max_x / 3 - ((total_items / 2) as i32);
    progress_bar_preparation(start, total_items, 8);
    let mut zip = zip::ZipWriter::new(writer);
    let options = FileOptions::default().compression_method(method).unix_permissions(0o755);

    let mut buffer = Vec::new();
    let mut times = -1;
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(prefix)).unwrap();
        if path.is_file() {
            string(8, start + times, "#");
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
        times += 1;
    }
    zip.finish()?;
    Result::Ok(())
}

fn doit(
    src_dir: &str,
    dst_file: &str,
    method: zip::CompressionMethod
) -> zip::result::ZipResult<()> {
    if !Path::new(src_dir).is_dir() {
        return Err(ZipError::FileNotFound);
    }
    let path = Path::new(dst_file);
    let file = File::create(path).unwrap();

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, file, method)?;

    Ok(())
}

const METHOD_STORED: Option<zip::CompressionMethod> = Some(zip::CompressionMethod::Stored);
pub(crate) async fn to_zip(src_dir: &str, dst_file: &str, handle_id: String) {
    if ARGS.web {
        info!("@{} Zipping files to: {} ...", handle_id, dst_file);
    }
    match doit(src_dir, dst_file, METHOD_STORED.unwrap()) {
        Ok(_) => string(9, 0, format!("   done: {} written to {}", src_dir, dst_file).as_str()),
        Err(e) => eprintln!("  Error: {e:?}"),
    }
    if ARGS.web {
        info!("@{} Zipping files to: {} Done", handle_id, dst_file);
    }
}
