//! Streaming ABI structure preflight before retaining a duplicate-free JSON value tree.

use std::fmt;

use serde::{
    Deserialize,
    de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;

use crate::{Diagnostic, DiagnosticCode, HardLimits};

/// All ABI byte-entry paths share this guard; byte limits remain the responsibility of the caller.
pub(crate) fn parse(bytes: &[u8], limits: HardLimits) -> Result<Vec<Value>, Diagnostic> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    preflight(&mut deserializer, limits)?;
    deserializer.end().map_err(|_| invalid())?;
    match crate::closed_json::parse(bytes).map_err(|()| invalid())? {
        Value::Array(values) => Ok(values),
        _ => Err(invalid()),
    }
}

/// Does not retain strings, object keys, arrays or items. Seeds charge each next item/field before
/// its body is deserialized; duplicate keys use a fixed bitset. Only the parser's string scratch
/// buffer and bounded recursion stack are needed, independently of the number of array elements.
fn preflight<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
    limits: HardLimits,
) -> Result<(), Diagnostic> {
    let mut budget = Budget {
        limits,
        items: 0,
        components: 0,
        exceeded: false,
    };
    let result = Seed {
        budget: &mut budget,
        kind: Kind::Items,
        json_depth: 0,
    }
    .deserialize(deserializer);
    result.map_err(|_| {
        if budget.exceeded {
            Diagnostic::error(
                DiagnosticCode::LimitExceeded,
                "ABI structure exceeds the effective limit",
            )
        } else {
            invalid()
        }
    })
}

struct Budget {
    limits: HardLimits,
    items: usize,
    components: usize,
    exceeded: bool,
}

impl Budget {
    fn limit<E: serde::de::Error>(&mut self) -> E {
        self.exceeded = true;
        E::custom("ABI structure limit")
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Items,
    Item,
    Fields(usize),
    Field(usize),
    Scalar,
}

struct Seed<'a> {
    budget: &'a mut Budget,
    kind: Kind,
    json_depth: usize,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        // Match serde_json's existing 128-container recursion bound before its generic syntax
        // error, so excessively nested ABI input consistently produces a resource diagnostic.
        if self.json_depth >= 127 && !matches!(self.kind, Kind::Scalar) {
            return Err(self.budget.limit());
        }
        match self.kind {
            Kind::Item => {
                if self.budget.items == self.budget.limits.abi_items {
                    return Err(self.budget.limit());
                }
                self.budget.items += 1;
            }
            Kind::Field(depth) => {
                if self.budget.components == self.budget.limits.abi_components
                    || depth > self.budget.limits.nesting
                {
                    return Err(self.budget.limit());
                }
                self.budget.components += 1;
            }
            _ => {}
        }
        deserializer.deserialize_any(self)
    }
}

/// The closed ABI item/field vocabulary bounds each object's width without collecting its keys.
#[derive(Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum Key {
    Type,
    Name,
    Inputs,
    Outputs,
    StateMutability,
    Anonymous,
    Components,
    InternalType,
    Indexed,
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded closed ABI JSON")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        let kind = match self.kind {
            Kind::Items => Kind::Item,
            Kind::Fields(depth) => Kind::Field(depth),
            _ => return Err(A::Error::custom("unexpected ABI array")),
        };
        while sequence
            .next_element_seed(Seed {
                budget: self.budget,
                kind,
                json_depth: self.json_depth + 1,
            })?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<(), A::Error> {
        if !matches!(self.kind, Kind::Item | Kind::Field(_)) {
            return Err(A::Error::custom("unexpected ABI object"));
        }
        let mut seen = 0_u16;
        while let Some(key) = object.next_key::<Key>()? {
            let bit = 1 << key as u16;
            if seen & bit != 0 {
                return Err(A::Error::custom("duplicate ABI key"));
            }
            seen |= bit;
            let kind = match (self.kind, key) {
                (Kind::Item, Key::Inputs | Key::Outputs) => Kind::Fields(0),
                (Kind::Field(depth), Key::Components) => Kind::Fields(depth + 1),
                (Kind::Item | Kind::Field(_), Key::Type | Key::Name)
                | (Kind::Item, Key::StateMutability | Key::Anonymous)
                | (Kind::Field(_), Key::InternalType | Key::Indexed) => Kind::Scalar,
                _ => return Err(A::Error::custom("unknown ABI key")),
            };
            object.next_value_seed(Seed {
                budget: self.budget,
                kind,
                json_depth: self.json_depth + 1,
            })?;
        }
        Ok(())
    }

    // Scalar semantics remain with the ordinary ABI parser. Even invalid scalar ABI entries are
    // counted, so enormous arrays of tiny values cannot bypass preflight or allocate a Value tree.
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
        self.scalar()
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
        self.scalar()
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
        self.scalar()
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
        self.scalar()
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
        self.scalar()
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
        self.scalar()
    }
}

impl Seed<'_> {
    fn scalar<E: serde::de::Error>(&self) -> Result<(), E> {
        if matches!(self.kind, Kind::Items | Kind::Fields(_)) {
            Err(E::custom("ABI array expected"))
        } else {
            Ok(())
        }
    }
}

fn invalid() -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "ABI input must be bounded unique-key ABI JSON",
    )
}

#[cfg(test)]
#[path = "abi_json_tests.rs"]
mod tests;
