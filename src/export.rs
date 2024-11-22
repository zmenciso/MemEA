use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::collections::HashMap;

pub fn vprint(message: &str, verbose: bool) {
    if verbose { println!("{}", message); }
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

pub fn export(input: &PathBuf, report: &HashMap<String, f32>, filename: Option<PathBuf>) {
    let buf = match filename {
        Some(x) => Some(File::create(x).expect("Could not create file")),
        None => None
    };

    let mut content = format!("periph_gen\n\
        Configuration: {:?}\n\n\
        Area breakdown:\n", input);

    for (name, area) in report.into_iter() {
        content = format!("{}    {:<24} | {:>10.3} μm²\n", content, name, area);
    }

    content = format!("{}\nTotal area: {:.3} μm²", content, area(report));

    writeout(content, buf);
}
