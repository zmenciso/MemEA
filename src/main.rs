
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

use mem_ea::*;
use mem_ea::config::Config;

const DEFAULT_DB: &str = "./data/db.txt";

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

    if args.config.len() == 0 {
        eprintln!("No configuration files specified; aborting...");
        std::process::exit(255);
    }

    if verbose { println!("Building database..."); }
    let db = primitives::build_db(&args.db)?;

    if verbose { println!("Reading configuration files..."); }
    let configs: Vec<Config> = args.config.iter()
        .map(|p| config::read(p).expect("Could not read configuration file"))
        .collect();

    if verbose { println!("Building solution..."); }
    let reports: Vec<Report> = configs.iter()
        .map(|c| tabulate::tabulate(c, &db))
        .collect();

    assert_eq!(configs.len(), reports.len());

    for i in 0 .. reports.len() {
        match args.area_only {
            true => { println!("{}\t{}", &configs[i].path, export::area(&reports[i])); }
            false => { export::export(&configs[i].path, &reports[i], &args.export); }
        }
    }

    Ok(())
}
