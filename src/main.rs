mod funcs;
use crate::funcs::*;

use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::ProgressBar;
use std::{
    env,
    io::{self, Write},
    time::Duration,
};
use sudo::{RunningAs, check};
use tokio;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    select: Select,
}

#[derive(Subcommand, Debug)]
enum Select {
    Install { name: String },
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
                                bar.enable_steady_tick(Duration::from_millis(100));
                                bar.set_prefix("Installing");

                                install(&name).await;

                                loop {}
                            }
                            Ask::No => (),
                        },
                        None => println!("{}", "Unknown action".red()),
                    }
                }
                false => package_not_found(&name),
            },
            _ => {
                let mut command = env::args().collect::<Vec<_>>().join(" ");
                command = format!("{} {command}", "sudo".green());

                println!("{}", "Root privileges required".red());
                println!("{} {}", "Try:".bold(), command)
            }
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
