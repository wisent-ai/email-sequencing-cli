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

pub fn render_template(template: &str, variables: &Map<String, Value>) -> Result<RenderedTemplate> {
    let template = required(template, "template")?;
    let mut text = String::with_capacity(template.len());
    let mut missing = BTreeSet::new();
    let mut cursor = 0;
    while let Some(relative_start) = template[cursor..].find("{{") {
        let start = cursor + relative_start;
        text.push_str(&template[cursor..start]);
        let Some(relative_end) = template[start + 2..].find("}}") else {
            text.push_str(&template[start..]);
            cursor = template.len();
            break;
        };
        let end = start + 2 + relative_end;
        let key = template[start + 2..end].trim();
        let valid = !key.is_empty()
            && key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character));
        if !valid {
            text.push_str(&template[start..end + 2]);
        } else if let Some(value) = lookup(variables, key).filter(|value| !value.is_null()) {
            text.push_str(&display_value(value));
        } else {
            missing.insert(key.to_owned());
        }
        cursor = end + 2;
    }
    text.push_str(&template[cursor..]);
    Ok(RenderedTemplate {
        text,
        missing: missing.into_iter().collect(),
    })
}
