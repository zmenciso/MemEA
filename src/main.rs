use clap::Parser;
use std::path::PathBuf;

use memea::config::Config;
use memea::*;

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

    #[arg(long, value_names = ["FROM", "TO"], num_args = 2)]
    autoscale: Option<Vec<usize>>,

    #[arg(long)]
    scale: Option<Float>,
}

fn main() -> Result<(), MemeaError> {
    let args = Args::parse();
    let verbose = !args.quiet && !args.area_only;

    if args.config.is_empty() {
        eprintln!("No configuration files specified; aborting...");
        std::process::exit(255);
    }

    if verbose {
        infoln!("Building database...");
    }
    let db = primitives::build_db(&args.db)?;

    if verbose {
        infoln!("Reading configuration files...");
    }
    let configs: Vec<Config> = args
        .config
        .iter()
        .map(|p| config::read(p))
        .filter_map(Result::ok)
        .collect();

    let scale: Float = match args.scale {
        Some(val) => val,
        None => match args.autoscale {
            Some(vals) => {
                let (from, to) = (vals[0], vals[1]);
                scale(from, to)
            }
            _ => 1.0,
        },
    };

    if verbose {
        infoln!("Building solution...");
    }
    let reports: Vec<Reports> = configs
        .iter()
        .map(|c| tabulate::tabulate(c, &db, scale))
        .filter_map(Result::ok)
        .collect();

    assert_eq!(configs.len(), reports.len());

    match args.area_only {
        true => {
            for i in 0..reports.len() {
                infoln!("{}\t{}", &configs[i].path, export::area(&reports[i]));
            }
        }
        false => {
            let names: Vec<String> = configs.iter().map(|c| c.path.to_string()).collect();

            export::export(names, &reports, &args.export)?;
        }
    }

    Ok(())
}
