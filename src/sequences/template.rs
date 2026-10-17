//! Rendering one template: the variables it names, the values they carry,
//! and a refusal when a variable has no value.

use anyhow::Result;
use serde_json::{Map, Value};

use std::collections::BTreeSet;

use super::RenderedTemplate;
use super::normalize::required;

fn lookup<'a>(variables: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    let mut parts = key.split('.');
    let first = parts.next()?;
    let mut current = variables.get(first)?;
    for part in parts {
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The delimiters a template variable is written between.
const OPEN: &str = "{{";
const CLOSE: &str = "}}";

pub fn render_template(template: &str, variables: &Map<String, Value>) -> Result<RenderedTemplate> {
    let template = required(template, "template")?;
    let mut text = String::with_capacity(template.len());
    let mut missing = BTreeSet::new();
    let mut rest: &str = &template;
    while let Some((before, opened)) = rest.split_once(OPEN) {
        text.push_str(before);
        // An opening delimiter no closing one follows is text, kept as written;
        // no later opening delimiter is closed either, so the rest stays text.
        let Some((inner, after)) = opened.split_once(CLOSE) else {
            text.push_str(OPEN);
            rest = opened;
            continue;
        };
        let key = inner.trim();
        let valid = !key.is_empty()
            && key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character));
        if !valid {
            text.push_str(OPEN);
            text.push_str(inner);
            text.push_str(CLOSE);
        } else if let Some(value) = lookup(variables, key).filter(|value| !value.is_null()) {
            text.push_str(&display_value(value));
        } else {
            missing.insert(key.to_owned());
        }
        rest = after;
    }
    text.push_str(rest);
    Ok(RenderedTemplate {
        text,
        missing: missing.into_iter().collect(),
    })
}
