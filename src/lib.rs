pub mod config;
pub mod db;
pub mod export;
pub mod gds;
pub mod lef;
pub mod tabulate;

use crate::config::ConfigError;
use crate::lef::LefError;

use dialoguer::Completion;
use std::fmt::Write;
use std::fs;
use std::io::{self, Write as IoWrite};
use std::path::Path;
use terminal_size::{terminal_size, Width};
use thiserror::Error;

pub type Float = f32;
pub type Mosaic = (usize, usize);

pub const VER: &str = "v0.1.2";

pub const LOGO: &str = r#"
    __  ___               _________ 
   /  |/  /__  ____ ___  / ____/   |
  / /|_/ / _ \/ __ `__ \/ __/ / /| |
 / /  / /  __/ / / / / / /___/ ___ |
/_/  /_/\___/_/ /_/ /_/_____/_/  |_|
"#;

#[macro_export]
macro_rules! eliteral {
    ($literal:expr) => {
        concat!("\x1b[1;30;41mERROR (Unrecoverable): ", $literal, "\x1b[0m")
    };
}

#[macro_export]
macro_rules! __log_internal {
    ($print:ident, $color:literal, $label:literal, $literal:literal $(, $args:expr)* $(,)?) => {
        $print!(
            concat!("\x1b[", $color, "m", $label, ": ", $literal, "\x1b[0m")
            $(, $args)*
        )
    };
}

// INFO
#[macro_export]
macro_rules! info {
    ($($tt:tt)*) => { $crate::__log_internal!(eprint, "32", "INFO", $($tt)*) }
}
#[macro_export]
macro_rules! infoln {
    ($($tt:tt)*) => { $crate::__log_internal!(eprintln, "32", "INFO", $($tt)*) }
}

// WARN
#[macro_export]
macro_rules! warn {
    ($($tt:tt)*) => { $crate::__log_internal!(eprint, "33", "WARNING", $($tt)*) }
}
#[macro_export]
macro_rules! warnln {
    ($($tt:tt)*) => { $crate::__log_internal!(eprintln, "33", "WARNING", $($tt)*) }
}

// ERROR
#[macro_export]
macro_rules! error {
    ($($tt:tt)*) => { $crate::__log_internal!(eprint, "31", "ERROR", $($tt)*) }
}
#[macro_export]
macro_rules! errorln {
    ($($tt:tt)*) => { $crate::__log_internal!(eprintln, "31", "ERROR", $($tt)*) }
}

// Verbose printing
#[macro_export]
macro_rules! vprintln {
    ($verbose:expr, $($arg:tt)*) => {
        if $verbose {
            $crate::infoln!($($arg)*);
        }
    };
}

#[derive(Debug, Error)]
pub enum MemeaError {
    #[error("GDS parsing error: {0}")]
    GdsParse(#[from] gds::GdsError),
    #[error("GDS error: {0}")]
    Gds(#[from] gds21::GdsError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    #[error("LEF error: {0}")]
    Lef(#[from] LefError),
    #[error("Dialogue error: {0}")]
    Dialogue(#[from] dialoguer::Error),
    #[error("YAML error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("CSV export error: {0}")]
    CSV(#[from] csv::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] crate::db::DBError),
}

pub enum QueryDefault {
    Yes,
    No,
    Neither,
}

pub struct FileCompleter;

// TODO: Remove spaghetti
impl Completion for FileCompleter {
    fn get(&self, input: &str) -> Option<String> {
        let expanded = shellexpand::tilde(input).to_string();
        let path = Path::new(&expanded);
        if let Some(parent) = path.parent() {
            if let Ok(entries) = fs::read_dir(parent) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                        if name.starts_with(path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
                        {
                            return Some(name.to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

pub fn query(prompt: &str, warn: bool, default: QueryDefault) -> Result<bool, MemeaError> {
    let query: &str = match default {
        QueryDefault::No => " (y/N) ",
        QueryDefault::Yes => " (Y/n) ",
        QueryDefault::Neither => " (y/n) ",
    };

    match warn {
        true => warn!("{} {}", prompt, query),
        false => {
            print!("{prompt} {query}");
            std::io::stdout().flush()?;
        }
    }

    let mut input = String::new();

    loop {
        io::stdin().read_line(&mut input)?;
        input = input.trim().to_lowercase();

        if input.is_empty() {
            return match default {
                QueryDefault::Neither => continue,
                QueryDefault::No => Ok(false),
                QueryDefault::Yes => Ok(true),
            };
        }

        match input.as_str() {
            "y" | "ye" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => continue,
        }
    }
}

pub fn bar(header: Option<&str>, ch: char) -> String {
    let width = if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    };

    let mut output = String::new();

    if let Some(text) = header {
        writeln!(output, "{}", ch.to_string().repeat(width)).ok();

        let text_len = text.chars().count();
        let padding = width.saturating_sub(text_len + 2);
        let left_pad = padding / 2;

        writeln!(
            output,
            "{}{}{}{}{}",
            ch,
            " ".repeat(left_pad),
            text,
            " ".repeat(padding - left_pad),
            ch
        )
        .ok();
    }

    write!(output, "{}", ch.to_string().repeat(width)).ok();

    output
}

fn get_scale(n: &usize) -> Option<Float> {
    match n {
        65 => Some(0.52),
        28 => Some(0.12),
        22 => Some(0.095),
        16 => Some(0.074),
        10 => Some(0.042),
        7 => Some(0.027),
        5 => Some(0.021),
        3 => Some(0.1999),
        _ => None,
    }
}

pub fn scale(from: usize, to: usize) -> Float {
    let scale_from = get_scale(&from);
    let scale_to = get_scale(&to);

    match (scale_from, scale_to) {
        (Some(val_a), Some(val_b)) => val_b / val_a,
        _ => {
            if scale_from.is_none() {
                warnln!(
                    "Warning: {} not a recognized automatic scaling technology size.",
                    from
                )
            }
            if scale_to.is_none() {
                warnln!(
                    "Warning: {} not a recognized automatic scaling technology size.",
                    to
                )
            }
            1.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: Float,
    pub max: Float,
}

pub fn parse_range(line: &str) -> Result<Range, MemeaError> {
    let (min, max) = parse_tuple(line)?;
    Ok(Range { min, max })
}

pub fn parse_tuple(line: &str) -> Result<(Float, Float), MemeaError> {
    let (a, b) = line
        .trim()
        .trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != ',' && c != ';' && c != '-')
        .split_once(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .ok_or(MemeaError::ParseError(line.to_string()))?;

    let a: Float = a.trim().parse::<Float>()?;
    let b: Float = b.trim().parse::<Float>()?;

    Ok((a, b))
}
