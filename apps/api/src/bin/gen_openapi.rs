use std::{fs::File, io::BufWriter, path::Path};

fn main() -> anyhow::Result<()> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("openapi.json");
    let file = File::create(&path)?;
    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, &api::generate_openapi())?;
    println!("generated {}", path.display());

    Ok(())
}
