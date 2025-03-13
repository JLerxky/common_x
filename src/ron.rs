use std::{fs, io::Write, path::Path};

use ron::{
    de::from_reader,
    ser::{PrettyConfig, to_string_pretty},
};
use serde::de::DeserializeOwned;

pub fn read_ron<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    let f = fs::File::open(&path).expect("Failed opening file");

    from_reader(f)
        .map_err(|e| println!("ron file deserialize err: {e}"))
        .unwrap()
}

pub fn write_ron<T: serde::Serialize>(content: T, path: impl AsRef<Path>) {
    let pretty = PrettyConfig::new()
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let s = to_string_pretty(&content, pretty).expect("Serialization failed");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.as_ref())
        .unwrap_or_else(|_| panic!("open file({:?}) failed.", path.as_ref().to_str()));
    file.write_all(s.as_bytes()).unwrap();
    file.write_all(b"\n").unwrap();
}
