mod comobj;

use anyhow::Result;

use crate::comobj::ComLibrary;

fn main() -> Result<()> {
    let _com = ComLibrary::new();
    println!("Hello, world!");
    Ok(())
}
