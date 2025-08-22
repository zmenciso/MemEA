use gds21::{GdsElement, GdsLibrary};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::db::Dims;
use crate::{errorln, vprintln, Float, MemeaError};

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
    verbose: bool,
) -> Result<(Float, Float), MemeaError> {
    if elems.is_empty() {
        errorln!("No geometry data for cell; cannot compute enclosure.");
        return Ok((0.0, 0.0));
    }

    let mut boundaries: usize = 0;
    let mut layers = HashSet::new();

    let mut iter = elems
        .iter()
        .filter_map(|elem| {
            if let GdsElement::GdsBoundary(b) = elem {
                boundaries += 1;
                layers.insert(b.layer);
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

    let scale = units as f32 / 1e-6;
    let (span_x, span_y) = (
        (max_x - min_x) as f32 * scale,
        (max_y - min_y) as f32 * scale,
    );
    let (enc_x, enc_y) = ((span_x - w) / 2.0, (span_y - h) / 2.0);

    vprintln!(
        verbose,
        "Computed enclosure [{:.4}, {:.4}] from {} polygons across {} layers",
        enc_x,
        enc_y,
        boundaries,
        layers.len()
    );

    Ok((enc_x as Float, enc_y as Float))
}

pub fn augment_dims(
    map: &HashMap<String, Vec<GdsElement>>,
    cell: &str,
    w: Float,
    h: Float,
    units: f64,
    verbose: bool,
) -> Result<Dims, MemeaError> {
    // Lookup cell
    if let Some(elems) = map.get(cell) {
        let (enc_x, enc_y) = compute_enc(elems, w, h, units, verbose)?;
        Ok(Dims::from(w, h, enc_x, enc_y))
    } else {
        errorln!(
            "Could not find matching cell {} in GDS database; cannot compute enclosure",
            cell
        );
        Ok(Dims::from(w, h, 0.0, 0.0))
    }
}
