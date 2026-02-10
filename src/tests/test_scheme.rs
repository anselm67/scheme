
use std::fs;
use std::path::PathBuf;

use crate::interp::Interp;

fn get_test_files() -> Vec<PathBuf> {
    let dir = "src/scheme";
    fs::read_dir(dir)
        .expect("Could not read directory")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let file_name = path.file_name()?.to_str()?;
            
            // Check if it starts with "test_" and ends with ".scm"
            if file_name.starts_with("test_") && file_name.ends_with(".scm") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn run_scheme_tests() {
    let interp = Interp::new();
    let test_files = get_test_files();

    for file in test_files {
        if let Err(e) = interp.load(&file) {
            panic!("Failed to run tests from {:?}: {:?}", &file, e)
        }
    }
}