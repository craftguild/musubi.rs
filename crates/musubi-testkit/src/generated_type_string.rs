//! Strict parsing and canonical reconstruction of EIP-712 `encodeType` evidence.

use std::collections::BTreeSet;

use musubi_core::{
    TypedDataField, TypedDataStruct, TypedDataType, is_canonical_solidity_identifier,
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_TYPES: usize = 4_096;
const MAX_FIELDS: usize = 65_536;
const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_DEPTH: usize = 64;

struct RawDefinition<'a> {
    name: &'a str,
    fields: Vec<RawField<'a>>,
}

struct RawField<'a> {
    name: &'a str,
    ty: &'a str,
}

pub(crate) fn canonical(primary: &str, text: &str) -> bool {
    if text.is_empty() || text.len() > MAX_BYTES || !text.is_ascii() {
        return false;
    }
    let Some(raw) = Parser::new(text).definitions() else {
        return false;
    };
    if raw
        .first()
        .is_none_or(|definition| definition.name != primary)
    {
        return false;
    }
    let names = raw
        .iter()
        .map(|definition| definition.name)
        .collect::<BTreeSet<_>>();
    if names.len() != raw.len() {
        return false;
    }
    let Some(types) = raw
        .into_iter()
        .map(|definition| convert(definition, &names))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    musubi_runtime::TypedDataGraph::new(&types)
        .and_then(|graph| graph.type_string(primary))
        .is_ok_and(|canonical| canonical == text)
}

fn convert(raw: RawDefinition<'_>, names: &BTreeSet<&str>) -> Option<TypedDataStruct> {
    let fields = raw
        .fields
        .into_iter()
        .map(|field| {
            Some(TypedDataField {
                name: field.name.into(),
                ty: parse_type(field.ty, names, 0)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(TypedDataStruct {
        name: raw.name.into(),
        fields,
    })
}

fn parse_type(value: &str, names: &BTreeSet<&str>, depth: usize) -> Option<TypedDataType> {
    if depth > MAX_DEPTH || value.is_empty() {
        return None;
    }
    if let Some(prefix) = value.strip_suffix(']') {
        let start = prefix.rfind('[')?;
        let length = &prefix[start + 1..];
        let element = Box::new(parse_type(&prefix[..start], names, depth + 1)?);
        return if length.is_empty() {
            Some(TypedDataType::Array(element))
        } else {
            let length = canonical_decimal(length)?;
            (length <= MAX_FIELDS).then_some(TypedDataType::FixedArray(element, length))
        };
    }
    match value {
        "address" => Some(TypedDataType::Address),
        "bool" => Some(TypedDataType::Bool),
        "string" => Some(TypedDataType::String),
        "bytes" => Some(TypedDataType::Bytes),
        _ if value.starts_with("bytes") => {
            width(&value[5..], 1, 32, false).map(TypedDataType::FixedBytes)
        }
        _ if value.starts_with("uint") => width(&value[4..], 8, 256, true).map(TypedDataType::Uint),
        _ if value.starts_with("int") => width(&value[3..], 8, 256, true).map(TypedDataType::Int),
        _ if names.contains(value) => Some(TypedDataType::Struct(value.into())),
        _ => None,
    }
}

fn width(value: &str, minimum: usize, maximum: usize, aligned: bool) -> Option<usize> {
    let value = canonical_decimal(value)?;
    ((minimum..=maximum).contains(&value) && (!aligned || value % 8 == 0)).then_some(value)
}

fn canonical_decimal(value: &str) -> Option<usize> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    value.parse().ok().filter(|value| *value != 0)
}

struct Parser<'a> {
    text: &'a str,
    offset: usize,
    fields: usize,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            offset: 0,
            fields: 0,
        }
    }

    fn definitions(mut self) -> Option<Vec<RawDefinition<'a>>> {
        let mut output = Vec::new();
        while self.offset < self.text.len() {
            if output.len() >= MAX_TYPES {
                return None;
            }
            let name = self.identifier()?;
            self.byte(b'(')?;
            let mut fields = Vec::new();
            loop {
                if self.peek()? == b')' || self.fields >= MAX_FIELDS {
                    return None;
                }
                let start = self.offset;
                while self.peek()? != b' ' {
                    self.offset += 1;
                }
                let ty = self.text.get(start..self.offset)?;
                self.byte(b' ')?;
                let field_name = self.identifier()?;
                self.fields += 1;
                fields.push(RawField {
                    name: field_name,
                    ty,
                });
                match self.peek()? {
                    b',' => self.offset += 1,
                    b')' => {
                        self.offset += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            output.push(RawDefinition { name, fields });
        }
        (!output.is_empty()).then_some(output)
    }

    fn identifier(&mut self) -> Option<&'a str> {
        let start = self.offset;
        while self.offset < self.text.len()
            && matches!(self.peek()?, b'_' | b'$' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z')
        {
            self.offset += 1;
        }
        let value = self.text.get(start..self.offset)?;
        (value.len() <= MAX_IDENTIFIER_BYTES && is_canonical_solidity_identifier(value))
            .then_some(value)
    }

    fn byte(&mut self, expected: u8) -> Option<()> {
        (self.peek()? == expected).then(|| self.offset += 1)
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.offset).copied()
    }
}
