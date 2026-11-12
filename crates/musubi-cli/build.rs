//! Verify shared preparation and retain the product record's actual build host.

#![forbid(unsafe_code)]

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let package =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").ok_or("package")?);
    craftguild_cli_build::emit(&package.join("../.."), "musubi")?;
    println!(
        "cargo:rustc-env=MUSUBI_BUILD_HOST={}",
        std::env::var("HOST")?
    );
    Ok(())
}
