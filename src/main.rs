use clap::{Parser, Subcommand};
use sudo::check;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    select: Select,
}

#[derive(Subcommand, Debug)]
enum Select {
    Install { name: String },
}

fn main() {
    let args = Args::parse();

    println!("{:?}", check());
    match args.select {
        Select::Install { name } => println!("Installing {}", name),
    }
}
