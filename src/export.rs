use serde::Serialize;
use std::collections::HashMap;
use std::fs::{metadata, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str;

use crate::db::DBError;
use crate::tabulate::{Report, Reports};
use crate::{infoln, query, Float, MemeaError};

pub fn area(reports: &Reports) -> Float {
    reports.iter().map(|r| r.area).sum()
}

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

fn export_csv(reports: &HashMap<String, Reports>, buf: Option<File>) -> Result<(), MemeaError> {
    let writer: Box<dyn Write> = match buf {
        Some(file) => Box::new(file),
        None => Box::new(io::stdout()),
    };

    let mut wtr = csv::Writer::from_writer(writer);

    for (config, reps) in reports {
        for rep in reps {
            // Wrap report with config name so it's included in CSV

            #[derive(Serialize)]
            struct Row<'a> {
                configuration: &'a str,
                #[serde(flatten)]
                report: &'a Report,
            }
            let row = Row {
                configuration: config,
                report: rep,
            };
            wtr.serialize(row)?;
        }
    }

    wtr.flush()?;
    Ok(())
}

fn export_json(reports: &HashMap<String, Reports>, buf: Option<File>) -> Result<(), MemeaError> {
    match buf {
        Some(file) => serde_json::to_writer_pretty(file, reports)?,
        None => serde_json::to_writer_pretty(io::stdout(), reports)?,
    }
    Ok(())
}

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

fn export_direct(reports: &HashMap<String, Reports>) -> Result<(), MemeaError> {
    for (name, r) in reports {
        println!("{}", fmt_direct(name, r));
    }
    Ok(())
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
            report.celltype.to_string(),
            report.count,
            report.loc,
            report.area
        );
    }

    content = format!("{}Total area: {:.1} μm²\n", content, area(reports));

    content
}
