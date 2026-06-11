use serde::Serialize;

use crate::config::Config;
use crate::db::*;
use crate::{warnln, Float, MemeaError, Mosaic};

// Logic with drive strength DX can drive switches with total drive DX * DX_SCALE
const DX_SCALE: Float = 3.0;

const SINGLE: Mosaic = (1, 1);

#[derive(Debug, Serialize)]
pub struct Report {
    pub name: String,
    pub count: usize,
    pub celltype: CellType,
    pub loc: String,
    pub area: Float,
}

pub type Reports = Vec<Report>;

fn locate_logic(
    db: &Database,
    dx: Float,
    bits: usize,
    mos: Mosaic,
) -> Result<(String, Logic), DBError> {
    let mut target = String::new();
    let mut sel: Option<&Logic> = None;

    for (name, logic) in &db.logic {
        let condition = || -> bool { logic.dx * DX_SCALE >= dx && logic.bits >= bits };

        if sel.is_none() && condition() {
            (target, sel) = (name.clone(), Some(logic));
        } else if sel.is_some() && condition() {
            let dims = sel.unwrap().dims;
            if logic.dims.area(mos) <= dims.area(mos) {
                (target, sel) = (name.clone(), Some(logic))
            }
        }
    }

    match sel {
        Some(x) => Ok((target, *x)),
        None => Err(DBError::NoSuitableCells(format!(
            "Logic with dx {dx} and {bits} bits"
        ))),
    }
}

fn locate_adc(
    db: &Database,
    fs: Float,
    bits: usize,
    mos: Mosaic,
) -> Result<(String, ADC), DBError> {
    let mut target = String::new();
    let mut sel: Option<&ADC> = None;

    for (name, adc) in &db.adc {
        let condition = || -> bool { adc.fs >= fs && adc.enob >= bits as Float };

        if sel.is_none() && condition() {
            (target, sel) = (name.clone(), Some(adc));
        } else if sel.is_some() && condition() {
            let dims = sel.unwrap().dims;
            if adc.dims.area(mos) <= dims.area(mos) {
                (target, sel) = (name.clone(), Some(adc))
            }
        }
    }

    match sel {
        Some(x) => Ok((target, *x)),
        None => Err(DBError::NoSuitableCells(format!(
            "ADC with fs {fs} and {bits} bits"
        ))),
    }
}

fn locate_switch(
    db: &Database,
    voltage: Float,
    res: Float,
    mos: Mosaic,
) -> Result<(String, Switch), DBError> {
    let mut target = String::new();
    let mut sel: Option<&Switch> = None;

    for (name, switch) in &db.switch {
        let condition = || -> bool {
            switch.res <= res && voltage >= switch.voltage[0] && voltage <= switch.voltage[1]
        };

        if sel.is_none() && condition() {
            (target, sel) = (name.clone(), Some(switch));
        } else if sel.is_some() && condition() {
            let dims = sel.unwrap().dims;
            if switch.dims.area(mos) <= dims.area(mos) {
                (target, sel) = (name.clone(), Some(switch))
            }
        }
    }

    match sel {
        Some(x) => Ok((target, *x)),
        None => Err(DBError::NoSuitableCells(format!(
            "Switch for voltage {voltage} and ON resistance {res}"
        ))),
    }
}

fn locate_core<'a>(
    config: &'a Config,
    db: &'a Database,
) -> Result<(&'a String, &'a Core), MemeaError> {
    let name = &config.cell;
    let cell = db
        .core
        .get(name)
        .ok_or(DBError::MissingCell(name.clone()))?;

    Ok((name, cell))
}

pub fn tabulate(
    id: &str,
    config: &Config,
    db: &Database,
    scale: Float,
) -> Result<Reports, MemeaError> {
    let mut results: Reports = Vec::new();

    // Core area
    let mos = (config.n, config.m);
    let (name, core) = locate_core(config, db)?;
    let report = Report {
        name: name.clone(),
        count: config.n * config.m,
        celltype: CellType::Core,
        loc: String::from("Array"),
        area: core.dims.area(mos) * scale,
    };
    results.push(report);

    // WL peripheral area
    let mos = (config.n, 1);
    if let Some(v) = &config.wl {
        let res: Float = 1.0 / (config.n as Float * core.cap_wl * config.fs);
        let mut dx: Float = 0.0;

        for voltage in v {
            let (target, switch) = locate_switch(db, *voltage, res, mos)?;
            let report = Report {
                name: target,
                count: config.n,
                celltype: CellType::Switch,
                loc: String::from("WL"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
            dx += switch.dx;
        }

        let bits = (v.len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(db, dx, bits, mos)?;
        let report = Report {
            name: target,
            count: config.n,
            celltype: CellType::Logic,
            loc: String::from("WL"),
            area: logic.dims.area(mos) * scale,
        };
        results.push(report);
    } else {
        warnln!(
            "No 'wl' key supplied, skipping wordline drivers for config {}",
            id
        )
    }

    // BL peripheral area
    let mos = (1, config.m);
    if let Some(v) = &config.bl {
        let res: Float = 1.0 / (config.m as Float * core.cap_bl * config.fs);
        let mut dx: Float = 0.0;

        for voltage in v {
            let (target, switch) = locate_switch(db, *voltage, res, mos)?;
            let report = Report {
                name: target,
                count: config.m,
                celltype: CellType::Switch,
                loc: String::from("BL"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
            dx += switch.dx;
        }

        let bits = (v.len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(db, dx, bits, mos)?;
        let report = Report {
            name: target,
            count: config.m,
            celltype: CellType::Logic,
            loc: String::from("BL"),
            area: logic.dims.area(mos) * scale,
        };
        results.push(report);
    } else {
        warnln!(
            "No 'bl' key supplied, skipping bitline drivers for config {}",
            id
        )
    }

    // Well peripheral area
    let mos = (1, config.m);
    if let (Some(v), Some(cap)) = (&config.well, core.cap_well) {
        let res: Float = 1.0 / (cap * config.n as Float * config.m as Float * config.fs);
        let mut dx: Float = 0.0;

        for voltage in v {
            let (target, switch) = locate_switch(db, *voltage, res, mos)?;
            let report = Report {
                name: target,
                count: config.m,
                celltype: CellType::Switch,
                loc: String::from("Well"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
            dx += switch.dx;
        }

        let bits = (v.len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(db, dx, bits, SINGLE)?;
        let report = Report {
            name: target,
            count: 1,
            celltype: CellType::Logic,
            loc: String::from("Well"),
            area: logic.dims.area(SINGLE) * scale,
        };
        results.push(report);
    } else {
        warnln!(
            "No 'well' key or well capacitance supplied, skipping well drivers for config {}",
            id
        )
    }

    // ADC area
    if let (Some(bits), Some(adcs)) = (config.bits, config.adcs) {
        let mos = (1, adcs);

        let (target, adc) = locate_adc(db, config.fs, bits, mos)?;
        let report = Report {
            name: target,
            count: adcs,
            celltype: CellType::ADC,
            loc: String::from("BL"),
            area: adc.dims.area(mos) * scale,
        };

        results.push(report);
    } else {
        warnln!(
            "Missing ADC config info for {} (expecting 'bits' and 'adcs'); ADCs will not be generated",
            id
        );
    }

    Ok(results)
}
