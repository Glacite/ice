mod funcs;
use crate::funcs::*;

use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    env,
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
    Install {
        names: Vec<String>,

        #[arg(long = "yes")]
        yes: bool,
    },
    Remove {
        name: String,
    },
    Search {
        name: String,
    },
}

enum Ask {
    Yes,
    No,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.select {
        Select::Install { names, yes } => match check() {
            RunningAs::Root => {
                github_check();

                let mut found: Vec<String> = Vec::new();
                for name in names {
                    if search(&name).await {
                        if is_installed(&name) {
                            already_installed(&name);
                            std::process::exit(1);
                        } else {
                            found.push(name);
                        }
                    } else {
                        package_not_found(&name);
                        std::process::exit(1);
                    }
                }

                println!("{}", format!("Found packages ({}):", found.len()).bold());
                for name in &found {
                    println!("{name}");
                }
                println!();

                let ask = if yes {
                    Some(Ask::Yes)
                } else {
                    print!("{}", "Do you want to install it? ".bold());
                    io::stdout().flush().unwrap();
                    ask(Ask::Yes)
                };

                match ask {
                    Some(a) => match a {
                        Ask::Yes => {
                            for name in found {
                                let bar = ProgressBar::new_spinner();
                                bar.set_style(ProgressStyle::default_spinner().tick_strings(LOAD));
                                bar.enable_steady_tick(Duration::from_millis(100));
                                bar.set_message(format!("| Installing {}...", name.green()));

                                install(&name).await;

                                bar.set_message(format!("| Installed {}", name.green()));
                                bar.finish();
                            }
                        }
                        Ask::No => (),
                    },
                    None => unknown_action(),
                }
            }
            _ => root_required(),
        },
        Select::Remove { name } => match check() {
            RunningAs::Root => match is_installed(&name) {
                true => {
                    package_found(&name);
                    print!("{}", "Do you want to remove it? ".bold());
                    io::stdout().flush().unwrap();
                    match ask(Ask::Yes) {
                        Some(a) => match a {
                            Ask::Yes => {
                                remove(&name);
                                removed_pkg(&name);
                            }
                            _ => {}
                        },
                        None => unknown_action(),
                    }
                }
                false => pkg_not_installed(&name),
            },
            _ => root_required(),
        },
        Select::Search { name } => {
            github_check();
            match search(&name).await {
                true => package_found(&name),
                false => package_not_found(&name),
            }
        }
    }
}

fn package_not_found(pkg: impl Into<String>) {
    println!("Package {} not found", pkg.into().red());
}

fn package_found(pkg: impl Into<String>) {
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
    println!("Package {} is not installed", pkg.red());
}

fn removed_pkg(pkg: impl Into<String>) {
    let pkg = pkg.into();
    println!("Removed {} package", pkg.green());
}

fn already_installed(pkg: impl Into<String>) {
    let pkg = pkg.into();
    println!("Package {} is already installed", pkg.red());
}

fn unknown_action() {
    println!("{}", "Unknown action".red());
}

fn github_check() {
    if !github_ping() {
        println!(
            "{}",
            "Can't reach GitHub servers\nCheck your internet connection".red()
        );
        std::process::exit(1);
    }
}
