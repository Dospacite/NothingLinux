use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
use thiserror::Error;

const APP_ID: &str = "io.github.nothinglinux.nothinglinux";

#[derive(Debug, Error)]
enum Error {
    #[error("HOME is not set")]
    NoHome,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("release build failed")]
    BuildFailed,
    #[error("usage: cargo run -p xtask -- <install-user|uninstall-user>")]
    Usage,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Error> {
    let action = env::args().nth(1).ok_or(Error::Usage)?;
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or(Error::Usage)?;
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(Error::NoHome)?;
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let bin_home = env::var_os("XDG_BIN_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/bin"));
    let files = [
        (
            workspace.join("target/release/nothing-linux"),
            bin_home.join("nothing-linux"),
        ),
        (
            workspace.join(format!("data/{APP_ID}.desktop")),
            data_home.join(format!("applications/{APP_ID}.desktop")),
        ),
        (
            workspace.join(format!("data/{APP_ID}.metainfo.xml")),
            data_home.join(format!("metainfo/{APP_ID}.metainfo.xml")),
        ),
        (
            workspace.join(format!("data/icons/{APP_ID}.svg")),
            data_home.join(format!("icons/hicolor/scalable/apps/{APP_ID}.svg")),
        ),
    ];
    match action.as_str() {
        "install-user" => {
            let status = Command::new("cargo")
                .args(["build", "--release", "-p", "nothing-linux"])
                .current_dir(workspace)
                .status()?;
            if !status.success() {
                return Err(Error::BuildFailed);
            }
            for (source, destination) in files {
                install(&source, &destination)?;
            }
            println!(
                "Installed Nothing Linux for the current user. Ensure {} is on PATH.",
                bin_home.display()
            );
        }
        "uninstall-user" => {
            for (_, destination) in files {
                if destination.exists() {
                    fs::remove_file(destination)?;
                }
            }
            println!("Removed Nothing Linux application files. User configuration was kept.");
        }
        _ => return Err(Error::Usage),
    }
    Ok(())
}

fn install(source: &Path, destination: &Path) -> Result<(), Error> {
    let parent = destination.parent().ok_or(Error::Usage)?;
    fs::create_dir_all(parent)?;
    let temporary = destination.with_extension("new");
    fs::copy(source, &temporary)?;
    fs::rename(temporary, destination)?;
    Ok(())
}
