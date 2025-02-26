
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

use memea::*;
use memea::config::Config;

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
    let reports: Vec<Reports> = configs.iter()
        .map(|c| tabulate::tabulate(c, &db))
        .collect();

    assert_eq!(configs.len(), reports.len());

    match args.area_only {
        true => {
            for i in 0 .. reports.len() {
                println!("{}\t{}", &configs[i].path, export::area(&reports[i]));
            }
        }
        false => {
            let names: Vec<String> = configs.iter()
                .map(|c| c.path.to_string())
                .collect();

            export::export(names, &reports, &args.export);
        }
    }

    Ok(())
}
