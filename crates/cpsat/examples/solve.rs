//! Solves a small model. CI runs this example from a directory that holds only
//! the executable and the copied runtime libraries, as a shipped binary would.

use cpsat::Result;
use cpsat::prelude::*;
use cpsat::proto::SatParameters;

fn main() -> Result<()> {
    let mut model = CpModel::new();
    let x = model.new_var(0..=10);
    let y = model.new_var(0..=10);
    model.add((x + 2 * y).le(14));
    model.maximize(x + y);

    let response = model.solve(&SatParameters::default())?;
    println!(
        "{}: x + y = {:?}",
        response.status(),
        response.solution().map(|s| s.value(x + y))
    );
    Ok(())
}
