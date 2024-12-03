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

    if verbose { println!("Reading configuration files..."); }
    let mut config: Vec<config::Config> = Vec::new();
    for path in args.config {
        let result = config::read(&path);
        match result {
            Ok(t) => config.push(t),
            Err(e) => {
                export::error(format!("Could not parse input {}", e));
                std::process::exit(5); 
            }
        }
    }

    if verbose { println!("Building database..."); }
    let db = primitives::build_db(&args.db)?;

    if verbose { println!("Computing solution...\n"); }
    let reports: Vec<HashMap<String, f32>> = config.
        iter().
        map(|x| tabulate::tabulate(x, &db)).
        collect();

    assert!(reports.len() == config.len());

    for i in 0..reports.len() {
        match args.area_only {
            true => println!("{}\t{}", config[i].path, export::area(&reports[i])),
            false => export::export(&config[i].path, &reports[i], &args.export)
        };
    }

    if verbose { println!("Cleaning up..."); }

    Ok(())
}
