use dialoguer::Input;
use gds21::GdsLibrary;
use regex::Regex;
use std::fs::{metadata, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::{db::*, gds, FileCompleter, QueryDefault};
use crate::{errorln, query, vprintln, warnln, Float, MemeaError};

#[derive(Debug, Error)]
pub enum LefError {
    #[error("Malformed MACRO line: {0}")]
    InvalidMacro(String),
    #[error("Malformed SIZE line: {0}")]
    InvalidSize(String),
}

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
