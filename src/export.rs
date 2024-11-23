use std::fs::{OpenOptions, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::collections::HashMap;

pub fn vprint(message: &str, verbose: bool) {
    if verbose { println!("{}", message); }
}

pub fn error(message: String) {
    eprintln!("\x1b[0;31;40mERROR: {}\x1b[0m", message);
}

fn writeout(content: String, buf: Option<File>) {
    match buf {
        Some(mut file) => file.write_all(content.as_bytes()),
        None => io::stdout().write_all(content.as_bytes()) 
    }.expect("Could not write bytes to file");
}

pub fn area(report: &HashMap<String, f32>) -> f32 {
    report.values().sum()
}

pub fn export(input: &str, report: &HashMap<String, f32>, filename: &Option<PathBuf>) {
    let buf = match filename {
        Some(x) => {
            let f = OpenOptions::new()
                .append(true)
                .create(true)
                .open(x)
                .expect("Could not open file");

            Some(f)
        },
        None => None
    };

    let mut content = format!("Configuration: {}\n\
        Area breakdown:\n", input);

    for (name, area) in report.into_iter() {
        content = format!("{}    {:<24} | {:>10.1} μm²\n", content, name, area);
    }

    content = format!("{}Total area: {:.1} μm²\n\n", content, area(report));

    writeout(content, buf);
}
