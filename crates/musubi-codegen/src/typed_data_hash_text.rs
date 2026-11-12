//! Canonical lexical validation for reviewed typed-data hash evidence.

use crate::IrError;

pub(crate) fn validate(document: &serde_json::Value) -> Result<(), IrError> {
    for field in [
        "solidity_source_sha256",
        "artifact_sha256",
        "compiler_settings_sha256",
    ] {
        let value = document
            .pointer(&format!("/evidence/{field}"))
            .and_then(serde_json::Value::as_str);
        if !value.is_some_and(canonical_nonzero_hash) {
            return Err(failure("typed-data evidence hash is not canonical"));
        }
    }
    let vectors = document
        .get("vectors")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| failure("typed-data vector list is malformed"))?;
    for vector in vectors {
        if ["domain_type", "domain", "domain_type_string"]
            .into_iter()
            .any(|field| vector.get(field).is_some_and(serde_json::Value::is_null))
        {
            return Err(failure(
                "typed-data optional domain fields must be omitted instead of null",
            ));
        }
        for field in ["type_hash", "struct_hash"] {
            if !vector
                .get(field)
                .and_then(serde_json::Value::as_str)
                .is_some_and(canonical_nonzero_hash)
            {
                return Err(failure("typed-data vector hash is not canonical"));
            }
        }
        for field in ["domain_type_hash", "domain_separator", "digest"] {
            if vector
                .get(field)
                .is_some_and(|value| !value.as_str().is_some_and(canonical_nonzero_hash))
            {
                return Err(failure("typed-data vector hash is not canonical"));
            }
        }
    }
    Ok(())
}

fn canonical_nonzero_hash(value: &str) -> bool {
    let Some(raw) = value.strip_prefix("0x") else {
        return false;
    };
    raw.len() == 64
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && raw.bytes().any(|byte| byte != b'0')
}

fn failure(message: &str) -> IrError {
    crate::typed_data_schema::failure(message)
}
