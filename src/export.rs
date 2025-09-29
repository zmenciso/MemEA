//! Export functionality for MemEA analysis results.
//!
//! This module provides multiple export formats for memory peripheral estimation
//! results, including CSV, JSON, YAML, and direct console output. It handles
//! file creation, overwrite confirmation, and format-specific serialization.

use std::collections::HashMap;
use std::fs::{metadata, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str;

use crate::db::DBError;
use crate::tabulate::{Report, Reports};
use crate::{infoln, query, Float, MemeaError};

/// Calculates the total area from a collection of reports.
///
/// # Arguments
/// * `reports` - Collection of reports to sum areas from
///
/// # Returns
/// Total area in square micrometers
pub fn area(reports: &Reports) -> Float {
    reports.iter().map(|r| r.area).sum()
}

/// Exports analysis results to various formats based on file extension.
///
/// This function determines the output format from the file extension and handles
/// file creation with overwrite confirmation. Supported formats include CSV, JSON,
/// YAML, and direct console output.
///
/// # Arguments
/// * `reports` - HashMap of configuration names to their corresponding reports
/// * `filename` - Optional output file path. If None, outputs to stdout
///
/// # Returns
/// * `Ok(())` - Export completed successfully
/// * `Err(MemeaError)` - File I/O error, serialization error, or unsupported format
///
/// # Examples
/// ```no_run
/// use memea::export::export;
/// use std::path::PathBuf;
/// use std::collections::HashMap;
///
/// let reports = HashMap::new(); // populated with analysis results
/// let output_file = Some(PathBuf::from("results.csv"));
/// export(&reports, &output_file).expect("Export failed");
/// ```
pub fn export(
    reports: &HashMap<String, Reports>,
    filename: &Option<PathBuf>,
) -> Result<(), MemeaError> {
    let buf = match filename {
        Some(x) => {
            if metadata(x).is_ok() {
                let allow = query(
                    format!("'{}' already exists. Overwrite?", x.to_string_lossy()).as_str(),
                    true,
                    crate::QueryDefault::Yes,
                )?;
                if !allow {
                    infoln!("Aborting...");
                    return Ok(());
                }
            }

            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(x)?;

            infoln!("Wrote output to {:#?}", x);

            Some(f)
        }
        None => None,
    };

    let format = filename
        .as_ref()
        .and_then(|f| f.extension().and_then(|s| s.to_str()))
        .unwrap_or("direct")
        .to_lowercase();

    match format.as_str() {
        "csv" => export_csv(reports, buf)?,

        "json" => export_json(reports, buf)?,
        "yaml" | "yml" => export_yaml(reports, buf)?,
        "direct" => export_direct(reports)?,
        other => {
            return Err(DBError::FileType(other.to_string()).into());
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct Row<'a> {
    #[serde(rename = "Configuration")]
    configuration: &'a str,
    #[serde(rename = "Name")]
    name: &'a str,
    #[serde(rename = "Type")]
    celltype: String,
    #[serde(rename = "Count")]
    count: usize,
    #[serde(rename = "Location")]
    location: &'a str,
    #[serde(rename = "Area (μm2)")]
    area: Float,
}

impl<'a> Row<'a> {
    fn from_report(config: &'a str, rep: &'a Report) -> Self {
        Row {
            configuration: config,
            name: &rep.name,
            celltype: rep.celltype.to_string(),
            count: rep.count,
            location: &rep.loc,
            area: rep.area,
        }
    }
}

/// Exports reports to CSV format with configuration names included.
///
/// Each row in the CSV contains a configuration name along with flattened
/// report data for easy analysis in spreadsheet applications.
///
/// # Arguments
/// * `reports` - HashMap of configuration names to reports
/// * `buf` - Optional file buffer, uses stdout if None
///
/// # Returns
/// * `Ok(())` - CSV export completed successfully
/// * `Err(MemeaError)` - Serialization or I/O error
fn export_csv(reports: &HashMap<String, Reports>, buf: Option<File>) -> Result<(), MemeaError> {
    let writer: Box<dyn Write> = match buf {
        Some(file) => Box::new(file),
        None => Box::new(io::stdout()),
    };

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(writer);

    for (config, reps) in reports {
        for rep in reps {
            // TODO: Cannot serialize maps
            wtr.serialize(Row::from_report(config, rep))?;
        }
    }

    wtr.flush()?;
    Ok(())
}

/// Exports reports to JSON format with pretty printing.
///
/// # Arguments
/// * `reports` - HashMap of configuration names to reports
/// * `buf` - Optional file buffer, uses stdout if None
///
/// # Returns
/// * `Ok(())` - JSON export completed successfully
/// * `Err(MemeaError)` - Serialization or I/O error
fn export_json(reports: &HashMap<String, Reports>, buf: Option<File>) -> Result<(), MemeaError> {
    match buf {
        Some(file) => serde_json::to_writer_pretty(file, reports)?,
        None => serde_json::to_writer_pretty(io::stdout(), reports)?,
    }
    Ok(())
}

/// Exports reports to YAML format.
///
/// # Arguments
/// * `reports` - HashMap of configuration names to reports
/// * `buf` - Optional file buffer, uses stdout if None
///
/// # Returns
/// * `Ok(())` - YAML export completed successfully
/// * `Err(MemeaError)` - Serialization or I/O error
fn export_yaml(reports: &HashMap<String, Reports>, buf: Option<File>) -> Result<(), MemeaError> {
    match buf {
        Some(mut file) => {
            let s = serde_yaml::to_string(reports)?;
            file.write_all(s.as_bytes())?;
        }
        None => {
            let s = serde_yaml::to_string(reports)?;
            println!("{s}");
        }
    }
    Ok(())
}

/// Exports reports in human-readable table format to stdout.
///
/// This format provides a clean, formatted table showing area breakdown
/// by component type with totals for each configuration.
///
/// # Arguments
/// * `reports` - HashMap of configuration names to reports
///
/// # Returns
/// * `Ok(())` - Direct export completed successfully
/// * `Err(MemeaError)` - Formatting or I/O error
fn export_direct(reports: &HashMap<String, Reports>) -> Result<(), MemeaError> {
    for (name, r) in reports {
        println!("{}", fmt_direct(name, r));
    }
    Ok(())
}

/// Formats reports into a human-readable table string.
///
/// Creates a formatted table showing component breakdown with columns for
/// name, type, count, location, and area. Includes a total area summary.
///
/// # Arguments
/// * `input` - Configuration name to display as header
/// * `reports` - Collection of reports to format
///
/// # Returns
/// Formatted string containing the complete table
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
            report.celltype.to_string(),
            report.count,
            report.loc,
            report.area
        );
    }

    content = format!("{}Total area: {:.1} μm²\n", content, area(reports));

    content
}
