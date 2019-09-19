use crate::lib::env::{BinaryCacheEnv, PlatformEnv, VersionEnv};
use crate::lib::error::{DfxError, DfxResult};
use crate::util::assets;
use clap::{App, Arg, ArgMatches, SubCommand};
use console::style;
use indicatif::{HumanBytes, ProgressBar, ProgressDrawTarget};
use std::io::Read;
use std::path::{Path, PathBuf};

const DRY_RUN: &str = "dry_run";
const PROJECT_NAME: &str = "project_name";

pub fn construct() -> App<'static, 'static> {
    SubCommand::with_name("new")
        .about("Create a new project.")
        .arg(
            Arg::with_name(PROJECT_NAME)
                .help("The name of the project to create.")
                .required(true),
        )
        .arg(
            Arg::with_name(DRY_RUN)
                .help("Do not write anything to the file system.")
                .long("dry-run")
                .takes_value(false),
        )
}

enum Status<'a> {
    Create(&'a Path, usize),
    CreateDir(&'a Path),
}

impl std::fmt::Display for Status<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Status::Create(path, size) => write!(
                f,
                "{:<12} {} ({})...",
                style("CREATE").green().bold(),
                path.to_str().unwrap_or("<unknown>"),
                HumanBytes(*size as u64),
            )?,
            Status::CreateDir(path) => write!(
                f,
                "{:<12} {}...",
                style("CREATE_DIR").blue().bold(),
                path.to_str().unwrap_or("<unknown>"),
            )?,
        };

        Ok(())
    }
}

pub fn create_file(path: &Path, content: &str, dry_run: bool) -> DfxResult {
    if !dry_run {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&path, content)?;
    }

    eprintln!("{}", Status::Create(path, content.len()));
    Ok(())
}

#[allow(dead_code)]
pub fn create_dir<P: AsRef<Path>>(path: P, dry_run: bool) -> DfxResult {
    let path = path.as_ref();
    if path.is_dir() {
        return Ok(());
    }

    if !dry_run {
        std::fs::create_dir_all(&path)?;
    }

    eprintln!("{}", Status::CreateDir(path));
    Ok(())
}

pub fn exec<T>(env: &T, args: &ArgMatches<'_>) -> DfxResult
where
    T: BinaryCacheEnv + PlatformEnv + VersionEnv,
{
    let dry_run = args.is_present(DRY_RUN);
    let project_name = Path::new(args.value_of(PROJECT_NAME).unwrap());

    if project_name.exists() {
        return Err(DfxError::ProjectExists());
    }

    let dfx_version = env.get_version();
    let b = ProgressBar::new_spinner();
    b.set_draw_target(ProgressDrawTarget::stderr());
    b.set_message("Looking for latest version...");
    b.enable_steady_tick(80);

    std::thread::sleep(std::time::Duration::from_secs(1));
    if !env.is_installed()? {
        env.install()?;
        b.finish_with_message(
            format!("Version v{} installed successfully.", env.get_version()).as_str(),
        );
    } else {
        b.finish_with_message(
            format!("Version v{} already installed.", env.get_version()).as_str(),
        );
    }

    eprintln!(r#"Creating new project "{}"..."#, project_name.display());
    if dry_run {
        eprintln!(r#"Running in dry mode. Nothing will be committed to disk."#);
    }

    let mut new_project_files = assets::new_project_files()?;
    for file in new_project_files.entries()? {
        let mut file = file?;

        if file.header().entry_type().is_dir() {
            continue;
        }

        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();

        // Perform replacements.
        let s = s.replace("{project_name}", project_name.to_str().unwrap());
        let s = s.replace("{dfx_version}", dfx_version);

        // Perform path replacements.
        let p = PathBuf::from(
            project_name
                .join(file.header().path()?)
                .to_str()
                .unwrap()
                .replace("__dot__", ".")
                .as_str(),
        );

        create_file(p.as_path(), s.as_str(), dry_run)?;
    }

    if !dry_run {
        // Check that git is available.
        let init_status = std::process::Command::new("git")
            .arg("init")
            .current_dir(&project_name)
            .status();
        if init_status.is_ok() && init_status.unwrap().success() {
            eprintln!("Creating git repository...");
            std::process::Command::new("git")
                .arg("add")
                .current_dir(&project_name)
                .arg(".")
                .output()?;
            std::process::Command::new("git")
                .arg("commit")
                .current_dir(&project_name)
                .arg("-a")
                .arg("--message=Initial commit.")
                .output()?;
        }
    }

    // Print welcome message.
    eprintln!(
        // This needs to be included here because we cannot use the result of a function for
        // the format!() rule (and so it cannot be moved in the util::assets module).
        include_str!("../../assets/welcome.txt"),
        dfx_version,
        assets::color_logo(),
        project_name.to_str().unwrap()
    );

    Ok(())
}
