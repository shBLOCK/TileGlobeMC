use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use tileglobe_utils::resloc::ResLoc;

pub fn read_json(path: impl AsRef<Path>) -> serde_json::Value {
    let path = Path::new("../neoforge/serverData/tileglobemc/").join(path.as_ref());
    // panic!("{:?}", path);
    serde_json::from_reader(File::open(path).unwrap()).unwrap()
}

pub fn resloc_path(prefix: impl AsRef<Path>, resloc: ResLoc, extension: impl AsRef<OsStr>) -> PathBuf {
    let mut buf = prefix.as_ref().to_path_buf();
    buf.push(PathBuf::from(resloc));
    buf.add_extension(extension);
    buf
}