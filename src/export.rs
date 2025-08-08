use std::fs::{metadata, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use crate::{eliteral, Float, MemeaError, Reports};

/// Write string content to buffer
///
/// # Arguments
/// * `content` - Pointer to string content to write out
/// * `buf` - Buffer created from OpenOptions.  If None, writes to stdout
fn writeout(content: &str, buf: Option<File>) -> Result<(), MemeaError> {
    match buf {
        Some(mut file) => file.write_all(content.as_bytes())?,
        None => io::stdout().write_all(content.as_bytes())?,
    }

    Ok(())
}

pub fn area(reports: &Reports) -> Float {
    reports.iter().map(|r| r.area).sum()
}

/// Exports reports as CSV file or as human-readable output to stdout
///
/// # Arguments
/// * `input` - Name of the configuration to export
/// * `report` - Pointer to a `Report` to output
/// * `filename` - Path of the output file to write.  If None, writes to stdout
pub fn export(
    inputs: Vec<String>,
    reports: &Vec<Reports>,
    filename: &Option<PathBuf>,
) -> Result<(), MemeaError> {
    let buf = match filename {
        Some(x) => {
            if metadata(x).is_ok() {
                print!(
                    "Warning: '{}' already exists.  Overwrite? (Y/n)",
                    x.to_string_lossy()
                );

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim().to_lowercase() == "n" {
                    println!("Aborting...");
                    process::exit(147);
                }
            }

            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(x)?;

            Some(f)
        }
        None => None,
    };

    let mut content = String::new();

    if buf.is_some() {
        // Print CSV header
        content = String::from(
            "Configuration,\
            Name,\
            Type,\
            Count,\
            Location,\
            Area (um2)\n",
        );
    }

    for i in 0..reports.len() {
        if buf.is_none() {
            content = format!("{}{}", content, fmt_direct(&inputs[i], &reports[i]));
        } else {
            content = format!("{}{}", content, fmt_csv(&inputs[i], &reports[i]));
        }
    }

    writeout(&content, buf)?;

    Ok(())
}

fn fmt_csv(input: &str, reports: &Reports) -> String {
    let mut content = String::new();

    for report in reports.iter() {
        content = format!(
            "{}{},{},{},{},{},{}\n",
            content, input, report.name, report.kind, report.count, report.loc, report.area
        );
    }

    content
}

fn fmt_direct(input: &str, reports: &Reports) -> String {
    let mut content = format!(
        "\nConfiguration: {input}\n\
        Area breakdown:\n    \
        Name                 | Type     | Count    | Location | Area (μm²)\n    \
        ---------------------|----------|----------|----------|------------\n"
    );

    for report in reports.iter() {
        content = format!(
            "{}    {:<20} | {:<8} | {:<8} | {:<8} | {:>11.1}\n",
            content,
            report.name,
            report.kind.to_string(),
            report.count,
            report.loc,
            report.area
        );
    }

    content = format!("{}Total area: {:.1} μm²\n", content, area(reports));

    content
}
