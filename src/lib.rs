pub mod config;
pub mod export;
pub mod primitives;
pub mod tabulate;

use crate::config::ConfigError;
use crate::primitives::{CellType, DBError};
use thiserror::Error;

pub type Float = f32;
pub type Mosaic = (usize, usize);

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

#[derive(Debug, Error)]
pub enum ValueError {
    #[error("Expected a Value::Usize")]
    NotUsize,
    #[error("Expected a Value::Float")]
    NotFloat,
    #[error("Expected a Value::FloatVec")]
    NotFloatVec,
    #[error("Expected a Value::String")]
    NotString,
}

#[derive(Debug, Error)]
pub enum MemeaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Parse float error: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Value error: {0}")]
    Value(#[from] ValueError),

    #[error("Database error: {0}")]
    Database(#[from] DBError),
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

#[derive(Debug)]
pub struct Report {
    pub name: String,
    pub count: usize,
    pub kind: CellType,
    pub loc: String,
    pub area: Float,
}

pub type Reports = Vec<Report>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: Float,
    pub max: Float,
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Float(Float),
    Usize(usize),
    String(String),
    FloatVec(Vec<Float>),
}

#[allow(dead_code)]
impl Value {
    fn to_float(&self) -> Result<Float, ValueError> {
        match self {
            Value::Float(num) => Ok(*num),
            _ => Err(ValueError::NotFloat),
        }
    }

    fn to_usize(&self) -> Result<usize, ValueError> {
        match self {
            Value::Usize(num) => Ok(*num),
            _ => Err(ValueError::NotUsize),
        }
    }

    fn as_vec(&self) -> Result<&Vec<Float>, ValueError> {
        match self {
            Value::FloatVec(v) => Ok(v),
            _ => Err(ValueError::NotFloatVec),
        }
    }

    fn as_str(&self) -> Result<&str, ValueError> {
        match self {
            Value::String(s) => Ok(s),
            _ => Err(ValueError::NotString),
        }
    }

    fn to_string(&self) -> Result<String, ValueError> {
        match self {
            Value::String(s) => Ok(s.to_owned()),
            _ => Err(ValueError::NotString),
        }
    }
}

enum ValueTypes {
    Float,
    Usize,
    String,
    FloatVec,
}

/// Decodes string input into Value
///
/// # Arguments
/// * `input` - Value to be decoded
/// * `kind` - Data type of `input` constrained by `Target`
fn decode(input: &str, kind: ValueTypes) -> Result<Value, MemeaError> {
    match kind {
        ValueTypes::Float => {
            let val = input.parse::<Float>()?;
            Ok(Value::Float(val))
        }
        ValueTypes::Usize => {
            let val = input.parse::<usize>()?;
            Ok(Value::Usize(val))
        }
        ValueTypes::String => Ok(Value::String(input.to_owned())),
        ValueTypes::FloatVec => {
            let vals: Result<Vec<Float>, _> = input
                .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                .filter(|s| !s.trim().is_empty())
                .map(|x| x.trim().parse::<Float>())
                .collect();
            Ok(Value::FloatVec(vals?)) // ? unwraps or returns Err
        }
    }
}
