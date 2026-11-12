//! Indexed, resolved comparison of normalized solc AST types with ABI wire types.

use std::collections::{BTreeMap, BTreeSet};

use musubi_core::AbiType;

use crate::ast_type_parse::{ParsedBase, ParsedType, TypeKind};

#[derive(Clone)]
pub(super) struct Candidate {
    pub(super) name: String,
    pub(super) canonical_name: Option<String>,
    pub(super) kind: CandidateKind,
}

#[derive(Clone)]
pub(super) enum CandidateKind {
    Enum(Vec<String>),
    Udvt(ParsedType),
    Struct(Vec<(String, ParsedType)>),
}

pub(super) struct Candidates {
    items: Vec<Candidate>,
    typed: BTreeMap<TypeKind, BTreeMap<String, Vec<usize>>>,
}

impl Candidates {
    pub(super) fn new(mut items: Vec<Candidate>) -> Self {
        let mut typed = BTreeMap::<TypeKind, BTreeMap<String, Vec<usize>>>::new();
        let mut any = BTreeMap::<String, Vec<usize>>::new();
        for (index, candidate) in items.iter().enumerate() {
            let kind = candidate_kind(&candidate.kind);
            let mut identities = vec![candidate.name.as_str()];
            if let Some(canonical) = candidate.canonical_name.as_deref()
                && canonical != candidate.name
            {
                identities.push(canonical);
            }
            for identity in identities {
                typed
                    .entry(kind)
                    .or_default()
                    .entry(identity.to_owned())
                    .or_default()
                    .push(index);
                any.entry(identity.to_owned()).or_default().push(index);
            }
        }
        for candidate in &mut items {
            match &mut candidate.kind {
                CandidateKind::Udvt(parsed) => resolve(parsed, &typed, &any),
                CandidateKind::Struct(members) => {
                    for (_, parsed) in members {
                        resolve(parsed, &typed, &any);
                    }
                }
                CandidateKind::Enum(_) => {}
            }
        }
        Self { items, typed }
    }

    pub(super) fn unique(&self, kind: TypeKind, identity: &str) -> Option<&Candidate> {
        let indexes = self.typed.get(&kind)?.get(identity)?;
        (indexes.len() == 1).then(|| &self.items[indexes[0]])
    }
}

