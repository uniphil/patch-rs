use std::fs;
use std::path::PathBuf;

use patch::Patch;

#[test]
fn parse_samples() {
    let samples_path = PathBuf::from(file!()).parent().unwrap().join("samples");
    for file in fs::read_dir(samples_path).unwrap() {
        let path = file.unwrap().path();
        if path.extension().unwrap_or_default() != "diff" {
            continue;
        }

        let data = fs::read_to_string(dbg!(&path)).unwrap();
        Patch::from_multiple(&data).unwrap_or_else(
            |err| panic!("failed to parse {:?}, error: {}", path, err)
        );
    }
}
