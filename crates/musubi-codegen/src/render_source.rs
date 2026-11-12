//! Shared bounded writers for generated source and serialized evidence.

use std::{fmt::Write as _, io::Write as _};

use crate::{RenderError, render::render_error};

pub(crate) struct BoundedSource {
    value: String,
    maximum_bytes: usize,
}

impl BoundedSource {
    pub(crate) fn new(maximum_bytes: usize) -> Self {
        Self {
            value: String::new(),
            maximum_bytes,
        }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.maximum_bytes.saturating_sub(self.value.len())
    }

    pub(crate) fn push(&mut self, value: &str) -> Result<(), RenderError> {
        self.write_str(value).map_err(|_| output_limit_error())
    }

    pub(crate) fn formatted(&mut self, value: std::fmt::Arguments<'_>) -> Result<(), RenderError> {
        self.write_fmt(value).map_err(|_| output_limit_error())
    }

    pub(crate) fn finish(self) -> String {
        self.value
    }
}

impl std::fmt::Write for BoundedSource {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        if value.len() > self.remaining() {
            return Err(std::fmt::Error);
        }
        self.value.push_str(value);
        Ok(())
    }
}

pub(crate) struct BoundedBytes {
    value: Vec<u8>,
    maximum_bytes: usize,
}

impl BoundedBytes {
    pub(crate) fn new(maximum_bytes: usize) -> Self {
        Self {
            value: Vec::new(),
            maximum_bytes,
        }
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.value
    }
}

impl std::io::Write for BoundedBytes {
    fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
        let remaining = self.maximum_bytes.saturating_sub(self.value.len());
        if value.len() > remaining {
            return Err(std::io::Error::other("generated output limit"));
        }
        self.value.extend_from_slice(value);
        Ok(value.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) fn serialize_pretty<T: serde::Serialize>(
    value: &T,
    maximum_bytes: usize,
) -> Result<Vec<u8>, RenderError> {
    let mut output = BoundedBytes::new(maximum_bytes);
    serde_json::to_writer_pretty(&mut output, value)
        .map_err(|_| render_error("release manifest serialization failed"))?;
    output.write_all(b"\n").map_err(|_| output_limit_error())?;
    Ok(output.finish())
}

fn output_limit_error() -> RenderError {
    render_error("generated source exceeds effective output limits")
}
