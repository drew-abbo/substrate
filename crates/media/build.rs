use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    match target_os.as_str() {
        "windows" => copy_dylibs_to_exe_dir("bin\\x64", "dll"),
        "macos" => copy_dylibs_to_exe_dir("lib", "dylib"),
        "linux" => {}
        _ => panic!("Unsupported target OS `{target_os}`."),
    }
}

fn copy_dylibs_to_exe_dir(dylib_dir_sub_path: impl AsRef<Path>, ext: &str) {
    // This is needed because we need to copy our FFmpeg DLLs/dylibs from the
    // ffmpeg folder into the target directory so executable can link to them at
    // runtime.

    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    let ffmpeg_dir = env::var("FFMPEG_DIR")
        .expect("`FFMPEG_DIR` environment variable unset. Please run `build_setup.py`.");

    let dylib_dir = Path::new(&ffmpeg_dir).join(dylib_dir_sub_path.as_ref());

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir.ancestors().nth(3).unwrap();

    for entry in fs::read_dir(&dylib_dir).unwrap() {
        let entry_path = entry.unwrap().path();

        if entry_path.extension().and_then(|s| s.to_str()) == Some(ext) {
            let dylib_file_name = entry_path.file_name().unwrap();

            let dest = target_dir.join(dylib_file_name);
            fs::copy(&entry_path, &dest).unwrap();

            let dest_deps = target_dir.join("deps").join(dylib_file_name);
            fs::copy(&entry_path, &dest_deps).unwrap();
        }
    }
}
