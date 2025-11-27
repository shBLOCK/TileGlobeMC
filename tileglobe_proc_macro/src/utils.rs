use itertools::Itertools;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use tileglobe_utils::resloc::ResLoc;
use walkdir::WalkDir;

pub const SERVER_DATA_ROOT: &str = "../neoforge/serverData/tileglobemc/";

pub fn read_json(path: impl AsRef<Path>) -> serde_json::Value {
    let path = Path::new(SERVER_DATA_ROOT).join(path.as_ref());
    // panic!("{:?}", path);
    serde_json::from_reader(File::open(path).unwrap()).unwrap()
}

pub fn resloc_path(
    root: impl AsRef<Path>,
    resloc: &ResLoc,
    extension: impl AsRef<OsStr>,
) -> PathBuf {
    let mut buf = root.as_ref().to_path_buf();
    buf.push(PathBuf::from(resloc));
    buf.add_extension(extension);
    buf
}

pub fn list_resloc_files_in_dir(
    root: impl AsRef<Path>,
) -> impl Iterator<Item = (ResLoc<'static>, PathBuf)> {
    let root_path = Path::new(SERVER_DATA_ROOT).join(root);
    WalkDir::new(&root_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(move |e| {
            let file = e.path().strip_prefix(&root_path).unwrap();
            let file_no_extension = file.with_extension("");
            let mut iter = file_no_extension.iter();
            (
                ResLoc::new_owned(
                    iter.next().unwrap().to_str().unwrap().to_owned(),
                    iter.map(|s| s.to_str().unwrap()).join("/"),
                ),
                file.to_path_buf(),
            )
        })
}
