use clap::Parser;
use std::{collections::HashMap, path::PathBuf, time::Instant};

use memea::*;

const DEFAULT_DB: &str = "./data/db.yaml";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, name = "MemEA", about = "Layout-informed memory macro area estimator")]
pub struct Args {
    #[arg(help = "Path(s) to configuration file(s)")]
    input: Vec<PathBuf>,

    #[arg(short, long, default_value = DEFAULT_DB, help = "Path to the database file")]
    db: PathBuf,

    #[arg(
        short,
        long,
        help = "Export results to file in CSV/JSON/YAML format (chosen from extension)"
    )]
    export: Option<PathBuf>,

    #[arg(
        short,
        long,
        help = "Do not print breakdown; only print total area for each configuration (automatically toggles `-q`)"
    )]
    area_only: bool,

    #[arg(short, long, help = "Suppress nonessential messages")]
    quiet: bool,

    #[arg(long, value_names = ["FROM", "TO"], num_args = 2, help = "Use built-in transistor scaling data to scale area from source technology node (e.g. 65) to target technology node (e.g. 22)")]
    autoscale: Option<Vec<usize>>,

    #[arg(
        long,
        help = "Manually specify a scaling value to scale area (e.g. 0.124)"
    )]
    scale: Option<Float>,

    #[arg(
        short,
        long,
        help = "Interactively build a database file from GDS and LEF data"
    )]
    build_db: bool,

    #[arg(long, help = "Launch GUI")]
    gui: bool,
}

fn main() -> Result<(), MemeaError> {
    let args = Args::parse();
    let verbose = !args.quiet && !args.area_only;

    if args.build_db {
        println!("{LOGO}");
        println!("{}\n", bar(Some("Interactive Database Builder"), '#'));
        lef::lefin(verbose)?;
        return Ok(());
    } else if args.input.is_empty() {
        errorln!("No configuration files provided, aborting...");
        return Ok(());
    }

    if args.gui {
        // TODO: GUI
        errorln!("GUI not yet implemented, falling back to CLI");
    }

    let start = Instant::now();
    let db = db::build_db(&args.db)?;

    vprintln!(verbose, "Built database in {:?}", start.elapsed());
    let start = Instant::now();

    let configs = config::read_all(&args.input);

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

    vprintln!(
        verbose,
        "Read {} configuration file(s) in {:?}",
        configs.len(),
        start.elapsed()
    );
    let start = Instant::now();

    let mut reports: HashMap<String, tabulate::Reports> = HashMap::new();
    for (name, c) in &configs {
        match tabulate::tabulate(name, c, &db, scale) {
            Ok(r) => {
                reports.insert(name.clone(), r);
            }
            Err(e) => errorln!("Failed to tabulate config: {}", e),
        }
    }

    if configs.len() != reports.len() {
        warnln!(
            "Number of reports ({}) does not match number of configs ({})",
            reports.len(),
            configs.len()
        );
    }

    vprintln!(
        verbose,
        "Built {}/{} solution(s) in {:?}",
        reports.len(),
        configs.len(),
        start.elapsed()
    );

    match args.area_only {
        true => {
            for (name, r) in &reports {
                println!("{}\t{}", name, export::area(r));
            }
        }
        false => {
            export::export(&reports, &args.export)?;
        }
    }

    Ok(())
}
