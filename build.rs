//! Generates static build info.

use std::process::Command;

use anyhow::Error;
use vergen::{BuildBuilder, CargoBuilder, Emitter, RustcBuilder, SysinfoBuilder};

fn main() -> Result<(), Error> {
    {
        let git_hash = std::env::var("GIT_COMMIT").unwrap_or_else(|_| {
            let output = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap();
            String::from_utf8(output.stdout).unwrap()
        });
        println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
    }

    {
        let build = BuildBuilder::all_build()?;
        let cargo = CargoBuilder::all_cargo()?;
        let rustc = RustcBuilder::all_rustc()?;
        let si = SysinfoBuilder::all_sysinfo()?;

        Emitter::default()
            .add_instructions(&build)?
            .add_instructions(&cargo)?
            .add_instructions(&rustc)?
            .add_instructions(&si)?
            .emit()?;
    }

    Ok(())
}
