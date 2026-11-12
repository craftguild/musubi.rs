//! Reachability checks for bytecode inventories retained by snapshots.

use musubi_core::{BytecodeRange, ImmutableReference, LinkReference};

use crate::{HardLimits, SnapshotError, names::validate_abi_name};

pub(crate) fn validate(
    template_bytes: usize,
    links: &[LinkReference],
    immutables: &[ImmutableReference],
    limits: HardLimits,
) -> Result<(), SnapshotError> {
    if !(1..=limits.bytecode_bytes).contains(&template_bytes) {
        return Err(SnapshotError::InvalidJson);
    }
    if !strict_order(
        links
            .iter()
            .map(|item| (item.source.as_str(), item.library.as_str())),
    ) || !strict_order(immutables.iter().map(|item| item.id.as_str()))
    {
        return Err(SnapshotError::InvalidJson);
    }
    let mut occupied = Vec::new();
    for link in links {
        if link.source.len() > limits.identifier_bytes
            || link.library.len() > limits.identifier_bytes
            || !musubi_core::is_canonical_source_path(&link.source)
            || validate_abi_name(&link.library).is_err()
            || link.ranges.is_empty()
        {
            return Err(SnapshotError::InvalidJson);
        }
        validate_ranges(
            &link.ranges,
            Some(20),
            template_bytes,
            &mut occupied,
            limits,
        )?;
    }
    for immutable in immutables {
        if immutable.id.is_empty()
            || immutable.id.len() > limits.identifier_bytes
            || !immutable.id.bytes().all(|byte| byte.is_ascii_digit())
            || immutable.ranges.is_empty()
        {
            return Err(SnapshotError::InvalidJson);
        }
        validate_ranges(
            &immutable.ranges,
            None,
            template_bytes,
            &mut occupied,
            limits,
        )?;
    }
    occupied.sort_unstable();
    if occupied.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(SnapshotError::InvalidJson);
    }
    Ok(())
}

fn validate_ranges(
    ranges: &[BytecodeRange],
    exact_length: Option<usize>,
    template_bytes: usize,
    occupied: &mut Vec<(usize, usize)>,
    limits: HardLimits,
) -> Result<(), SnapshotError> {
    if !strict_order(ranges.iter().map(|range| (range.start, range.length))) {
        return Err(SnapshotError::InvalidJson);
    }
    for range in ranges {
        let end = range
            .start
            .checked_add(range.length)
            .ok_or(SnapshotError::InvalidJson)?;
        if range.length == 0
            || end > template_bytes
            || occupied.len() >= limits.bytecode_bytes
            || exact_length.is_some_and(|length| range.length != length)
        {
            return Err(SnapshotError::InvalidJson);
        }
        occupied.push((range.start, end));
    }
    Ok(())
}

fn strict_order<T: Ord>(values: impl Iterator<Item = T>) -> bool {
    let values = values.collect::<Vec<_>>();
    values.windows(2).all(|pair| pair[0] < pair[1])
}
