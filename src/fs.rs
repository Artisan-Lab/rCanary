use crate::log::rlc_error_and_exit;

use std::fs;
use std::path::Path;

pub fn rlc_create_dir<P: AsRef<Path>>(path: P, msg: impl AsRef<str>) {
    if fs::read_dir(&path).is_err() {
        fs::create_dir(path)
            .unwrap_or_else(|e|
                rlc_error_and_exit(format!("{}: {}", msg.as_ref(), e))
            );
    }
}

pub fn rlc_remove_dir<P: AsRef<Path>>(path: P, msg: impl AsRef<str>) {
    if fs::read_dir(&path).is_ok() {
        fs::remove_dir_all(path)
            .unwrap_or_else(|e|
                rlc_error_and_exit(format!("{}: {}", msg.as_ref(), e))
            );
    }
}

pub fn rlc_can_read_dir<P: AsRef<Path>>(path: P, msg: impl AsRef<str>) -> bool {
    match fs::read_dir(path) {
        Ok(_) => true,
        Err(e) => rlc_error_and_exit(format!("{}: {}", msg.as_ref(), e)),
    }
}

pub fn rlc_copy_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q, msg: impl AsRef<str>) {
    fs::copy(from, to)
        .unwrap_or_else(|e|
            rlc_error_and_exit(format!("{}: {}", msg.as_ref(), e))
        );
}

pub fn rlc_read<P: AsRef<Path>>(path: P, msg: impl AsRef<str>) -> fs::File {
    match fs::File::open(path) {
        Ok(file) => file,
        Err(e) => rlc_error_and_exit(format!("{}: {}", msg.as_ref(), e)),
    }
}