fn resolve(
    parsed: &mut ParsedType,
    typed: &BTreeMap<TypeKind, BTreeMap<String, Vec<usize>>>,
    any: &BTreeMap<String, Vec<usize>>,
) {
    let ParsedBase::Named { kind, identity } = &parsed.base else {
        return;
    };
    let indexes = kind
        .and_then(|kind| typed.get(&kind).and_then(|entries| entries.get(identity)))
        .or_else(|| kind.is_none().then(|| any.get(identity)).flatten());
    parsed.base = indexes
        .filter(|indexes| indexes.len() == 1)
        .map_or(ParsedBase::Unknown, |indexes| {
            ParsedBase::Resolved(indexes[0])
        });
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Comparison {
    Equal,
    Unknown,
    Mismatch,
}

pub(super) fn compare(
    parsed: &ParsedType,
    expected: &AbiType,
    candidates: &Candidates,
    maximum_depth: usize,
) -> Comparison {
    compare_inner(
        parsed,
        expected,
        candidates,
        maximum_depth,
        0,
        &mut BTreeSet::new(),
    )
}

fn compare_inner(
    parsed: &ParsedType,
    expected: &AbiType,
    candidates: &Candidates,
    maximum_depth: usize,
    depth: usize,
    visiting: &mut BTreeSet<usize>,
) -> Comparison {
    if depth.saturating_add(parsed.nesting) > maximum_depth {
        return Comparison::Unknown;
    }
    let mut expected = expected;
    for dimension in &parsed.dimensions {
        expected = match (dimension, expected) {
            (None, AbiType::Array(element)) => element,
            (Some(left), AbiType::FixedArray(element, right)) if left == right => element,
            _ => return Comparison::Mismatch,
        };
    }
    let nested_depth = depth.saturating_add(parsed.dimensions.len());
    match &parsed.base {
        ParsedBase::Abi(primitive) => equality(primitive, expected),
        ParsedBase::InternalFunction => Comparison::Mismatch,
        ParsedBase::Resolved(index) => compare_candidate(
            *index,
            expected,
            candidates,
            maximum_depth,
            nested_depth,
            visiting,
        ),
        ParsedBase::Named { .. } | ParsedBase::Unknown => Comparison::Unknown,
    }
}

fn compare_candidate(
    index: usize,
    expected: &AbiType,
    candidates: &Candidates,
    maximum_depth: usize,
    depth: usize,
    visiting: &mut BTreeSet<usize>,
) -> Comparison {
    if !visiting.insert(index) {
        return Comparison::Unknown;
    }
    let result = match &candidates.items[index].kind {
        CandidateKind::Enum(variants) if (1..=256).contains(&variants.len()) => {
            equality(&AbiType::Uint(8), expected)
        }
        CandidateKind::Udvt(underlying) => compare_inner(
            underlying,
            expected,
            candidates,
            maximum_depth,
            depth.saturating_add(1),
            visiting,
        ),
        CandidateKind::Struct(members) if depth < maximum_depth => compare_struct(
            members,
            expected,
            candidates,
            maximum_depth,
            depth,
            visiting,
        ),
        CandidateKind::Enum(_) | CandidateKind::Struct(_) => Comparison::Unknown,
    };
    visiting.remove(&index);
    result
}

fn compare_struct(
    members: &[(String, ParsedType)],
    expected: &AbiType,
    candidates: &Candidates,
    maximum_depth: usize,
    depth: usize,
    visiting: &mut BTreeSet<usize>,
) -> Comparison {
    let AbiType::Tuple(fields) = expected else {
        return Comparison::Mismatch;
    };
    if fields.len() != members.len() {
        return Comparison::Mismatch;
    }
    let mut result = Comparison::Equal;
    for (field, (name, parsed)) in fields.iter().zip(members) {
        if !field.name.is_empty() && field.name != *name {
            result = Comparison::Mismatch;
        }
        result = merge(
            result,
            compare_inner(
                parsed,
                &field.ty,
                candidates,
                maximum_depth,
                depth.saturating_add(1),
                visiting,
            ),
        );
    }
    result
}

fn merge(left: Comparison, right: Comparison) -> Comparison {
    match (left, right) {
        (Comparison::Mismatch, _) | (_, Comparison::Mismatch) => Comparison::Mismatch,
        (Comparison::Unknown, _) | (_, Comparison::Unknown) => Comparison::Unknown,
        _ => Comparison::Equal,
    }
}

fn equality(left: &AbiType, right: &AbiType) -> Comparison {
    if wire_equal(left, right) {
        Comparison::Equal
    } else {
        Comparison::Mismatch
    }
}

pub(super) fn valid_qualified(value: &str, maximum: usize) -> bool {
    crate::ast_type_parse::valid_qualified(value, maximum)
}

pub(super) fn candidate_kind(kind: &CandidateKind) -> TypeKind {
    match kind {
        CandidateKind::Enum(_) => TypeKind::Enum,
        CandidateKind::Udvt(_) => TypeKind::Udvt,
        CandidateKind::Struct(_) => TypeKind::Struct,
    }
}

pub(super) fn wire_equal(left: &AbiType, right: &AbiType) -> bool {
    match (left, right) {
        (AbiType::Array(left), AbiType::Array(right)) => wire_equal(left, right),
        (AbiType::FixedArray(left, left_len), AbiType::FixedArray(right, right_len)) => {
            left_len == right_len && wire_equal(left, right)
        }
        (AbiType::Tuple(left), AbiType::Tuple(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| wire_equal(&left.ty, &right.ty))
        }
        _ => left == right,
    }
}
