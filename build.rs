use std::error::Error;
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .type_attribute(
            ".proto",
            "#[derive(serde::Deserialize)] #[serde(rename_all = \"snake_case\")]",
        )
        .file_descriptor_set_path(out_dir.join("indicator_descriptor.bin"))
        .compile_protos(&["proto/indicators.proto"], &["proto"])?;
    tonic_build::compile_protos("proto/indicators.proto")?;

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("plot_descriptor.bin"))
        .compile_protos(&["proto/plot.proto"], &["proto"])?;
    tonic_build::compile_protos("proto/plot.proto")?;

    Ok(())
}
