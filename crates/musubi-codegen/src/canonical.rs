//! Canonical JSON encoding for deterministic ABI assets.

use std::collections::BTreeMap;

pub(crate) fn canonical_abi(values: &[serde_json::Value]) -> Result<Vec<u8>, serde_json::Error> {
    let mut values = values.iter().map(canonical_value).collect::<Vec<_>>();
    values.sort_by_key(serde_json::Value::to_string);
    let mut output = serde_json::to_vec_pretty(&values)?;
    output.push(b'\n');
    Ok(output)
}

fn canonical_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => {
            let ordered: BTreeMap<_, _> = object
                .iter()
                .map(|(key, value)| (key.clone(), canonical_value(value)))
                .collect();
            serde_json::to_value(ordered).unwrap_or(serde_json::Value::Null)
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(canonical_value).collect())
        }
        scalar => scalar.clone(),
    }
}
