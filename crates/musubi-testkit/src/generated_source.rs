//! Restricts every leaf's compiler inputs to exact inventory-bound module and asset declarations.

use std::collections::{BTreeMap, BTreeSet};

use syn::visit::Visit as _;

use crate::GeneratedCrateError;

/// A facade may declare only these top-level out-of-line modules.
#[derive(Default)]
pub(crate) struct Inputs {
    pub(crate) assets: BTreeSet<String>,
    /// None denotes the sole public conventional `types` module; Some is a private exact path.
    pub(crate) modules: BTreeMap<String, Option<String>>,
}

pub(crate) fn validate(source: &[u8], inputs: &Inputs) -> Result<(), GeneratedCrateError> {
    let source = std::str::from_utf8(source).map_err(invalid)?;
    let syntax = syn::parse_file(source).map_err(invalid)?;
    let approved = modules(&syntax, &inputs.modules)?;
    let mut identifiers = Identifiers::default();
    identifiers.scan(source.parse().map_err(invalid)?, false);
    if identifiers.invalid
        || identifiers.includes != inputs.assets.len()
        || identifiers.paths
            != inputs
                .modules
                .values()
                .filter(|path| path.is_some())
                .count()
    {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    let mut visitor = Visitor {
        inputs,
        approved,
        seen_assets: BTreeSet::new(),
        modules: 0,
        invalid: false,
    };
    visitor.visit_file(&syntax);
    if visitor.invalid
        || visitor.seen_assets != inputs.assets
        || visitor.modules != identifiers.modules
    {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    Ok(())
}

fn modules<'a>(
    syntax: &'a syn::File,
    expected: &BTreeMap<String, Option<String>>,
) -> Result<Vec<&'a syn::ItemMod>, GeneratedCrateError> {
    let mut remaining = expected.clone();
    let mut approved = Vec::new();
    for item in &syntax.items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        if module.content.is_some() {
            continue;
        }
        let path = remaining
            .remove(&module.ident.to_string())
            .ok_or(GeneratedCrateError::InvalidLayout)?;
        match path {
            None if module.ident == "types"
                && matches!(module.vis, syn::Visibility::Public(_))
                && module.attrs.is_empty() => {}
            Some(path)
                if matches!(module.vis, syn::Visibility::Inherited) && module.attrs.len() == 1 =>
            {
                let attribute = &module.attrs[0];
                let syn::Meta::NameValue(value) = &attribute.meta else {
                    return Err(GeneratedCrateError::InvalidLayout);
                };
                let syn::Expr::Lit(literal) = &value.value else {
                    return Err(GeneratedCrateError::InvalidLayout);
                };
                let syn::Lit::Str(literal) = &literal.lit else {
                    return Err(GeneratedCrateError::InvalidLayout);
                };
                if !matches!(attribute.style, syn::AttrStyle::Outer)
                    || !value.path.is_ident("path")
                    || literal.value() != path
                {
                    return Err(GeneratedCrateError::InvalidLayout);
                }
            }
            _ => return Err(GeneratedCrateError::InvalidLayout),
        }
        approved.push(module);
    }
    if !remaining.is_empty() {
        return Err(GeneratedCrateError::InvalidLayout);
    }
    Ok(approved)
}

/// Token-level counts also see macro bodies and nested cfg attributes that syn's visitor skips.
#[derive(Default)]
struct Identifiers {
    includes: usize,
    paths: usize,
    modules: usize,
    invalid: bool,
}

impl Identifiers {
    fn scan(&mut self, tokens: proc_macro2::TokenStream, inside_attribute: bool) {
        let mut attribute_prefix = false;
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Group(group) => {
                    self.scan(
                        group.stream(),
                        inside_attribute
                            || (attribute_prefix
                                && group.delimiter() == proc_macro2::Delimiter::Bracket),
                    );
                    attribute_prefix = false;
                }
                proc_macro2::TokenTree::Ident(identifier) => match normalized(&identifier).as_str()
                {
                    "include_bytes" => self.includes += 1,
                    "include" | "include_str" | "env" | "option_env" => self.invalid = true,
                    "path" if inside_attribute => self.paths += 1,
                    "mod" if identifier == "mod" => self.modules += 1,
                    _ => {}
                },
                proc_macro2::TokenTree::Punct(punctuation) => {
                    attribute_prefix = punctuation.as_char() == '#';
                }
                proc_macro2::TokenTree::Literal(_) => attribute_prefix = false,
            }
        }
    }
}

struct Visitor<'a> {
    inputs: &'a Inputs,
    approved: Vec<&'a syn::ItemMod>,
    seen_assets: BTreeSet<String>,
    modules: usize,
    invalid: bool,
}

impl<'ast> syn::visit::Visit<'ast> for Visitor<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        self.modules += 1;
        if module.content.is_none() && !self.approved.iter().any(|item| std::ptr::eq(*item, module))
        {
            self.invalid = true;
        }
        syn::visit::visit_item_mod(self, module);
    }

    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if attribute
            .path()
            .segments
            .iter()
            .any(|segment| normalized(&segment.ident) == "path")
            && !self
                .approved
                .iter()
                .flat_map(|module| &module.attrs)
                .any(|item| std::ptr::eq(item, attribute))
        {
            self.invalid = true;
        }
        syn::visit::visit_attribute(self, attribute);
    }

    fn visit_macro(&mut self, expression: &'ast syn::Macro) {
        let Some(name) = expression
            .path
            .segments
            .last()
            .map(|segment| normalized(&segment.ident))
        else {
            self.invalid = true;
            return;
        };
        if name == "include_bytes" {
            match syn::parse2::<syn::LitStr>(expression.tokens.clone()) {
                Ok(path)
                    if expression.path.is_ident("include_bytes")
                        && self.inputs.assets.contains(&path.value()) =>
                {
                    if !self.seen_assets.insert(path.value()) {
                        self.invalid = true;
                    }
                }
                _ => self.invalid = true,
            }
        } else if matches!(
            name.as_str(),
            "include" | "include_str" | "env" | "option_env"
        ) {
            self.invalid = true;
        }
        syn::visit::visit_macro(self, expression);
    }
}

fn normalized(identifier: &proc_macro2::Ident) -> String {
    let name = identifier.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_owned()
}

fn invalid<T>(_: T) -> GeneratedCrateError {
    GeneratedCrateError::InvalidLayout
}
