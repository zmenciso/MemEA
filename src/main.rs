//! MemEA - Memory Peripheral Estimation and Analysis Tool
//!
//! This is the main command-line application for MemEA, providing layout-informed
//! memory macro area estimation. It processes configuration files, component databases,
//! and generates detailed area reports for memory peripherals.

use clap::Parser;
use std::{collections::HashMap, path::PathBuf, time::Instant};

use memea::*;

/// Default path to the component database file.
const DEFAULT_DB: &str = "./data/db.yaml";

/// Command-line arguments for the MemEA application.
///
/// This struct defines all command-line options and arguments using the clap derive API.
/// It supports various modes of operation including area estimation, database building,
/// and result export in multiple formats.
#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = None,
    name = "MemEA",
    about = "Layout-informed memory macro area estimator"
)]
pub struct Args {
    /// Path(s) to configuration file(s) containing memory specifications.
    #[arg(help = "Path(s) to configuration file(s)")]
    input: Vec<PathBuf>,

    /// Path to the component database file (YAML or JSON format).
    #[arg(
        short,
        long,
        default_value = DEFAULT_DB,
        help = "Path to the database file"
    )]
    db: PathBuf,

    /// Export results to file in CSV/JSON/YAML format (format chosen from extension).
    #[arg(
        short,
        long,
        help = "Export results to file in CSV/JSON/YAML format (chosen from extension)"
    )]
    export: Option<PathBuf>,

    /// Print only total area for each configuration without detailed breakdown.
    ///
    /// This automatically enables quiet mode to suppress verbose output.
    #[arg(
        short,
        long,
        help = "Do not print breakdown; only print total area for each configuration (automatically toggles `-q`)"
    )]
    area_only: bool,

    /// Suppress nonessential informational messages.
    #[arg(short, long, help = "Suppress nonessential messages")]
    quiet: bool,

    /// Scale area using built-in technology node data.
    ///
    /// Takes two arguments: source node (e.g., 65) and target node (e.g., 22).
    /// Uses predefined scaling factors for common semiconductor processes.
    #[arg(
        long,
        value_names = ["FROM", "TO"],
        num_args = 2,
        help = "Use built-in transistor scaling data to scale area from source technology node (e.g. 65) to target technology node (e.g. 22)"
    )]
    autoscale: Option<Vec<usize>>,

    /// Manually specify a scaling factor to apply to all area calculations.
    #[arg(
        long,
        help = "Manually specify a scaling value to scale area (e.g. 0.124)"
    )]
    scale: Option<Float>,

    /// Launch interactive database builder from GDS and LEF files.
    #[arg(
        short,
        long,
        help = "Interactively build a database file from GDS and LEF data"
    )]
    build_db: bool,

    /// Launch graphical user interface (not yet implemented).
    #[arg(long, help = "Launch GUI")]
    gui: bool,
}

/// Main entry point for the MemEA application.
///
/// This function orchestrates the complete workflow:
/// 1. Parse command-line arguments
/// 2. Handle special modes (database building, GUI)
/// 3. Load component database and configurations
/// 4. Process area estimations with optional scaling
/// 5. Export results in the requested format
///
/// # Returns
/// * `Ok(())` - Application completed successfully
/// * `Err(MemeaError)` - Error during processing (file I/O, parsing, etc.)
fn main() -> Result<(), MemeaError> {
    let args = Args::parse();
    let verbose = !args.quiet && !args.area_only;

    // Handle special operating modes first
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
        // TODO: GUI implementation
        errorln!("GUI not yet implemented, falling back to CLI");
    }

    // Load component database
    let start = Instant::now();
    let db = db::build_db(&args.db)?;
    vprintln!(verbose, "Built database in {:?}", start.elapsed());

    // Load configuration files
    let start = Instant::now();
    let configs = config::read_all(&args.input);

    // Determine scaling factor from command-line arguments
    let scale: Float = match args.scale {
        Some(val) => val,
        None => match args.autoscale {
            Some(vals) => {
                let (from, to) = (vals[0], vals[1]);
                scale(from, to)
            }
            None => 1.0,
        },
    };

    vprintln!(
        verbose,
        "Read {} configuration file(s) in {:?}",
        configs.len(),
        start.elapsed()
    );
    // Generate area estimation reports for each configuration
    let start = Instant::now();
    let mut reports: HashMap<String, tabulate::Reports> = HashMap::new();

    for (name, c) in &configs {
        match tabulate::tabulate(name, c, &db, scale) {
            Ok(r) => {
                reports.insert(name.clone(), r);
            }
            Err(e) => errorln!("Failed to tabulate config '{}': {}", name, e),
        }
    }

    // Warn if some configurations failed to process
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

    // Output results in the requested format
    match args.area_only {
        true => {
            // Simple tab-separated output: configuration name and total area
            for (name, r) in &reports {
                println!("{}\t{}", name, export::area(r));
            }
        }
        false => {
            // Full export with detailed breakdown
            export::export(&reports, &args.export)?;
        }
    }

    Ok(())
}
