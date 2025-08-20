use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;

use memea::config::Config;
use memea::*;

const DEFAULT_DB: &str = "./data/db.txt";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(required = true)]
    input: Vec<PathBuf>,

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

    #[arg(long)]
    gds: bool,

    #[arg(long)]
    gds_layer: Option<i16>,

    #[arg(short, long)]
    interactive: bool,
}

fn main() -> Result<(), MemeaError> {
    let args = Args::parse();
    let verbose = !args.quiet && !args.area_only;

    if args.gds {
        println!("{LOGO}");
        println!("{}", bar(Some("Interactive GDS Import"), '#'));

        gds::read(&args.input[0].to_string_lossy(), args.gds_layer)?;

        return Ok(());
    }

    if args.interactive {
        // TODO: Interactive
    }

    let start = Instant::now();
    let db = db::build_db(&args.db)?;

    vprintln!(verbose, "Built database in {:?}", start.elapsed());
    let start = Instant::now();

    let mut configs: Vec<Config> = Vec::new();
    for c in args.input {
        match config::read(&c) {
            Ok(r) => configs.push(r),
            Err(e) => errorln!("Failed to read config {:?} ({})", &c, e),
        }
    }

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

    let mut reports: Vec<Reports> = Vec::new();
    for c in &configs {
        match tabulate::tabulate(c, &db, scale) {
            Ok(r) => reports.push(r),
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
            for i in 0..reports.len() {
                println!("{}\t{}", &configs[i].path, export::area(&reports[i]));
            }
        }
        false => {
            let names: Vec<String> = configs.iter().map(|c| c.path.to_string()).collect();

            export::export(names, &reports, &args.export)?;
        }
    }

    Ok(())
}
