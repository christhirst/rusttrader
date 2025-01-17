use std::error::Error;
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("indicator_descriptor.bin"))
        .compile(&["proto/indicators.proto"], &["proto"])?;
    tonic_build::compile_protos("proto/indicators.proto")?;

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("plot_descriptor.bin"))
        .compile(&["proto/plot.proto"], &["proto"])?;
    tonic_build::compile_protos("proto/plot.proto")?;

    Ok(())
}
