//! MemEA - Memory Peripheral Estimation and Analysis Library
//!
//! MemEA is a Rust library for estimating the area and characteristics of memory
//! peripheral components. It provides tools for parsing component databases,
//! processing layout files (GDS/LEF), and generating detailed area reports for
//! memory arrays and their supporting circuitry.
//!
//! # Features
//!
//! - **Component Database Management**: Create and manage databases of memory cells,
//!   logic blocks, switches, and ADCs with their physical and electrical characteristics
//! - **Layout File Processing**: Extract dimensions and enclosure data from GDS and LEF files
//! - **Configuration Management**: Handle multiple memory configurations with YAML/JSON support
//! - **Area Estimation**: Calculate detailed area breakdowns for memory peripherals
//! - **Multiple Export Formats**: Output results in CSV, JSON, YAML, or human-readable tables
//!
//! # Quick Start
//!
//! ```rust
//! use memea::{config, db, export};
//! use std::path::PathBuf;
//! use std::collections::HashMap;
//!
//! // Load component database
//! let db_path = PathBuf::from("components.yaml");
//! let database = db::build_db(&db_path)?;
//!
//! // Load configurations
//! let config_paths = vec![PathBuf::from("config.yaml")];
//! let configs = config::read_all(&config_paths);
//!
//! // Process and export results
//! let reports = HashMap::new(); // populated with analysis results
//! let output_file = Some(PathBuf::from("results.csv"));
//! export::export(&reports, &output_file)?;
//! # Ok::<(), memea::MemeaError>(())
//! ```

pub mod config;
pub mod db;
pub mod export;
pub mod gds;
pub mod lef;
pub mod tabulate;

use crate::config::ConfigError;
use crate::lef::LefError;

use dialoguer::Completion;
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs::{self, metadata};
use std::io::{self, Write as IoWrite};
use std::path::Path;
use terminal_size::{terminal_size, Width};
use thiserror::Error;

/// Floating-point type used throughout MemEA for measurements and calculations.
pub type Float = f32;

/// Type representing memory array dimensions as (rows, columns).
pub type Mosaic = (usize, usize);

/// Current version of the MemEA library.
pub const VER: &str = "v0.1.2";

