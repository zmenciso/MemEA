use std::fs::{OpenOptions, File};
use std::io::{self, Write};
use std::path::PathBuf;

use crate::{eliteral, Report};

/// Write string content to buffer
///
/// # Arguments
/// * `content` - Pointer to string content to write out
/// * `buf` - Buffer created from OpenOptions.  If None, writes to stdout
///
/// # Panics
/// Could not write bytes to the output file
fn writeout(content: &str, buf: Option<File>) {
    match buf {
        Some(mut file) => file.write_all(content.as_bytes()),
        None => io::stdout().write_all(content.as_bytes()) 
    }.expect(eliteral!("Could not write bytes to file"));
}

pub fn area(reports: &Report) -> f32 {
    reports.iter()
        .map(|&(_, value)| value)
        .sum()
}

/// Exports `Report` to file.
/// TODO: If exporting to a file (not stdout), format as CSV
///
/// # Arguments
/// * `input` - Name of the configuration to export
/// * `report` - Pointer to a `Report` to output
/// * `filename` - Path of the output file to write.  If None, writes to stdout
///
/// # Panics
/// Could not open output file for writing
pub fn export(input: &str, report: &Report, filename: &Option<PathBuf>) {
    let buf = match filename {
        Some(x) => {
            let f = OpenOptions::new()
                .append(true)
                .create(true)
                .open(x)
                .expect(eliteral!("Could not open file"));

            Some(f)
        },
        None => None
    };

    let mut content = format!("\nConfiguration: {}\n\
        Area breakdown:\n", input);

    for (name, area) in report.iter() {
        content = format!("{}    {:<24} | {:>10.1} μm²\n", content, name, area);
    }

    content = format!("{}Total area: {:.1} μm²\n", content, area(report));

    writeout(&content, buf);
}
