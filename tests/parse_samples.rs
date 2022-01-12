use std::fs;
use std::path::PathBuf;

use pretty_assertions::assert_eq;

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
        let patches = Patch::from_multiple(&data)
            .unwrap_or_else(|err| panic!("failed to parse {:?}, error: {}", path, err));

        // Make sure that the patch file we produce parses to the same information as the original
        // patch file.
        let patch_file: String = patches.iter().map(|patch| format!("{}\n", patch)).collect();
        println!("{}", patch_file);
        let patches2 = Patch::from_multiple(&patch_file).unwrap_or_else(|err| {
            panic!(
                "failed to re-parse {:?} after formatting, error: {}",
                path, err
            )
        });
        assert_eq!(patches, patches2);
    }
}
