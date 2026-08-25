mod funcs;
use crate::funcs::*;

use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    env, fs,
    io::{self, Write},
    time::Duration,
};
use sudo::{RunningAs, check};
use tokio;

static LOAD: &[&str; 13] = &[
    "⠀⠙", "⠀⠸", "⠀⢰", "⠀⣠", "⢀⣀", "⣀⡀", "⣄⠀", "⡆⠀", "⠇⠀", "⠋⠀", "⠉⠁", "⠈⠉", "✓",
];

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    select: Select,
}

#[derive(Subcommand, Debug)]
enum Select {
    Install { name: String },
    Remove { name: String },
    Search { name: String },
}

enum Ask {
    Yes,
    No,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.select {
        Select::Install { name } => match check() {
            RunningAs::Root => match search(&name).await {
                true => {
                    package_found(&name);
                    match ask(Ask::Yes) {
                        Some(a) => match a {
                            Ask::Yes => {
                                let bar = ProgressBar::new_spinner();
                                bar.set_style(ProgressStyle::default_spinner().tick_strings(LOAD));
                                bar.enable_steady_tick(Duration::from_millis(100));
                                bar.set_message("| Installing...");

                                install(&name).await;

                                bar.finish();
                            }
                            Ask::No => (),
                        },
                        None => println!("{}", "Unknown action".red()),
                    }
                }
                false => package_not_found(&name),
            },
            _ => root_required(),
        },
        Select::Remove { name } => match check() {
            RunningAs::Root => {
                let path = "/lib/ice/packages/";

                match fs::read_dir(&path)
                    .unwrap()
                    .find(|f| f.as_ref().unwrap().file_name().to_str().unwrap() == &name)
                {
                    Some(_) => {
                        remove(&name);
                        fs::remove_dir_all(&path).unwrap();
                    }
                    None => pkg_not_installed(&name),
                };
            }
            _ => root_required(),
        },
        Select::Search { name } => match search(&name).await {
            true => package_found(&name),
            false => package_not_found(&name),
        },
    }
}

fn package_not_found<S: Into<String>>(pkg: S) {
    println!("Package {} not found", pkg.into().red());
}

fn package_found<S: Into<String>>(pkg: S) {
    println!("Found package {}", pkg.into().green());
}

fn ask(default: Ask) -> Option<Ask> {
    print!("{}: ", ask_message(&default));
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    let action = buf.trim().to_lowercase();

    match action.as_str() {
        "y" => Some(Ask::Yes),
        "n" => Some(Ask::No),
        _ => {
            if action.is_empty() {
                Some(default)
            } else {
                None
            }
        }
    }
}

fn ask_message(default: &Ask) -> String {
    match default {
        Ask::Yes => "[Y/n]".into(),
        Ask::No => "[y/N]".into(),
    }
}

fn root_required() {
    let mut command = env::args().collect::<Vec<_>>().join(" ");
    command = format!("{} {command}", "sudo".green());

    println!("{}", "Root privileges required".red());
    println!("{} {}", "Try:".bold(), command)
}

fn pkg_not_installed(pkg: impl Into<String>) {
    let pkg = pkg.into();
    println!("Package {} is not installed", &pkg.red());
}
