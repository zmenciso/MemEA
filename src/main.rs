use clap::Parser;
use std::path::PathBuf;
use std::error::Error;

mod config;
mod primitives;
mod tabulate;
mod export;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    config: PathBuf,

    #[arg(short, long, default_value = "./data/db.txt")]
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

    export::vprint("Reading configuration...", verbose);
    let config = config::read(&args.config)?;

    export::vprint("Building database...", verbose);
    let db = primitives::build_db(&args.db)?;

    export::vprint("Tabulating solution...", verbose);
    let report = tabulate::tabulate(config, &db).expect("Could not tabulate solution");

    match args.area_only {
        true => println!("{}", export::area(&report)),
        false => export::export(&args.config, &report, args.export)
    };

    Ok(())
}
