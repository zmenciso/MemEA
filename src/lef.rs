//! LEF file parsing and database creation for MemEA memory components.
//!
//! This module provides functionality to parse Library Exchange Format (LEF) files
//! and create component databases. It extracts cell dimensions from LEF files and
//! optionally augments them with enclosure data from corresponding GDS layout files.
//! The resulting data is saved as a component database for use in area estimation.

use dialoguer::Input;
use gds21::GdsLibrary;
use regex::Regex;
use std::fs::{metadata, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::{db::*, gds, FileCompleter, QueryDefault};
use crate::{errorln, query, vprintln, warnln, Float, MemeaError};

/// Errors that can occur during LEF file parsing.
#[derive(Debug, Error)]
pub enum LefError {
    /// Indicates that a MACRO line in the LEF file is malformed.
    #[error("Malformed MACRO line: {0}")]
    InvalidMacro(String),
    /// Indicates that a SIZE line in the LEF file cannot be parsed.
    #[error("Malformed SIZE line: {0}")]
    InvalidSize(String),
}

/// Interactively adds a cell to the database with user confirmation and type selection.
///
/// This function displays cell information to the user, asks for confirmation to add
/// the cell, and prompts for the cell type (core, switch, logic, or ADC). It handles
/// the interactive workflow for building a component database from LEF data.
///
/// # Arguments
/// * `name` - Name of the cell to add
/// * `dims` - Physical dimensions of the cell
/// * `db` - Mutable reference to the database to update
///
/// # Returns
/// * `Ok(())` - Cell was successfully processed (added or skipped)
/// * `Err(MemeaError)` - Error during user interaction or database update
fn add_cell(name: &str, dims: Dims, db: &mut Database) -> Result<(), MemeaError> {
    println!("\nCell.......: {name}");
    dims.dump();
    println!();

    // See if the user wants to add it
    if !query(
        &format!("Add cell {name} to database?"),
        false,
        QueryDefault::Yes,
    )? {
        return Ok(());
    }

    loop {
        let mut celltype: String = prompt("Cell type");
        celltype = celltype.trim().to_lowercase();

        match celltype.as_str() {
            "1" | "core" => {
                db.add_core(name, dims);
                break;
            }
            "2" | "switch" | "sw" => {
                db.add_switch(name, dims);
                break;
            }
            "3" | "logic" | "log" => {
                db.add_logic(name, dims);
                break;
            }
            "4" | "adc" => {
                db.add_adc(name, dims);
                break;
            }
            _ => {
                errorln!(
                    "Invalid cell type (must be one of 1/core, 2/sw/switch, 3/log/logic, or 4/adc)"
                );
            }
        }
    }

    println!("\n{}", crate::bar(None, '-'));
    Ok(())
}

/// Interactive LEF file processing workflow.
///
/// This function provides an interactive command-line interface for processing
/// LEF files and creating component databases. It prompts the user for:
/// - GDS file (optional, for enclosure computation)
/// - LEF file (required, for cell dimensions)
/// - Output database file (YAML or JSON format)
///
/// # Arguments
/// * `verbose` - Whether to show detailed processing information
///
/// # Returns
/// * `Ok(())` - LEF processing completed successfully
/// * `Err(MemeaError)` - File I/O error, parsing error, or user interaction error
///
/// # Examples
/// ```no_run
/// use memea::lef::lefin;
///
/// // Start interactive LEF processing
/// lefin(true).expect("LEF processing failed");
/// ```
pub fn lefin(verbose: bool) -> Result<(), MemeaError> {
    let mut gdsfile: String;
    let mut leffile: String;
    let mut dbout: String;

    loop {
        gdsfile = Input::new()
            .with_prompt("GDS file")
            .completion_with(&FileCompleter)
            .interact_text()?;

        let path = Path::new(&gdsfile);

        if gdsfile.is_empty() {
            warnln!("No GDS file provided; enclosures will not be computed.");
            break;
        } else if metadata(path).is_ok() && path.extension().and_then(|e| e.to_str()) == Some("gds")
        {
            break;
        } else {
            errorln!("{} is not a GDS file", gdsfile);
        }
    }

    loop {
        leffile = Input::new()
            .with_prompt("LEF file")
            .completion_with(&FileCompleter)
            .interact_text()?;

        let path = Path::new(&leffile);

        if metadata(path).is_ok() && path.extension().and_then(|e| e.to_str()) == Some("lef") {
            break;
        } else {
            errorln!("{} is not a LEF file", leffile);
        }
    }

    loop {
        dbout = Input::new()
            .with_prompt("Output database file")
            .completion_with(&FileCompleter)
            .interact_text()?;

        let valid = valid_ext(&dbout);

        if valid && metadata(&dbout).is_ok() {
            let allow = query(
                format!("'{dbout}' already exists. Overwrite?").as_str(),
                true,
                crate::QueryDefault::Yes,
            )?;

            if allow {
                break;
            }
        } else if valid {
            break;
        } else {
            errorln!(
                "Output database {} must be a YAML (.yml, .yaml) or JSON (.json) file",
                dbout
            );
        }
    }

    println!();

    let gdsin = if gdsfile.is_empty() {
        None
    } else {
        Some(PathBuf::from(&gdsfile))
    };

    read_lef(PathBuf::from(leffile), gdsin, PathBuf::from(dbout), verbose)
}

/// Parses width and height from a LEF SIZE line using regex.
///
/// This function extracts two floating-point numbers from a SIZE line in a LEF file,
/// representing the width and height of a cell in micrometers.
///
/// # Arguments
/// * `line` - The SIZE line from the LEF file to parse
///
/// # Returns
/// * `Ok((width, height))` - Successfully parsed dimensions in micrometers
/// * `Err(LefError::InvalidSize)` - Line format is invalid or missing numbers
///
/// # Examples
/// ```
/// use memea::lef::parse_size;
///
/// let line = "    SIZE 1.5 BY 2.0 ;";
/// let (w, h) = parse_size(line).expect("Failed to parse size");
/// assert_eq!((w, h), (1.5, 2.0));
/// ```
fn parse_size(line: &str) -> Result<(Float, Float), LefError> {
    let re = Regex::new(r"([0-9]+\.?[0-9]*)").unwrap();

    let mut nums = re
        .captures_iter(line)
        .filter_map(|cap| cap.get(1))
        .filter_map(|m| m.as_str().parse::<Float>().ok());

    match (nums.next(), nums.next()) {
        (Some(a), Some(b)) => Ok((a, b)),
        _ => Err(LefError::InvalidSize(line.to_string())),
    }
}

/// Reads and processes a LEF file to create a component database.
///
/// This function parses a LEF file line by line, extracting MACRO names and SIZE
/// information to build component dimensions. If a GDS file is provided, it augments
/// the dimensions with enclosure data computed from the layout geometry.
///
/// # Arguments
/// * `lefin` - Path to the input LEF file
/// * `gdsin` - Optional path to GDS file for enclosure computation
/// * `dbout` - Path where the output database should be saved
/// * `verbose` - Whether to show detailed processing information
///
/// # Returns
/// * `Ok(())` - LEF file processed and database saved successfully
/// * `Err(MemeaError)` - File I/O error, parsing error, or database save error
///
/// # LEF File Format
/// The function expects LEF files with MACRO definitions containing SIZE lines:
/// ```text
/// MACRO cell_name
///   SIZE width BY height ;
/// END cell_name
/// ```
fn read_lef(
    lefin: PathBuf,
    gdsin: Option<PathBuf>,
    dbout: PathBuf,
    verbose: bool,
) -> Result<(), MemeaError> {
    let lefin = File::open(lefin)?;
    let rdr = BufReader::new(lefin);

    // TODO: Currently assuming microns for LEF, need to scale this by LEF unit scale
    let mut gdsunits = 1e-9;

    let map = match gdsin {
        Some(file) => {
            let lib = GdsLibrary::load(&file)?;
            gdsunits = lib.units.db_unit();

            vprintln!(
                verbose,
                "GDS library {} loaded, found {} cells",
                file.to_string_lossy(),
                lib.structs.len()
            );

            Some(gds::hash_lib(lib))
        }
        None => None,
    };

    let mut name: String = String::new();
    let mut dims: Option<Dims> = None;

    let mut db = Database::new();

    println!("Cell types: 1/core, 2/sw/switch, 3/log/logic, or 4/adc\n");
    println!("{}", crate::bar(None, '-'));

    for line in rdr.lines() {
        let line = line?;
        let line = line.trim();

        if line.contains("MACRO") {
            // Push previous cell
            if let Some(c) = dims.take() {
                add_cell(&name, c, &mut db)?;
            }

            // Get new cell name
            let n = line
                .split_once(' ')
                .ok_or(LefError::InvalidMacro(line.to_owned()))?
                .1;

            name = n.to_string();
        }

        if line.contains("SIZE") {
            // Get size
            let (w, h) = parse_size(line)?;
            dims = match &map {
                Some(m) => Some(gds::augment_dims(m, &name, w, h, gdsunits, verbose)?),
                None => Some(Dims::from(w, h, 0.0, 0.0)),
            }
        }
    }

    // Push last cell
    if let Some(c) = dims {
        add_cell(&name, c, &mut db)?;
        println!();
    }

    // Write database to file
    db.save(&dbout, verbose)?;

    Ok(())
}
