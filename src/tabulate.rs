use crate::db::*;
use crate::warnln;
use crate::MemeaError;

use crate::config::Config;
use crate::{Float, Mosaic, Report, Reports};

// Drive strength multipliers
const WELL_SCALE: Float = 0.25;
const LOGIC_SCALE: Float = 0.5;

const SINGLE: Mosaic = (1, 1);

fn locate(
    condition: impl Fn(&dyn Geometry) -> bool,
    cells: &CellList,
    mos: Mosaic,
    kind: CellType,
) -> Result<(String, &dyn Geometry), MemeaError> {
    let mut target = String::new();
    let mut sel: Option<&dyn Geometry> = None;

    for (name, cell) in cells.iter() {
        let cell: &dyn Geometry = match cell {
            Cell::ADC(x) => x,
            Cell::Logic(x) => x,
            Cell::Switch(x) => x,
            _ => continue,
        };

        if sel.is_none() && condition(cell) {
            (target, sel) = (name.to_owned(), Some(cell));
        } else if sel.is_some() && condition(cell) {
            let sel_dims = sel.unwrap().dims();
            if cell.dims().area(mos) <= sel_dims.area(mos) {
                (target, sel) = (name.to_owned(), Some(cell));
            }
        }
    }

    // TODO: Provide details on what we were trying to find
    match sel {
        Some(x) => Ok((target, x)),
        None => Err(DBError::NoSuitableCells(kind).into()),
    }
}

fn locate_type<T: 'static>(
    condition: impl Fn(&T) -> bool,
    cells: &CellList,
    mos: Mosaic,
    kind: CellType,
) -> Result<(String, &T), MemeaError> {
    let (name, cell) = locate(
        |cell: &dyn Geometry| {
            if let Some(typed) = cell.as_any().downcast_ref::<T>() {
                condition(typed)
            } else {
                false
            }
        },
        cells,
        mos,
        kind,
    )?;

    Ok((name, cell.as_any().downcast_ref::<T>().unwrap()))
}

fn locate_adc(
    fs: Float,
    bits: usize,
    adcs: &CellList,
    mos: Mosaic,
) -> Result<(String, &ADC), MemeaError> {
    locate_type(
        |adc: &ADC| adc.fs >= fs && adc.bits >= bits,
        adcs,
        mos,
        CellType::ADC,
    )
}

fn locate_logic(
    dx: Float,
    bits: usize,
    logics: &CellList,
    mos: Mosaic,
) -> Result<(String, &Logic), MemeaError> {
    locate_type(
        |logic: &Logic| logic.dx >= dx && logic.bits >= bits,
        logics,
        mos,
        CellType::Logic,
    )
}

fn locate_switch(
    voltage: Float,
    dx: Float,
    switches: &CellList,
    mos: Mosaic,
) -> Result<(String, &Switch), MemeaError> {
    locate_type(
        |switch: &Switch| {
            switch.dx >= dx && voltage >= switch.voltage.min && voltage <= switch.voltage.max
        },
        switches,
        mos,
        CellType::Switch,
    )
}

fn locate_core(config: &Config, core: &CellList) -> Result<(String, Core), MemeaError> {
    let name = &config.cell;
    let cell = core.get(name).ok_or(DBError::MissingCell(name.clone()))?;

    match cell {
        Cell::Core(x) => Ok((name.to_string(), *x)),
        _ => Err(DBError::InvalidCellType(CellType::Core).into()),
    }
}

// TODO: scale as Option, only multiply if present
pub fn tabulate(id: &str, config: &Config, db: &DB, scale: Float) -> Result<Reports, MemeaError> {
    let mut results: Reports = Vec::new();

    // Core area
    let mos = (config.n, config.m);
    let (name, core) = locate_core(config, db.retrieve(CellType::Core)?)?;
    let report = Report {
        name,
        count: config.n * config.m,
        kind: CellType::Core,
        loc: String::from("Array"),
        area: core.dims.area(mos) * scale,
    };
    results.push(report);

    let switches = db.retrieve(CellType::Switch)?;
    let logics = db.retrieve(CellType::Logic)?;

    // WL peripheral area
    let mos = (config.n, 1);
    if let Some(v) = &config.wl {
        let dx = config.n as Float * core.dx_wl;

        for voltage in v {
            let (target, switch) = locate_switch(*voltage, dx, switches, mos)?;
            let report = Report {
                name: target,
                count: config.n,
                kind: CellType::Switch,
                loc: String::from("WL"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
        }

        let bits = (v.len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(dx * LOGIC_SCALE, bits, logics, mos)?;
        let report = Report {
            name: target,
            count: config.n,
            kind: CellType::Logic,
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
        let dx = config.m as Float * core.dx_bl;

        for voltage in v {
            let (target, switch) = locate_switch(*voltage, dx, switches, mos)?;
            let report = Report {
                name: target,
                count: config.m,
                kind: CellType::Switch,
                loc: String::from("BL"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
        }

        let bits = (v.len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(dx * LOGIC_SCALE, bits, logics, mos)?;
        let report = Report {
            name: target,
            count: config.m,
            kind: CellType::Logic,
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
    if let Some(v) = &config.well {
        let dx = config.n as Float * ((core.dx_bl + core.dx_wl) / 2.0) * WELL_SCALE;

        for voltage in v {
            let (target, switch) = locate_switch(*voltage, dx, switches, mos)?;
            let report = Report {
                name: target,
                count: config.m,
                kind: CellType::Switch,
                loc: String::from("Well"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
        }

        let bits = (v.len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(dx * LOGIC_SCALE, bits, logics, SINGLE)?;
        let report = Report {
            name: target,
            count: 1,
            kind: CellType::Logic,
            loc: String::from("Well"),
            area: logic.dims.area(SINGLE) * scale,
        };
        results.push(report);
    } else {
        warnln!(
            "No 'well' key supplied, skipping well drivers for config {}",
            id
        )
    }

    // ADC area
    if let (Some(enob), Some(fs), Some(adcs)) = (config.enob, config.fs, config.adcs) {
        let (enob, fs, adcs) = (enob, fs, adcs);
        let mos = (1, adcs);

        let (target, adc) = locate_adc(fs, enob, db.retrieve(CellType::ADC)?, mos)?;
        let report = Report {
            name: target,
            count: adcs,
            kind: CellType::ADC,
            loc: String::from("BL"),
            area: adc.dims.area(mos) * scale,
        };

        results.push(report);
    } else {
        warnln!(
            "Missing ADC config info for {} (expecting 'enob', 'fs', and 'adcs'); ADCs will not be generated", 
            id
        );
    }

    Ok(results)
}
