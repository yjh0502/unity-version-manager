#[cfg(unix)]
macro_rules! lock_process {
    ($lock_path:expr) => {
        let lock_file = fs::File::create($lock_path)?;
        let _lock = uvm_core::utils::lock_process_or_wait(&lock_file)?;
    };
}

#[cfg(windows)]
macro_rules! lock_process {
    ($lock_path:expr) => {};
}

use log::*;
use std::fs::DirBuilder;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

pub mod error;
pub mod installer;
mod loader;

mod sys;

use self::installer::*;
use error::*;

pub use self::loader::Loader;
pub use self::sys::*;

pub struct UnityModule;
pub struct UnityEditor;

pub trait InstallHandler {
    fn install_handler(&self) -> Result<()>;

    fn install(&self) -> Result<()> {
        self.before_install()
            .chain_err(|| "pre install step failed")?;
        self.install_handler()
            .map_err(|err| {
                self.error_handler();
                err
            })
            .chain_err(|| "installation failed")?;
        self.after_install()
            .chain_err(|| "post install step failed")
    }

    fn error_handler(&self) {}

    fn before_install(&self) -> Result<()> {
        Ok(())
    }

    fn after_install(&self) -> Result<()> {
        Ok(())
    }
}

pub fn handle_notfound(program: &str, e: std::io::Error) -> Error {
    if let std::io::ErrorKind::NotFound = e.kind() {
        error!("Error: '{0}' command not found.\nuvm requires '{0}' to install modules. Please ensure '{0}' is installed and accessible in your PATH.", program);
    }
    e.into()
}

fn path_contains_subpath<P: AsRef<Path>, Q: AsRef<Path>>(path: P, subpath: Q) -> bool {
    let path = path.as_ref();
    let subpath = subpath.as_ref();

    path.components()
        .collect::<Vec<_>>()
        .windows(subpath.components().count())
        .any(|window| window == subpath.components().collect::<Vec<_>>())
}

fn path_to_editor_root<'a>(path: &'a Path) -> &'a Path {
    let is_editor = path.file_name().unwrap() == "Editor";
    if is_editor {
        path.parent().unwrap()
    } else {
        path_to_editor_root(path.parent().unwrap())
    }
}