/// ASCII art logo for the MemEA application.
pub const LOGO: &str = r#"
    __  ___               _________
   /  |/  /__  ____ ___  / ____/   |
  / /|_/ / _ \/ __ `__ \/ __/ / /| |
 / /  / /  __/ / / / / / /___/ ___ |
/_/  /_/\___/_/ /_/ /_/_____/_/  |_|
"#;

/// Macro for creating formatted error literals with red background.
#[macro_export]
macro_rules! eliteral {
    ($literal:expr) => {
        concat!("\x1b[1;30;41mERROR (Unrecoverable): ", $literal, "\x1b[0m")
    };
}

/// Internal macro for colored log message formatting.
#[macro_export]
macro_rules! __log_internal {
    ($print:ident, $color:literal, $label:literal, $literal:literal $(, $args:expr)* $(,)?) => {
        $print!(
            concat!("\x1b[", $color, "m", $label, ": ", $literal, "\x1b[0m")
            $(, $args)*
        )
    };
}

/// Macro for printing informational messages in green without newline.
#[macro_export]
macro_rules! info {
    ($($tt:tt)*) => { $crate::__log_internal!(eprint, "32", "INFO", $($tt)*) }
}

/// Macro for printing informational messages in green with newline.
#[macro_export]
macro_rules! infoln {
    ($($tt:tt)*) => { $crate::__log_internal!(eprintln, "32", "INFO", $($tt)*) }
}

/// Macro for printing warning messages in yellow without newline.
#[macro_export]
macro_rules! warn {
    ($($tt:tt)*) => { $crate::__log_internal!(eprint, "33", "WARNING", $($tt)*) }
}

/// Macro for printing warning messages in yellow with newline.
#[macro_export]
macro_rules! warnln {
    ($($tt:tt)*) => { $crate::__log_internal!(eprintln, "33", "WARNING", $($tt)*) }
}

/// Macro for printing error messages in red without newline.
#[macro_export]
macro_rules! error {
    ($($tt:tt)*) => { $crate::__log_internal!(eprint, "31", "ERROR", $($tt)*) }
}

/// Macro for printing error messages in red with newline.
#[macro_export]
macro_rules! errorln {
    ($($tt:tt)*) => { $crate::__log_internal!(eprintln, "31", "ERROR", $($tt)*) }
}

/// Macro for conditional verbose printing - only prints if verbose flag is true.
#[macro_export]
macro_rules! vprintln {
    ($verbose:expr, $($arg:tt)*) => {
        if $verbose {
            $crate::infoln!($($arg)*);
        }
    };
}

/// Comprehensive error type for all MemEA operations.
///
/// This enum covers all possible errors that can occur during MemEA operations,
/// including file I/O, parsing, database operations, and user interaction errors.
/// Most variants automatically convert from their underlying error types using
/// the `#[from]` attribute.
#[derive(Debug, Error)]
pub enum MemeaError {
    /// GDS parsing-specific error from the gds module.
    #[error("GDS parsing error: {0}")]
    GdsParse(#[from] gds::GdsError),
    /// GDS library error from the gds21 crate.
    #[error("GDS error: {0}")]
    Gds(#[from] gds21::GdsError),
    /// Standard I/O error (file operations, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Integer parsing error.
    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    /// Floating-point parsing error.
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),
    /// Configuration file parsing error.
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    /// LEF file parsing error.
    #[error("LEF error: {0}")]
    Lef(#[from] LefError),
    /// User interaction error from dialoguer.
    #[error("Dialogue error: {0}")]
    Dialogue(#[from] dialoguer::Error),
    /// YAML serialization/deserialization error.
    #[error("YAML error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    /// CSV export error.
    #[error("CSV export error: {0}")]
    CSV(#[from] csv::Error),
    /// General parsing error with custom message.
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Database operation error.
    #[error("Database error: {0}")]
    DatabaseError(#[from] crate::db::DBError),
}

/// Default response options for user queries.
pub enum QueryDefault {
    /// Default to "yes" if user presses enter without input.
    Yes,
    /// Default to "no" if user presses enter without input.
    No,
    /// Require explicit user input (no default).
    Neither,
}

/// File completion handler for interactive prompts.
///
/// Provides tab completion functionality for file paths in interactive
/// command-line interfaces.
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

/// Prompts the user for a yes/no response with optional default.
///
/// This function displays a prompt to the user and waits for a yes/no response.
/// It can display the prompt as a warning (in yellow) or normal text, and
/// supports default responses when the user presses enter without typing.
///
/// # Arguments
/// * `prompt` - The question to ask the user
/// * `warn` - Whether to display the prompt as a warning (colored)
/// * `default` - Default response behavior if user presses enter
///
/// # Returns
/// * `Ok(true)` - User confirmed with yes
/// * `Ok(false)` - User declined with no
/// * `Err(MemeaError)` - I/O error during user interaction
///
/// # Examples
/// ```no_run
/// use memea::{query, QueryDefault};
///
/// let overwrite = query("File exists. Overwrite?", true, QueryDefault::No)?;
/// if overwrite {
///     println!("User chose to overwrite");
/// }
/// # Ok::<(), memea::MemeaError>(())
/// ```
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

/// Creates a formatted horizontal bar for terminal output.
///
/// This function generates a horizontal separator bar using the specified character,
/// optionally with a centered header text. The bar width adapts to the terminal
/// size or defaults to 80 characters.
///
/// # Arguments
/// * `header` - Optional text to center in the bar
/// * `ch` - Character to use for the bar (e.g., '-', '=', '*')
///
/// # Returns
/// Formatted string containing the bar with optional header
///
/// # Examples
/// ```
/// use memea::bar;
///
/// let simple_bar = bar(None, '-');
/// let header_bar = bar(Some("Results"), '=');
/// println!("{}", header_bar);
/// ```
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

/// Returns the scaling factor for a given technology node.
///
/// This function provides predefined scaling factors based on industry-
/// reported SRAM cell size trends. Returns `None` for unrecognized nodes.
///
/// # Arguments
/// * `n` - Technology node size in nanometers
///
/// # Returns
/// Scaling factor for the technology node, or `None` if not recognized
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

/// Calculates scaling factor between two technology nodes.
///
/// This function computes the scaling factor needed to convert measurements
/// from one technology node to another. If either node is not recognized,
/// it returns 1.0 and prints a warning.
///
/// # Arguments
/// * `from` - Source technology node in nanometers
/// * `to` - Target technology node in nanometers
///
/// # Returns
/// Scaling factor to convert from source to target technology
///
/// # Examples
/// ```
/// use memea::scale;
///
/// let scaling_factor = scale(65, 28); // Scale from 65nm to 28nm
/// let scaled_area = original_area * scaling_factor;
/// ```
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

/// Represents a numeric range with minimum and maximum values.
///
/// This struct is commonly used for voltage ranges, parameter bounds,
/// and other min/max value pairs in memory component specifications.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    /// Minimum value of the range.
    pub min: Float,
    /// Maximum value of the range.
    pub max: Float,
}

/// Parses a range from a string containing two comma or semicolon-separated values.
///
/// # Arguments
/// * `line` - String containing two numeric values separated by comma, semicolon, or whitespace
///
/// # Returns
/// * `Ok(Range)` - Successfully parsed range
/// * `Err(MemeaError)` - Parsing error if format is invalid
///
/// # Examples
/// ```
/// use memea::parse_range;
///
/// let range = parse_range("1.2, 3.4").expect("Failed to parse range");
/// assert_eq!(range.min, 1.2);
/// assert_eq!(range.max, 3.4);
/// ```
pub fn parse_range(line: &str) -> Result<Range, MemeaError> {
    let (min, max) = parse_tuple(line)?;
    Ok(Range { min, max })
}

/// Checks whether a given path points to an existing file **and** that the file’s
/// extension matches one of the supplied allowed extensions.
///
/// The function prints a helpful error message when the check fails, then
/// returns `false`.  On success it simply returns `true`.
///
/// # Arguments
///
/// * `path`    – A reference to a `Path` that should point to the file you want to validate.
/// * `allowed` – A slice of accepted extensions **without** the leading dot (e.g. `&["txt", "md", "csv"]`).  Comparison is case‑insensitive, so `"JPG"` matches `"jpg"` on all platforms.
///
/// # Returns
///
/// * `true`  – The file exists **and** its extension is allowed.
/// * `false` – File cannot be accessed or has wrong extension
///
/// # Example
///
/// ```rust
/// use std::path::Path;
///
/// // Accept either .txt or .md files
/// let ok = check_filetype(Path::new("notes.txt"), &["txt", "md"]);
/// assert!(ok);
///
/// // Wrong extension: prints an error and returns false
/// let not_ok = check_filetype(Path::new("image.png"), &["txt", "md"]);
/// assert!(!not_ok);
/// ```
pub fn check_filetype(path: &Path, allowed: &[&str]) -> bool {
    if metadata(path).is_err() {
        errorln!("{:?} does not exist or cannot be accessed", path);
        return false;
    }

    match path.extension().and_then(OsStr::to_str) {
        Some(actual) => {
            // Normalise to lower‑case once for the comparison
            let actual_lc = actual.to_ascii_lowercase();
            if allowed.iter().any(|&e| e.eq_ignore_ascii_case(&actual_lc)) {
                true
            } else {
                errorln!(
                    "{:?} must be one of the following file types: {:?}",
                    path,
                    allowed
                );
                false
            }
        }
        None => {
            errorln!("{:?} must have a file extension", path);
            false
        }
    }
}

/// Parses a tuple of two floating-point values from a string.
///
/// This function extracts two numeric values from a string, handling various
/// separators including commas, semicolons, and whitespace.
///
/// # Arguments
/// * `line` - String containing two numeric values with separators
///
/// # Returns
/// * `Ok((a, b))` - Successfully parsed tuple of values
/// * `Err(MemeaError)` - Parsing error if format is invalid or values cannot be parsed
///
/// # Examples
/// ```
/// use memea::parse_tuple;
///
/// let (a, b) = parse_tuple("1.5; 2.7").expect("Failed to parse tuple");
/// assert_eq!((a, b), (1.5, 2.7));
/// ```
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
