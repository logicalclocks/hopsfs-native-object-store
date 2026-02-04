use flate2::read::GzDecoder;
use log::info;
use std::env;
use std::fs::{self, File};
use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

const LIBHDFS_BASE_URL: &str = "https://repo.hops.works/master/libhdfs";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    download_and_extract_libhdfs()?;
    set_libraries();
    Ok(())
}

fn download_and_extract_libhdfs() -> Result<(), Box<dyn std::error::Error>> {
    let lib_dir = Path::new("lib");

    // Skip download if feature is enabled and lib directory already contains files
    if env::var("CARGO_FEATURE_SKIP_DOWNLOAD").is_ok() && lib_dir.exists() {
        let has_files = fs::read_dir(lib_dir)?.next().is_some();
        if has_files {
            info!("Skipping download: lib directory already exists with files");
            return Ok(());
        }
    }

    // Read version from HOPS_VERSION file
    let version = fs::read_to_string("HOPS_VERSION")
        .expect("Failed to read HOPS_VERSION file")
        .trim()
        .to_string();

    let tarball_url = format!("{}/libhdfs-golang-{}.tar.gz", LIBHDFS_BASE_URL, version);
    info!("Downloading libhdfs-golang from {}", tarball_url);

    // Download the tarball
    let client = reqwest::blocking::Client::new();
    let response = client.get(&tarball_url).send()?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download libhdfs-golang: HTTP {}",
            response.status()
        )
        .into());
    }

    // Create lib directory if it doesn't exist
    if lib_dir.exists() {
        fs::remove_dir_all(lib_dir)?;
    }
    fs::create_dir(lib_dir)?;

    // Extract tarball directly to lib directory
    let decoder = GzDecoder::new(response);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;

        // Skip directory entries and extract only files
        if !entry.header().entry_type().is_file() {
            continue;
        }

        let path = entry.path()?.to_path_buf();
        let file_name = path
            .file_name()
            .ok_or("Invalid file name in archive")?
            .to_str()
            .ok_or("Non-UTF8 file name")?
            .to_string();

        let dest_path = lib_dir.join(&file_name);
        let mut dest_file = File::create(&dest_path)?;
        io::copy(&mut entry, &mut dest_file)?;

        info!("Extracted: {}", file_name);
    }

    info!("Successfully extracted libhdfs-golang to {:?}", lib_dir);
    Ok(())
}

#[cfg(target_os = "macos")]
fn set_libraries() {
    create_symlinks("macos", false);
    println!("cargo:rustc-link-search=native=.");
    println!("cargo:rustc-link-lib=static=hdfs");
    println!("cargo:rustc-link-lib=framework=Security");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=resolv");
}

#[cfg(target_os = "linux")]
fn set_libraries() {
    create_symlinks("linux", true);
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", manifest_dir);
    println!("cargo:rustc-link-lib=hdfs");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn set_libraries() {
    panic!("Unsupported target OS: HopsFS object store only supports macOS and Linux.");
}

fn create_symlinks(target_os: &str, shared: bool) {
    let lib_dir = Path::new("lib");

    let filter = match target_os {
        "linux" => "linux-amd64",
        "macos" => "arm64",
        other => panic!("Unsupported target OS: {}", other),
    };

    let lib_ext = if shared {
        match target_os {
            "linux" => ".so",
            "macos" => ".dylib",
            other => panic!("Unsupported target OS: {}", other),
        }
    } else {
        ".a"
    };

    let mut lib_file = None;
    let mut header_file = None;

    for entry in fs::read_dir(lib_dir).expect("Could not read lib directory") {
        let entry = entry.expect("Error reading directory entry");
        let file_name = entry
            .file_name()
            .into_string()
            .expect("Invalid file name");

        if file_name.ends_with(lib_ext) && file_name.contains(filter) {
            lib_file = Some(entry.path());
        } else if file_name.ends_with(".h") && file_name.contains(filter) {
            header_file = Some(entry.path());
        }
    }

    let lib_file = lib_file.expect("Library file not found");
    let header_file = header_file.expect("Header file not found");

    let symlink_lib_str = format!("libhdfs{}", lib_ext);
    let symlink_lib = Path::new(&symlink_lib_str);
    let symlink_header = Path::new("libhdfs.h");

    // Remove existing symlinks (use symlink_metadata to detect broken symlinks too)
    if symlink_lib.symlink_metadata().is_ok() {
        fs::remove_file(symlink_lib).expect("Failed to remove existing library symlink");
    }
    if symlink_header.symlink_metadata().is_ok() {
        fs::remove_file(symlink_header).expect("Failed to remove existing header symlink");
    }

    symlink(&lib_file, symlink_lib).expect("Failed to create symlink for library");
    symlink(&header_file, symlink_header).expect("Failed to create symlink for header");

    info!(
        "Created symlinks: {} -> {:?}, libhdfs.h -> {:?}",
        symlink_lib_str, lib_file, header_file
    );
}
