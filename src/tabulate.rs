use std::process;

use crate::eliteral;
use crate::{Reports, Report, Float};
use crate::config::Config;
use crate::primitives::*;

// Drive strength multipliers
const WELL_SCALE: Float = 0.25;
const LOGIC_SCALE: Float = 0.5;

const ONE: Float = 1 as Float;

fn locate(condition: impl Fn(&dyn Geometry) -> bool, cells: &CellList) -> (String, &dyn Geometry) {
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
        }
        else if sel.is_some() && condition(cell) {
            let sel_dims = sel.unwrap().dims();
            if cell.dims().area(ONE, ONE) <= sel_dims.area(ONE, ONE) {
                (target, sel) = (name.to_owned(), Some(cell));
            }
        }
    }

    // TODO: Output which type of cell we failed to find
    match sel {
        Some(x) => (target, x),
        None => {
            eprintln!("Failed to find suitable cell");
            process::exit(4);
        }
    }
}

fn locate_type<T: 'static>(condition: impl Fn(&T) -> bool, cells: &CellList) -> (String, &T) {
    let (name, cell) = locate(
        |cell: &dyn Geometry| {
            if let Some(typed) = cell.as_any().downcast_ref::<T>() {
                condition(typed)
            } 
            else {
                false 
            }
        },
        cells,
    );

    (name, cell.as_any().downcast_ref::<T>().unwrap())

}

fn locate_adc(fs: Float, bits: Float, adcs: &CellList) -> (String, &ADC) {
    locate_type(|adc: &ADC| adc.fs >= fs && adc.bits >= bits, 
        adcs)
}

fn locate_logic(dx: Float, bits: Float, logics: &CellList) -> (String, &Logic) {
    locate_type(|logic: &Logic| logic.dx >= dx && logic.bits >= bits, 
        logics)
}

fn locate_switch(voltage: Float, dx: Float, switches: &CellList) -> (String, &Switch) {
    locate_type(|switch: &Switch| switch.dx >= dx && switch.voltage >= voltage, 
    switches)
}

fn locate_core(config: &Config, core: &CellList) -> (String, Core) {
    let name = config.retrieve("cell").to_string();
    let cell = core.get(&name)
        .expect(eliteral!("Could not find target cell"));

    match cell {
        Cell::Core(x) => (name, *x),
        _ => panic!(eliteral!("Core is not of type Cell::Core"))
    }
}

pub fn tabulate(config: &Config, db: &DB) -> Reports {
    let mut results: Reports = Vec::new();

    let n = config.retrieve("n").to_f32();
    let m = config.retrieve("m").to_f32();

    // Core area
    let (name, core) = locate_core(config, db.retrieve(CellType::Core));
    let report = Report {
        name,
        kind: CellType::Core,
        loc: String::from("Array"),
        area: core.dims.area(n, m)
    };
    results.push(report);

    let switches = db.retrieve(CellType::Switch);
    let logics = db.retrieve(CellType::Logic);

    // WL peripheral area
    if let Some(v) = config.get("wl") {
        let dx = n * core.dx_wl;

        for voltage in v.as_vec() {
            let (target, switch) = locate_switch(*voltage, dx, switches);
            let report = Report {
                name: target,
                kind: CellType::Switch,
                loc: String::from("WL"),
                area: switch.dims.area(n, ONE)
            };
            results.push(report);
        }

        let bits = (v.as_vec().len() as Float).log2().ceil();
        let (target, logic) = locate_logic(dx*LOGIC_SCALE, bits, logics);
        let report = Report {
            name: target,
            kind: CellType::Logic,
            loc: String::from("WL"),
            area: logic.dims.area(n, ONE)
        };
        results.push(report);
    }

    // BL peripheral area
    if let Some(v) = config.get("bl") {
        let dx = m * core.dx_bl;

        for voltage in v.as_vec() {
            let (target, switch) = locate_switch(*voltage, dx, switches);
            let report = Report {
                name: target,
                kind: CellType::Switch,
                loc: String::from("BL"),
                area: switch.dims.area(ONE, m)
            };
            results.push(report);
        }

        let bits = (v.as_vec().len() as Float).log2().ceil();
        let (target, logic) = locate_logic(dx*LOGIC_SCALE, bits, logics);
        let report = Report {
            name: target,
            kind: CellType::Logic,
            loc: String::from("BL"),
            area: logic.dims.area(ONE, m)
        };
        results.push(report);
    }

    // Well peripheral area
    if let Some(v) = config.get("well") {
        let dx = n * (core.dx_bl + core.dx_wl) / 2.0 * WELL_SCALE;

        for voltage in v.as_vec() {
            let (target, switch) = locate_switch(*voltage, dx, switches);
            let report = Report {
                name: target,
                kind: CellType::Switch,
                loc: String::from("Well"),
                area: switch.dims.area(ONE, m)
            };
            results.push(report);
        }

        let bits = (v.as_vec().len() as Float).log2().ceil();
        let (target, logic) = locate_logic(dx*LOGIC_SCALE, bits, logics);
        let report = Report {
            name: target,
            kind: CellType::Logic,
            loc: String::from("Well"),
            area: logic.dims.area(ONE, ONE)
        };
        results.push(report);
    }

    // ADC area
    let enob = config.get("enob");
    let fs = config.get("fs");
    let adcs = config.get("adcs");

    if enob.is_some() && fs.is_some() && adcs.is_some() {
        let (target, adc) = locate_adc(fs.unwrap().to_f32(), enob.unwrap().to_f32(), db.retrieve(CellType::ADC));
        let report = Report {
            name: target,
            kind: CellType::ADC,
            loc: String::from("Array"),
            area: adc.dims.area(ONE, adcs.unwrap().to_f32())
        };
        results.push(report);
    }
    else {
        eprintln!("WARNING: Missing ADC config info; ADCs will not be generated");
    }

    results
}
