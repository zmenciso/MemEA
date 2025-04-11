pub mod config;
pub mod primitives;
pub mod tabulate;
pub mod export;

use crate::primitives::CellType;

pub type Float = f32;
pub type Mosaic = (usize, usize);

#[macro_export]
macro_rules! eliteral {
    ($literal:expr) => {
        concat!("\x1b[31mERROR: ", $literal, "\x1b[0m")
    };
}

#[derive (Debug)]
pub struct Report {
    pub name: String,
    pub count: usize,
    pub kind: CellType,
    pub loc: String,
    pub area: f32
}

pub type Reports = Vec<Report>;

#[derive (Debug, PartialEq)]
pub enum Value {
    Float(f32),
    Usize(usize),
    String(String),
    FloatVec(Vec<f32>),
}

#[allow(dead_code)]
impl Value {
    fn to_f32(&self) -> f32 {
        match self {
            Value::Float(num) => *num,
            _ => panic!(eliteral!("Expected a Value::Float")),
        }
    }

    fn to_usize(&self) -> usize {
        match self {
            Value::Usize(num) => *num,
            _ => panic!(eliteral!("Expected a Value::Usize"))
        }
    }

    fn as_vec(&self) -> &Vec<f32> {
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
    FloatVec
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
        ValueTypes::FloatVec => Value::FloatVec(input.split(',')
            .map(|x| x.trim().parse::<Float>()
                .expect(eliteral!("Could not parse float")))
            .collect()),
    }
}

fn parse_float(input: &str) -> Float {
    input.parse::<Float>()
        .expect(eliteral!("Could not parse float"))
}

fn parse_usize(input: &str) -> usize {
    input.parse::<usize>()
        .expect(eliteral!("Could not parse usize"))
}
