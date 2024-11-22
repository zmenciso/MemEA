use std::collections::HashMap;
use std::path::PathBuf;
use std::error::Error;

use crate::config::Config;
use crate::primitives::{Cell, DB};

const WELL_SCALE: f32 = 0.25;
const LOGIC_SCALE: f32 = 0.5;

fn locate_driver(voltage: f32, dx: f32, switches: &HashMap<String, Cell>) -> (String, Option<&Cell>) {
    let mut target = String::from("");
    let mut driver: Option<&Cell> = None;

    for (name, cell) in switches.into_iter() {
        // If we don't have a target, we take the first valid switch
        if (target.len() == 0) && (cell.dx >= dx) && (cell.voltage >= voltage) {
            target = format!("{}", name);
            driver = Some(cell);
        }
        // Otherwise we check to see if this switch is better suited
        else if (target.len() > 0) && ((cell.dx >= dx) && (cell.dx < driver.unwrap().dx)) && ((cell.voltage >= voltage) && (cell.voltage < driver.unwrap().voltage)) && (cell.area(1,1) <= driver.unwrap().area(1,1)) {
            driver = Some(cell);
        }
    }

    if driver.is_none() {
        eprintln!("ERROR: Failed to find suitable switch for voltage {} and drive strength {}", voltage, dx);
        std::process::exit(4);
    }

    (target, driver)
}

fn locate_adc(fs: f32, bits: i32, adcs: &HashMap<String, Cell>) -> (String, Option<&Cell>) {
    let mut target = String::from("");
    let mut adc: Option<&Cell> = None;

    for (name, cell) in adcs.into_iter() {
        // If we don't have a target, we take the first valid adc
        if (target.len() == 0) && (cell.fs >= fs) && (cell.bits >= bits) {
            target = format!("{}", name);
            adc = Some(cell);
        }
        // Otherwise we check to see if this adc is better suited
        else if (target.len() > 0) && ((cell.fs >= fs) && (cell.fs < adc.unwrap().fs)) && ((cell.bits >= bits) && (cell.bits < adc.unwrap().bits)) && (cell.area(1,1) <= adc.unwrap().area(1,1)) {
            adc = Some(cell);
        }
    }

    if adc.is_none() {
        eprintln!("ERROR: Failed to find suitable {}-bit adc with fs={}", bits, fs);
        std::process::exit(4);
    }

    (target, adc)
}

fn locate_logic(dx: f32, bits: i32, logic: &HashMap<String, Cell>) -> (String, Option<&Cell>) {
    let mut target = String::from("");
    let mut driver: Option<&Cell> = None;

    for (name, cell) in logic.into_iter() {
        // If we don't have a target, we take the first valid logic
        if (target.len() == 0) && (cell.bits >= bits) && (cell.dx >= dx) {
            target = format!("{}", name);
            driver = Some(cell);
        }
        // Otherwise we check to see if this logic is better suited
        else if (target.len() > 0) && ((cell.dx >= dx) && (cell.dx < driver.unwrap().dx)) && ((cell.bits >= bits) && (cell.bits < driver.unwrap().bits)) && (cell.area(1,1) <= driver.unwrap().area(1,1)) {
            driver = Some(cell);
        }
    }

    if driver.is_none() {
        eprintln!("ERROR: Failed to find suitable switch for driver logic with {} bits and drive strength {}", bits, dx);
        std::process::exit(4);
    }

    (target, driver)
}

pub fn tabulate(config: Config, db: &DB) -> Result<HashMap<String, f32>, Box<dyn Error>> {
    let mut results: HashMap<String, f32> = HashMap::new();

    // Get cell and compute area
    let arr_cell = match db.cells.get(&config.cell) {
        Some(x) => {
            results.insert(format!("CELL {}",config.cell), x.area(config.n, config.m));
            x
        }
        None => {
            eprintln!("ERROR: cell {} not found", config.cell);
            std::process::exit(3);
        }
    };

    // Compute required dx
    let dx_bl = config.n as f32 * arr_cell.dx;
    let dx_wl = config.m as f32 * arr_cell.dx;
    let dx_well = dx_wl * WELL_SCALE;

    let bits_wl = (config.wl.len() as f32).log2().ceil() as i32;
    let bits_bl = (config.bl.len() as f32).log2().ceil() as i32;

    // Get WL drivers and compute area
    for voltage in config.wl.into_iter() {
        let (target, driver) = locate_driver(voltage, dx_wl, &db.switches);
        results.insert(format!("WL   {}", target), driver.unwrap().area(config.n, 1));
    }

    // Get WL logic area
    let (target, driver) = locate_logic(dx_wl/2.0, bits_wl, &db.logic);
    results.insert(format!("WL   {}", target), driver.unwrap().area(config.n, 1));

    // Get BL drivers and compute area
    for voltage in config.bl.into_iter() {
        let (target, driver) = locate_driver(voltage, dx_bl, &db.switches);
        results.insert(format!("BL   {}", target), driver.unwrap().area(1, config.m));
    }
    
    // Get BL logic area
    let (target, driver) = locate_logic(dx_bl * LOGIC_SCALE, bits_bl, &db.logic);
    results.insert(format!("BL   {}", target), driver.unwrap().area(1, config.m));

    // Get well drivers and compute area
    for voltage in config.well.into_iter() {
        let (target, driver) = locate_driver(voltage, dx_well, &db.switches);
        results.insert(format!("WELL {}", target), driver.unwrap().area(1, config.m));
    }

    // Get ADC area
    let mm_n = config.enob < 0.0 || config.fs < 0.0 || config.adcs < 0;
    let mm_na = !(config.enob < 0.00 && config.fs < 0.0 && config.adcs < 0);
    if (config.enob > 0.0) && (config.fs > 0.0) && (config.adcs > 0) {
        let (target, adc) = locate_adc(config.fs, config.enob.ceil() as i32, &db.adcs);
        results.insert(format!("ADC  {}", target), adc.unwrap().area(1, config.adcs));
    }
    else if mm_n && mm_na {
        println!("WARNING: ADC configuration error; ADCs will not be generated");
    }

    Ok(results)
}
