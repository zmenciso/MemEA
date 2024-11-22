use clap::Parser;
use std::path::PathBuf;
use std::error::Error;
use std::collections::HashMap;

const DEFAULT_DB: &str = "./data/db.txt";

mod config;
mod primitives;
mod tabulate;
mod export;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    config: Vec<PathBuf>,

    #[arg(short, long, default_value = DEFAULT_DB)]
    db: PathBuf,

    #[arg(short, long)]
    export: Option<PathBuf>,

    #[arg(short, long)]
    area_only: bool,

    #[arg(short, long)]
    quiet: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let verbose = !args.quiet && !args.area_only;

    export::vprint("Reading configuration files...", verbose);
    let mut config: Vec<config::Config> = Vec::new();
    for path in args.config {
        let result = config::read(&path);
        match result {
            Ok(t) => config.push(t),
            Err(e) => {
                eprintln!("ERROR: Could not parse input {}", e);
                std::process::exit(5); 
            }
        }
    }

    export::vprint("Building database...", verbose);
    let db = primitives::build_db(&args.db)?;

    export::vprint("Tabulating solution...", verbose);
    let reports: Vec<HashMap<String, f32>> = config.
        iter().
        map(|x| tabulate::tabulate(x, &db)).
        collect();

    assert!(reports.len() == config.len());

    for i in 0..reports.len() {
        match args.area_only {
            true => println!("{} {}", config[i].path, export::area(&reports[i])),
            false => export::export(&config[i].path, &reports[i], &args.export)
        };
    }

    Ok(())
}
