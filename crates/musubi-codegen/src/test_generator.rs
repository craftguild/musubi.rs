//! Explicit metadata fixture used only by pure renderer and snapshot tests.

pub(crate) fn info() -> musubi_core::GeneratorInfo {
    musubi_core::GeneratorInfo {
        schema_version: musubi_core::GENERATOR_INFO_SCHEMA.into(),
        version: "0.1.0".into(),
        revision: "unknown".into(),
        git_commit: None,
        dirty: None,
        build: musubi_core::GeneratorBuild {
            rustc: "unknown".into(),
            host: "test-host".into(),
            target: "test-target".into(),
            profile: "test".into(),
        },
    }
}
