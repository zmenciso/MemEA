use gds21::{GdsElement, GdsLibrary};

pub fn read(filename: &str, layer: Option<i16>) -> Result<(), gds21::GdsError> {
    let lib = GdsLibrary::load(filename)?;

    let units = lib.units.db_unit();

    let structs = lib.structs;

    for cell in structs {
        println!("Cell name: {}", cell.name);

        for elem in cell.elems {
            if let GdsElement::GdsBoundary(b) = elem {
                println!(
                    "Layer {} boundary ({}) ({} points) BOUNDARY",
                    b.layer,
                    b.datatype,
                    b.xy.len()
                );
                for point in b.xy {
                    println!("  ({}, {})", point.x, point.y);
                }
            }
        }
    }

    // TODO: Largest of the 5-point layer is the area?
    // TODO: User-specified layer

    Ok(())
}
