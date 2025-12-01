use itertools::Itertools;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use syn::parse::ParseBuffer;
use tileglobe_utils::resloc::ResLoc;
use walkdir::WalkDir;

pub const SERVER_DATA_ROOT: &str = "../neoforge/serverData/tileglobemc/";

pub fn read_json(path: impl AsRef<Path>) -> Result<serde_json::Value, Box<dyn Error>> {
    let path = Path::new(SERVER_DATA_ROOT).join(path.as_ref());
    Ok(serde_json::from_reader(File::open(path)?)?)
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

pub trait ParseIdent {
    fn parse_ident(&self, expected: &str) -> syn::Result<syn::Ident>;
}
impl ParseIdent for ParseBuffer<'_> {
    fn parse_ident(&self, expected: &str) -> syn::Result<syn::Ident> {
        let ident = self.parse::<syn::Ident>()?;
        if ident.to_string() == expected {
            Ok(ident)
        } else {
            Err(self.error(format_args!(
                "Expected identifier \"{}\", got \"{}\"",
                expected, ident
            )))
        }
    }
}
