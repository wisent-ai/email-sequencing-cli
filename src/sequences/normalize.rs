//! Reading a sequence as given: the fields it must carry, its dates, and
//! the steps in the order they run.

use anyhow::{Context, Result, bail};
use chrono::{DateTime, SecondsFormat, Utc};

use std::collections::HashSet;

use super::{Sequence, SequenceInput, SequenceStep};

pub(crate) fn required(value: &str, label: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("{label} must be a non-empty string");
    }
    Ok(normalized.to_owned())
}

pub(crate) fn optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let normalized = item.trim();
        (!normalized.is_empty()).then(|| normalized.to_owned())
    })
}

pub(crate) fn parse_date(value: &str, label: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label} must be a valid ISO-8601 date"))
        .map(|date| date.with_timezone(&Utc))
}

pub(crate) fn format_date(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn normalize_sequence(input: SequenceInput) -> Result<Sequence> {
    if input.steps.is_empty() {
        bail!("sequence.steps must be a non-empty array");
    }
    let mut seen = HashSet::new();
    let mut steps = Vec::with_capacity(input.steps.len());
    for (index, step) in input.steps.into_iter().enumerate() {
        let id = required(&step.id, &format!("sequence.steps[{index}].id"))?;
        if !seen.insert(id.clone()) {
            bail!("duplicate sequence step id: {id}");
        }
        if !step.after_hours.is_finite() || step.after_hours < 0.0 {
            bail!("sequence.steps[{index}].afterHours must be a non-negative number");
        }
        steps.push(SequenceStep {
            id,
            after_hours: step.after_hours,
            subject: optional(step.subject),
            template: required(&step.template, &format!("sequence.steps[{index}].template"))?,
        });
    }
    Ok(Sequence {
        id: required(&input.id, "sequence.id")?,
        name: optional(input.name),
        steps,
    })
}
pub fn parse_optional_date(value: Option<&str>, label: &str) -> Result<Option<DateTime<Utc>>> {
    value.map(|item| parse_date(item, label)).transpose()
}
