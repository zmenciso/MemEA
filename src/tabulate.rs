use std::process;

use crate::config::Config;
use crate::eliteral;
use crate::primitives::*;
use crate::{Float, Mosaic, Report, Reports};

// Drive strength multipliers
const WELL_SCALE: Float = 0.25;
const LOGIC_SCALE: Float = 0.5;

const SINGLE: Mosaic = (1, 1);

fn locate(
    condition: impl Fn(&dyn Geometry) -> bool,
    cells: &CellList,
    mos: Mosaic,
) -> (String, &dyn Geometry) {
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

    // TODO: Output which type of cell we failed to find
    match sel {
        Some(x) => (target, x),
        None => {
            eprintln!("Failed to find suitable cell");
            process::exit(4);
        }
    }
}

fn locate_type<T: 'static>(
    condition: impl Fn(&T) -> bool,
    cells: &CellList,
    mos: Mosaic,
) -> (String, &T) {
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
    );

    (name, cell.as_any().downcast_ref::<T>().unwrap())
}

fn locate_adc(fs: Float, bits: usize, adcs: &CellList, mos: Mosaic) -> (String, &ADC) {
    locate_type(|adc: &ADC| adc.fs >= fs && adc.bits >= bits, adcs, mos)
}

fn locate_logic(dx: Float, bits: usize, logics: &CellList, mos: Mosaic) -> (String, &Logic) {
    locate_type(
        |logic: &Logic| logic.dx >= dx && logic.bits >= bits,
        logics,
        mos,
    )
}

fn locate_switch(voltage: Float, dx: Float, switches: &CellList, mos: Mosaic) -> (String, &Switch) {
    locate_type(
        |switch: &Switch| switch.dx >= dx && switch.voltage >= voltage,
        switches,
        mos,
    )
}

fn locate_core(config: &Config, core: &CellList) -> (String, Core) {
    let name = config.retrieve("cell").to_string();
    let cell = core
        .get(&name)
        .expect(eliteral!("Could not find target cell"));

    match cell {
        Cell::Core(x) => (name, *x),
        _ => panic!(eliteral!("Core is not of type Cell::Core")),
    }
}

// TODO: scale as Option, only multiply if present
pub fn tabulate(config: &Config, db: &DB, scale: Float) -> Reports {
    let mut results: Reports = Vec::new();

    let n = config.retrieve("n").to_usize();
    let m = config.retrieve("m").to_usize();

    // Core area
    let mos = (n, m);
    let (name, core) = locate_core(config, db.retrieve(CellType::Core));
    let report = Report {
        name,
        count: n * m,
        kind: CellType::Core,
        loc: String::from("Array"),
        area: core.dims.area(mos) * scale,
    };
    results.push(report);

    let switches = db.retrieve(CellType::Switch);
    let logics = db.retrieve(CellType::Logic);

    // WL peripheral area
    let mos = (n, 1);
    if let Some(v) = config.get("wl") {
        let dx = n as Float * core.dx_wl;

        for voltage in v.as_vec() {
            let (target, switch) = locate_switch(*voltage, dx, switches, mos);
            let report = Report {
                name: target,
                count: n,
                kind: CellType::Switch,
                loc: String::from("WL"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
        }

        let bits = (v.as_vec().len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(dx * LOGIC_SCALE, bits, logics, mos);
        let report = Report {
            name: target,
            count: n,
            kind: CellType::Logic,
            loc: String::from("WL"),
            area: logic.dims.area(mos) * scale,
        };
        results.push(report);
    }

    // BL peripheral area
    let mos = (1, m);
    if let Some(v) = config.get("bl") {
        let dx = m as Float * core.dx_bl;

        for voltage in v.as_vec() {
            let (target, switch) = locate_switch(*voltage, dx, switches, mos);
            let report = Report {
                name: target,
                count: m,
                kind: CellType::Switch,
                loc: String::from("BL"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
        }

        let bits = (v.as_vec().len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(dx * LOGIC_SCALE, bits, logics, mos);
        let report = Report {
            name: target,
            count: m,
            kind: CellType::Logic,
            loc: String::from("BL"),
            area: logic.dims.area(mos) * scale,
        };
        results.push(report);
    }

    // Well peripheral area
    let mos = (1, m);
    if let Some(v) = config.get("well") {
        let dx = n as Float * ((core.dx_bl + core.dx_wl) / 2.0) * WELL_SCALE;

        for voltage in v.as_vec() {
            let (target, switch) = locate_switch(*voltage, dx, switches, mos);
            let report = Report {
                name: target,
                count: m,
                kind: CellType::Switch,
                loc: String::from("Well"),
                area: switch.dims.area(mos) * scale,
            };
            results.push(report);
        }

        let bits = (v.as_vec().len() as Float).log2().ceil() as usize;
        let (target, logic) = locate_logic(dx * LOGIC_SCALE, bits, logics, SINGLE);
        let report = Report {
            name: target,
            count: 1,
            kind: CellType::Logic,
            loc: String::from("Well"),
            area: logic.dims.area(SINGLE) * scale,
        };
        results.push(report);
    }

    // ADC area
    if let (Some(enob), Some(fs), Some(adcs)) =
        (config.get("enob"), config.get("fs"), config.get("adcs"))
    {
        let (enob, fs, adcs) = (enob.to_usize(), fs.to_float(), adcs.to_usize());
        let mos = (1, adcs);

        let (target, adc) = locate_adc(fs, enob, db.retrieve(CellType::ADC), mos);
        let report = Report {
            name: target,
            count: adcs,
            kind: CellType::ADC,
            loc: String::from("BL"),
            area: adc.dims.area(mos) * scale,
        };

        results.push(report);
    } else {
        eprintln!("WARNING: Missing ADC config info; ADCs will not be generated");
    }

    results
}
