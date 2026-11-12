//! Closed package identity accepted at input, snapshot, and public rendering boundaries.

pub(crate) fn valid(name: &str, version: &str, maximum_bytes: usize) -> bool {
    let mut name_bytes = name.bytes();
    name.len() <= maximum_bytes
        && name_bytes
            .next()
            .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && name_bytes.all(|byte| byte == b'_' || byte == b'-' || byte.is_ascii_alphanumeric())
        && valid_version(version, maximum_bytes)
}

pub(crate) fn valid_version(version: &str, maximum_bytes: usize) -> bool {
    version.len() <= maximum_bytes && semver::Version::parse(version).is_ok()
}
