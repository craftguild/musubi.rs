//! Shared terminal output with the existing closed product `GeneratorInfo` adapter.

use std::{
    borrow::Cow,
    io::{self, Write},
    process::ExitCode,
    sync::LazyLock,
};

use musubi_core::{GENERATOR_INFO_SCHEMA, GeneratorBuild, GeneratorInfo};

use crate::args::OutputFormat;

// The shared generated API also provides an optional JSON view.
#[allow(dead_code)]
mod captured {
    include!(concat!(env!("OUT_DIR"), "/craftguild.rs"));
}

const BRAND: craftguild_cli::Brand<'static> = craftguild_cli::Brand {
    banner: include_str!("../assets/version-banner.txt"),
    project: "musubi.rs",
};

static GENERATOR: LazyLock<GeneratorInfo> = LazyLock::new(|| {
    let info = captured::BUILD_INFO;
    // GeneratorInfo couples the two Git observations and accepts SHA-1 only.
    // Shared human/SBOM identity retains its independent observations unchanged.
    let commit = info.commit;
    let known = commit.len() == 40
        && commit.bytes().any(|b| b != b'0')
        && commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    let (git_commit, dirty) = if known && info.dirty.is_some() {
        (Some(Cow::Borrowed(commit)), info.dirty)
    } else {
        (None, None)
    };
    let revision = match (&git_commit, dirty) {
        (Some(commit), Some(true)) => format!("{commit}-dirty").into(),
        (Some(commit), Some(false)) => commit.clone(),
        _ => "unknown".into(),
    };
    let generator = GeneratorInfo {
        schema_version: GENERATOR_INFO_SCHEMA.into(),
        version: info.version.into(),
        revision,
        git_commit,
        dirty,
        build: GeneratorBuild {
            rustc: info.compiler.into(),
            host: env!("MUSUBI_BUILD_HOST").into(),
            target: info.target.into(),
            profile: info.profile.into(),
        },
    };
    generator
        .validate()
        .expect("valid captured product metadata");
    generator
});

pub(crate) fn generator() -> &'static GeneratorInfo {
    &GENERATOR
}

pub(crate) fn help(text: &str) -> io::Result<()> {
    craftguild_cli::help(&mut io::stdout().lock(), BRAND, text)
}

pub(crate) fn sbom() -> io::Result<()> {
    craftguild_cli::sbom(&mut io::stdout().lock(), captured::SBOM)
}

pub(crate) fn print(format: OutputFormat) -> io::Result<()> {
    let mut output = io::stdout().lock();
    match format {
        OutputFormat::Human => craftguild_cli::version(&mut output, BRAND, captured::BUILD_INFO),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut output, generator()).map_err(io::Error::other)?;
            output.write_all(b"\n")?;
            output.flush()
        }
    }
}

pub(crate) fn exit(result: io::Result<()>) -> ExitCode {
    result.map_or_else(
        |_| {
            let _ = io::stderr().lock().write_all(b"musubi: output failed\n");
            ExitCode::from(2)
        },
        |()| ExitCode::SUCCESS,
    )
}
