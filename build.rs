use flate2::read::GzDecoder;
use sha2::{Digest, Sha512};
use std::{env, fs, io::Read, path::PathBuf};
use tar::Archive;

const VERSION: &str = "2.1.1";
const URL: &str = "https://registry.npmjs.org/@phosphor-icons/core/-/core-2.1.1.tgz";
const SHA512: &str = "bf8011beb8a9e2a042226384e6b98f53294e10ae228840fd6722a372fcee7b3a8c6a24404821ca71122ebefc4bfedc2f2e991f9c438308e269e5d3387998a5c5";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let archive_path = out.join(format!("phosphor-core-{VERSION}.tgz"));
    let blob_path = out.join("phosphor.bin");
    let index_path = out.join("phosphor_index.rs");

    if !blob_path.exists() || !index_path.exists() {
        download_archive(&archive_path);
        unpack_assets(&archive_path, &blob_path, &index_path);
        let _ = fs::remove_file(archive_path);
    }
}

fn download_archive(path: &PathBuf) {
    println!("cargo:warning=Downloading Phosphor Icons {VERSION} for the build");
    let response = ureq::get(URL)
        .call()
        .unwrap_or_else(|error| panic!("could not download Phosphor Icons {VERSION}: {error}"));
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .expect("read Phosphor archive response");

    let actual = format!("{:x}", Sha512::digest(&bytes));
    assert_eq!(actual, SHA512, "Phosphor archive SHA-512 mismatch");
    fs::write(path, bytes).expect("cache Phosphor archive");
}

fn unpack_assets(archive_path: &PathBuf, blob_path: &PathBuf, index_path: &PathBuf) {
    let file = fs::File::open(archive_path).expect("open Phosphor archive");
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let mut files = Vec::new();
    for item in archive.entries().expect("read Phosphor archive entries") {
        let mut entry = item.expect("read Phosphor archive entry");
        let Ok(path) = entry.path() else { continue };
        let path = path.into_owned();
        let Some(relative) = path.strip_prefix("package/assets").ok() else { continue };
        if relative.components().count() != 2
            || !relative.extension().is_some_and(|ext| ext == "svg")
        {
            continue;
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents).expect("read Phosphor SVG");
        files.push((relative.to_path_buf(), contents));
    }
    files.sort_by(|(a, _), (b, _)| a.cmp(b));

    let mut data = Vec::new();
    let mut index = String::from("&[\n");
    for (path, contents) in files {
        let weight = path
            .parent()
            .and_then(|parent| parent.file_name())
            .expect("Phosphor weight directory")
            .to_string_lossy();
        let stem = path.file_stem().expect("Phosphor icon name").to_string_lossy();
        let suffix = format!("-{weight}");
        let name = stem.strip_suffix(&suffix).unwrap_or(&stem);
        let key = format!("{weight}/{name}");
        let offset = data.len();
        let len = contents.len();
        data.extend_from_slice(&contents);
        index.push_str(&format!("    ({key:?}, {offset}, {len}),\n"));
    }
    index.push_str("]");
    fs::write(blob_path, data).expect("write packed Phosphor icons");
    fs::write(index_path, index).expect("write Phosphor icon index");
}
