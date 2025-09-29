//! GDS layout file processing for MemEA component enclosure calculation.
//!
//! This module provides functionality to parse GDS layout files, inspect all
//! layers, and calculate enclosure size based on the relative difference
//! between the cell footprint and PR boundary.
use gds21::{GdsElement, GdsLibrary};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::db::Dims;
use crate::{errorln, vprintln, Float, MemeaError};

/// Errors that can occur during GDS layout processing.
#[derive(Debug, Error)]
pub enum GdsError {
    /// Indicates that a requested cell was not found in the GDS library.
    #[error("Cell not found: {0}")]
    InvalidCell(String),
    /// Indicates that a GDS element contains no geometry data.
    #[error("Inspected element is empty: {0}")]
    EmptyElement(String),
}

/// Creates a hashmap of GDS library cells indexed by name for fast lookup.
///
/// This function transforms a GDS library into a HashMap where each cell name
/// maps to its corresponding vector of geometric elements, enabling efficient
/// cell lookup during dimension computation.
///
/// # Arguments
/// * `lib` - GDS library containing cell structures and elements
///
/// # Returns
/// HashMap mapping cell names to their geometric elements
///
/// # Examples
/// ```no_run
/// use gds21::GdsLibrary;
/// use memea::gds::hash_lib;
///
/// let library = GdsLibrary::load("layout.gds").expect("Failed to load GDS");
/// let cell_map = hash_lib(library);
/// println!("Loaded {} cells from GDS", cell_map.len());
/// ```
pub fn hash_lib(lib: GdsLibrary) -> HashMap<String, Vec<GdsElement>> {
    // Hash cells by name for fast lookup
    lib.structs.into_iter().map(|s| (s.name, s.elems)).collect()
}

/// Computes enclosure requirements from GDS geometry elements.
///
/// This function analyzes the boundary polygons in a GDS cell to determine
/// the enclosure margins needed around the core component dimensions. It
/// calculates the bounding box of all geometry and computes the difference
/// between the total span and the core dimensions.
///
/// # Arguments
/// * `elems` - Vector of GDS elements containing boundary polygons
/// * `w` - Core component width in micrometers
/// * `h` - Core component height in micrometers
/// * `units` - GDS unit conversion factor (database units to meters)
/// * `verbose` - Whether to print detailed computation information
///
/// # Returns
/// * `Ok((enc_x, enc_y))` - Horizontal and vertical enclosure margins
/// * `Err(MemeaError)` - Error if no valid geometry is found
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

/// Augments component dimensions with enclosure data from GDS layout.
///
/// This function looks up a cell in the GDS library hashmap and computes
/// the required enclosure margins by analyzing the cell's geometry. It
/// returns a complete `Dims` structure with both core dimensions and
/// enclosure requirements.
///
/// # Arguments
/// * `map` - HashMap of cell names to GDS elements (from `hash_lib`)
/// * `cell` - Name of the cell to analyze
/// * `w` - Core component width in micrometers
/// * `h` - Core component height in micrometers
/// * `units` - GDS unit conversion factor
/// * `verbose` - Whether to show detailed computation output
///
/// # Returns
/// * `Ok(Dims)` - Complete dimensions with enclosure data
/// * `Err(MemeaError)` - Error during geometry analysis
///
/// # Examples
/// ```no_run
/// use memea::gds::{hash_lib, augment_dims};
/// use gds21::GdsLibrary;
///
/// let library = GdsLibrary::load("cells.gds").expect("Failed to load GDS");
/// let cell_map = hash_lib(library);
/// let units = 1e-9; // 1 nm database units
///
/// let dims = augment_dims(&cell_map, "sram_6t", 0.5, 0.8, units, true)
///     .expect("Failed to compute dimensions");
/// println!("Cell area: {:.2} μm²", dims.area((1, 1)));
/// ```
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
