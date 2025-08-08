pub mod config;
pub mod export;
pub mod primitives;
pub mod tabulate;

use crate::primitives::CellType;

pub type Float = f32;
pub type Mosaic = (usize, usize);

#[macro_export]
macro_rules! eliteral {
    ($literal:expr) => {
        concat!("\x1b[31mERROR: ", $literal, "\x1b[0m")
    };
}

#[macro_export]
macro_rules! errorln {
    ($literal:literal $(, $args:expr)* $(,)?) => {
        eprintln!(
            concat!("\\x1b[31mERROR: ", $literal, "\\x1b[0m")
            $(, $args)*
        )
    };
}

#[macro_export]
macro_rules! warnln {
    ($literal:literal $(, $args:expr)* $(,)?) => {
        eprintln!(
            concat!("\\x1b[33mWARNING: ", $literal, "\\x1b[0m")
            $(, $args)*
        )
    };
}

#[macro_export]
macro_rules! infoln {
    ($literal:literal $(, $args:expr)* $(,)?) => {
        eprintln!(
            concat!("\\x1b[32mINFO: ", $literal, "\\x1b[0m")
            $(, $args)*
        )
    };
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

#[derive(Debug, PartialEq)]
pub enum Value {
    Float(Float),
    Usize(usize),
    String(String),
    FloatVec(Vec<Float>),
}

#[allow(dead_code)]
impl Value {
    fn to_float(&self) -> Float {
        match self {
            Value::Float(num) => *num,
            _ => panic!(eliteral!("Expected a Value::Float")),
        }
    }

    fn to_usize(&self) -> usize {
        match self {
            Value::Usize(num) => *num,
            _ => panic!(eliteral!("Expected a Value::Usize")),
        }
    }

    fn as_vec(&self) -> &Vec<Float> {
        match self {
            Value::FloatVec(v) => v,
            _ => panic!(eliteral!("Expected a Value::FloatVec")),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Value::String(s) => s,
            _ => panic!(eliteral!("Expected a Value::String")),
        }
    }

    fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.to_owned(),
            _ => panic!(eliteral!("Expected a Value::String")),
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
///
/// # Panics
/// Incorrect `kind` for `input`
fn decode(input: &str, kind: ValueTypes) -> Value {
    match kind {
        ValueTypes::Float => Value::Float(parse_float(input)),
        ValueTypes::Usize => Value::Usize(parse_usize(input)),
        ValueTypes::String => Value::String(input.to_owned()),
        ValueTypes::FloatVec => Value::FloatVec(
            input
                .split(',')
                .map(|x| {
                    x.trim()
                        .parse::<Float>()
                        .expect(eliteral!("Could not parse float"))
                })
                .collect(),
        ),
    }
}

fn parse_float(input: &str) -> Float {
    input
        .parse::<Float>()
        .expect(eliteral!("Could not parse float"))
}

fn parse_usize(input: &str) -> usize {
    input
        .parse::<usize>()
        .expect(eliteral!("Could not parse usize"))
}
