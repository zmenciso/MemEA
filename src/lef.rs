use chrono::Local;
use dialoguer::Input;
use gds21::GdsLibrary;
use regex::Regex;
use std::fs::{metadata, File, OpenOptions};
use std::io::{stdin, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::{bar, db::*, gds, FileCompleter, QueryDefault, VER};
use crate::{errorln, infoln, query, warnln, Float, MemeaError};

#[derive(Debug, Error)]
pub enum LefError {
    #[error("Malformed MACRO line: {0}")]
    InvalidMacro(String),
    #[error("Malformed SIZE line: {0}")]
    InvalidSize(String),
}

fn get_value(prompt: &str) -> Result<String, MemeaError> {
    print!("{prompt} ");

    let mut value = String::new();
    stdin().read_line(&mut value)?;
    Ok(value)
}

fn write_cell(name: &str, dims: Dims, wtr: &mut BufWriter<File>) -> Result<(), MemeaError> {
    // See if the user wants to add it
    if !query(
        &format!("Add cell {name} to database?"),
        true,
        QueryDefault::Yes,
    )? {
        return Ok(());
    }

    let mut celltype = String::new();
    let allowed = [
        "1", "2", "3", "4", "core", "switch", "sw", "logic", "log", "adc",
    ];

    while !allowed.iter().any(|&s| celltype.contains(s)) {
        match celltype.as_str() {
            "1" | "core" => {
                writeln!(wtr, "core: {name}")?;
                writeln!(wtr, "dx_wl {}", get_value("dx_wl:")?)?;
                writeln!(wtr, "dx_bl {}", get_value("dx_bl:")?)?;
                writeln!(wtr, "{}", dims.dump())?;
            }
            "2" | "switch" | "sw" => {
                writeln!(wtr, "switch: {name}")?;
                writeln!(wtr, "voltage {}", get_value("voltage:")?)?;
                writeln!(wtr, "dx {}", get_value("dx:")?)?;
                writeln!(wtr, "{}", dims.dump())?;
            }
            "3" | "logic" | "log" => {
                writeln!(wtr, "logic: {name}")?;
                writeln!(wtr, "bits {}", get_value("bits:")?)?;
                writeln!(wtr, "fs {}", get_value("fs:")?)?;
                writeln!(wtr, "dx {}", get_value("dx:")?)?;
                writeln!(wtr, "{}", dims.dump())?;
            }
            "4" | "adc" => {
                writeln!(wtr, "adc: {name}")?;
                writeln!(wtr, "bits {}", get_value("bits:")?)?;
                writeln!(wtr, "fs {}", get_value("fs:")?)?;
                writeln!(wtr, "{}", dims.dump())?;
            }
            _ => {
                print!("Cell type? 1/core, 2/sw/switch, 3/log/logic, 4/adc ");
                stdin().read_line(&mut celltype)?;
                celltype = celltype.to_lowercase();
            }
        }
    }

    writeln!(wtr)?;
    println!("{}", bar(None, '-'));

    Ok(())
}

pub fn lefin() -> Result<(), MemeaError> {
    let mut gdsfile: String;
    let mut leffile: String;

    loop {
        gdsfile = Input::new()
            .with_prompt("GDS file")
            .completion_with(&FileCompleter)
            .interact_text()?;

        if gdsfile.is_empty() {
            warnln!("No GDS file provided; enclosures will not be computed.");
            break;
        } else if metadata(Path::new(&gdsfile)).is_ok() {
            break;
        } else {
            errorln!("{} is not a regular file", gdsfile);
        }
    }

    loop {
        leffile = Input::new()
            .with_prompt("LEF file")
            .completion_with(&FileCompleter)
            .interact_text()?;

        if metadata(Path::new(&leffile)).is_ok() {
            break;
        } else {
            errorln!("{} is not a regular file", leffile);
        }
    }

    let dbout: String = Input::new()
        .with_prompt("Output database file")
        .completion_with(&FileCompleter)
        .interact_text()?;

    let gdsin = if gdsfile.is_empty() {
        None
    } else {
        Some(PathBuf::from(&gdsfile))
    };

    read_lef(PathBuf::from(leffile), gdsin, PathBuf::from(dbout))
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

fn stamp(wtr: &mut BufWriter<File>) -> Result<(), MemeaError> {
    let now = Local::now();
    writeln!(wtr, "MemEA {VER}")?;
    writeln!(wtr, "File generated with LEF+GDS import on {now}")?;

    Ok(())
}

fn read_lef(lefin: PathBuf, gdsin: Option<PathBuf>, dbout: PathBuf) -> Result<(), MemeaError> {
    let lefin = File::open(lefin)?;
    let rdr = BufReader::new(lefin);

    // If file already exists, prompt to overwrite
    if metadata(&dbout).is_ok() {
        let allow = query(
            format!("'{}' already exists. Overwrite?", dbout.to_string_lossy()).as_str(),
            true,
            crate::QueryDefault::Yes,
        )?;

        if !allow {
            infoln!("Aborting...");
            return Ok(());
        }
    }

    // TODO: Currently assuming microns for LEF, need to scale this by LEF unit scale
    let mut gdsunits = 1e-9;

    let map = match gdsin {
        Some(file) => {
            let lib = GdsLibrary::load(file)?;
            gdsunits = lib.units.db_unit();

            Some(gds::hash_lib(lib))
        }
        None => None,
    };

    let dbout = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dbout)?;

    let mut wtr = BufWriter::new(dbout);
    stamp(&mut wtr)?;

    let mut name: String = String::new();
    let mut dims: Option<Dims> = None;

    for line in rdr.lines() {
        let line = line?;
        let line = line.trim();

        if line.contains("MACRO") {
            // Push previous cell
            if let Some(c) = dims.take() {
                write_cell(&name, c, &mut wtr)?;
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
                Some(m) => Some(gds::augment_dims(m, &name, w, h, gdsunits)?),
                None => Some(Dims::from(w, h, 0.0, 0.0)),
            }
        }
    }

    // Push last cell
    if let Some(c) = dims {
        write_cell(&name, c, &mut wtr)?;
    }

    Ok(())
}
