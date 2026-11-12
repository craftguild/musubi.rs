//! Literal SDK accessor rendering from the exact same validated metadata serialized to JSON.

use musubi_core::GeneratorInfo;

use crate::{RenderError, render_source::BoundedSource};

pub(crate) fn accessor(
    source: &mut BoundedSource,
    generator: &GeneratorInfo,
) -> Result<(), RenderError> {
    source.push("/// Returns the generator's recorded build observations without a connection.\n///\n/// This is not authentication. A hash-dirty revision does not uniquely identify\n/// uncommitted contents; unknown provides no source identity. Runtime source is separate.\n#[rustfmt::skip]\npub fn generator() -> &'static musubi_core::GeneratorInfo {\n    static INFO: musubi_core::GeneratorInfo = ")?;
    source.formatted(format_args!(
        "musubi_core::GeneratorInfo {{ schema_version: {}, version: {}, revision: {}, git_commit: {}, dirty: {:?}, build: musubi_core::GeneratorBuild {{ rustc: {}, host: {}, target: {}, profile: {} }} }};\n    &INFO\n}}\n\n",
        borrowed(&generator.schema_version), borrowed(&generator.version),
        borrowed(&generator.revision),
        generator.git_commit.as_ref().map_or_else(|| "None".into(), |commit| format!("Some({})", borrowed(commit))),
        generator.dirty, borrowed(&generator.build.rustc), borrowed(&generator.build.host),
        borrowed(&generator.build.target), borrowed(&generator.build.profile)
    ))
}

fn borrowed(value: &str) -> String {
    format!("::std::borrow::Cow::Borrowed({value:?})")
}
