mod utils;

use fs_extra::dir::CopyOptions;
use std::{fs, path::PathBuf, time::SystemTime};
use utils::add_extension;

use clap::*;
use serde::*;
use similar::*;

#[derive(Serialize, Deserialize)]
struct Config {
    last_write: SystemTime,
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Switch { variant: String },
    Write { variant: String },
}

fn main() {
    let cli = Cli::parse();

    let dot_kloop = PathBuf::from("./.kloop");
    let kloop_preview = PathBuf::from("./kloop-preview");

    let config_path = dot_kloop.join("config.json");

    let config: Config =
        serde_json::from_slice(&fs::read(&config_path).expect("Failed to read config"))
            .expect("Failed to deserialize config");

    fs::create_dir_all(&kloop_preview).ok();

    match cli.command {
        Command::Write { variant } => {
            fs::create_dir_all(&dot_kloop.join(&variant)).expect("Failed to create variant folder");

            for preview in ignore::WalkBuilder::new("kloop-preview")
                .add_custom_ignore_filename("../.kloopignore")
                .build()
                .flatten()
            {
                let meta = preview.metadata().expect("Failed to read metadata");

                let normalised = preview.path().strip_prefix("kloop-preview/").unwrap();

                if meta.is_file() {
                    let base = dot_kloop.join("base").join(&normalised);

                    let preview_contents =
                        fs::read_to_string(preview.path()).unwrap_or_else(|_| {
                            panic!("Failed to read path {}", preview.path().display())
                        });
                    let base_contents = fs::read_to_string(&base)
                        .unwrap_or_else(|_| panic!("Failed to read path {}", &base.display()));

                    let diff = TextDiff::from_lines(&base_contents, &preview_contents)
                        .unified_diff()
                        .to_string();

                    let mut patch = normalised.to_path_buf();

                    add_extension(&mut patch, "patch");

                    let dot_kloop_patch = dot_kloop.join(&variant).join(patch);

                    fs::create_dir_all(dot_kloop_patch.parent().unwrap())
                        .expect("Failed to create parent dirs");

                    fs::write(dot_kloop_patch, diff).expect("Failed to write to patch file");
                }
            }

            let config = Config {
                last_write: SystemTime::now(),
                ..config
            };

            fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap())
                .expect("Failed to write config");
        }
        Command::Switch { variant } => {
            let meta = kloop_preview.metadata().expect("Failed to get metadata");

            let last_write = meta.modified().expect("Failed to read last modified time");

            dbg!(&last_write, &config.last_write);

            if last_write > config.last_write && kloop_preview.read_dir().unwrap().next().is_some()
            {
                println!("Failed to switch as you have unwritten changes");
                return;
            }

            for preview in ignore::WalkBuilder::new(&kloop_preview)
                .add_custom_ignore_filename("../.kloopignore")
                .build()
                .flatten()
            {
                if preview.path() == kloop_preview {
                    continue;
                }

                dbg!(&preview);
                let meta = preview.metadata().expect("Failed to get preview metadata");

                if meta.is_dir() {
                    fs::remove_dir_all(preview.path()).ok();
                } else {
                    fs::remove_file(preview.path()).ok();
                }
            }

            fs_extra::dir::copy(dot_kloop.join("base"), &kloop_preview, &{
                let mut op = CopyOptions::new();
                op.overwrite = true;
                op.content_only = true;
                op
            })
            .expect("Failed to copy files");

            if variant == "base" {
                return;
            }

            let dot_kloop_variant = dot_kloop.join(&variant);

            for patch in ignore::WalkBuilder::new(&dot_kloop_variant)
                .build()
                .flatten()
            {
                let path = patch.path();

                if path == dot_kloop_variant {
                    continue;
                }

                if path.is_dir() {
                    fs::create_dir_all(path);
                } else {
                    let diff = fs::read_to_string(path).expect("Failed to read patch");
                }
            }
        }
    }
}
