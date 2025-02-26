use std::fs::{OpenOptions, File, metadata};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use crate::{eliteral, Reports};

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

pub fn area(reports: &Reports) -> f32 {
    reports.iter()
        .map(|r| r.area)
        .sum()
}

/// Exports reports as CSV file or as human-readable output to stdout
///
/// # Arguments
/// * `input` - Name of the configuration to export
/// * `report` - Pointer to a `Report` to output
/// * `filename` - Path of the output file to write.  If None, writes to stdout
///
/// # Panics
/// Could not open output file for writing
/// Cannot read user input during prompt from stdin
pub fn export(inputs: Vec<String>, reports: &Vec<Reports>, filename: &Option<PathBuf>) {
    let buf = match filename {
        Some(x) => {
            if metadata(x).is_ok() {
                println!("Warning: '{}' already exists.  Overwrite? (Y/n)", x.to_string_lossy());

                let mut input = String::new();
                io::stdin().read_line(&mut input)
                    .expect("Error: Could not read user input");

                if input.trim().to_lowercase() == "n" {
                    println!("Aborting...");
                    process::exit(147);
                }
            }

            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(x)
                .expect(eliteral!("Could not open file"));

            Some(f)
        },
        None => None
    };

    let mut content = String::new();

    if buf.is_some() {
        // Print CSV header
        content = String::from("Configuration,\
            Name,\
            Type,\
            Count,\
            Location,\
            Area\n");
    }

    for i in 0 .. reports.len() {
        if buf.is_none() {
            content = format!("{}{}", 
                content, 
                fmt_direct(&inputs[i], &reports[i]));
        }
        else {
            content = format!("{}{}", 
                content, 
                fmt_csv(&inputs[i], &reports[i]));
        }
    }

    writeout(&content, buf);

}

fn fmt_csv(input: &str, reports: &Reports) -> String {
    let mut content = String::new();

    for report in reports.iter() {
        content = format!("{}{},{},{},{},{},{}\n",
            content,
            input,
            report.name,
            report.kind,
            report.count,
            report.loc,
            report.area);
    }

    content
}

fn fmt_direct(input: &str, reports: &Reports) -> String {
    let mut content = format!("\nConfiguration: {}\n\
        Area breakdown:\n    \
        Name                 | Type     | Count    | Location | Area (μm²)\n    \
        ---------------------|----------|----------|----------|------------\n", input);

    for report in reports.iter() {
        content = format!("{}    {:<20} | {:<8} | {:<8} | {:<8} | {:>11.1}\n",
            content,
            report.name,
            report.kind.to_string(),
            report.count,
            report.loc,
            report.area);
    }

    content = format!("{}Total area: {:.1} μm²\n", content, area(reports));

    content
}
