use gds21::{GdsElement, GdsLibrary};
use std::collections::HashMap;
use thiserror::Error;

use crate::db::Dims;
use crate::{errorln, Float, MemeaError};

#[derive(Debug, Error)]
pub enum GdsError {
    #[error("Cell not found: {0}")]
    InvalidCell(String),
    #[error("Inspected element is empty: {0}")]
    EmptyElement(String),
}

pub fn hash_lib(lib: GdsLibrary) -> HashMap<String, Vec<GdsElement>> {
    // Hash cells by name for fast lookup
    lib.structs.into_iter().map(|s| (s.name, s.elems)).collect()
}

fn compute_enc(
    elems: &Vec<GdsElement>,
    w: Float,
    h: Float,
    units: f64,
) -> Result<(Float, Float), MemeaError> {
    if elems.is_empty() {
        errorln!("No geometry data for cell; cannot compute enclosure.");
        return Ok((0.0, 0.0));
    }

    let mut iter = elems
        .iter()
        .filter_map(|elem| {
            if let GdsElement::GdsBoundary(b) = elem {
                Some(b.xy.iter())
            } else {
                None
            }
        })
        .flatten();

    let first = iter
        .next()
        .ok_or(GdsError::EmptyElement(format!("{elems:?}")))?;
    let mut min_x = first.x;
    let mut max_x = first.x;
    let mut min_y = first.y;
    let mut max_y = first.y;

    for p in iter.skip(1) {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    let scale = units / 1e-6;
    let enc_x = ((max_x - min_x) as f64 - w as f64) * scale;
    let enc_y = ((max_y - min_y) as f64 - h as f64) * scale;

    Ok((enc_x as Float, enc_y as Float))
}

pub fn augment_dims(
    map: &HashMap<String, Vec<GdsElement>>,
    cell: &str,
    w: Float,
    h: Float,
    units: f64,
) -> Result<Dims, MemeaError> {
    // Lookup cell
    if let Some(elems) = map.get(cell) {
        println!("Cell {} has {} elements", cell, elems.len());
        let (enc_x, enc_y) = compute_enc(elems, w, h, units)?;
        Ok(Dims::from(w, h, enc_x, enc_y))
    } else {
        errorln!(
            "Could not find matching cell {} in GDS database; cannot compute enclosure",
            cell
        );
        Ok(Dims::from(w, h, 0.0, 0.0))
    }
}
