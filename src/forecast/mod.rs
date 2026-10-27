//! How many messages fall due on each coming day: every pending step of every
//! active enrollment, by the UTC day it falls due within the days asked, so
//! a sender's daily quota can be checked before the day it is exceeded. A
//! step already past due goes out at once and is counted under `overdue`, not
//! spread over the days.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::sequences::enrollment::normalize_enrollment;
use crate::sequences::normalize::parse_date;
use crate::sequences::plan::plan_enrollment;
use crate::sequences::{Enrollment, EnrollmentStatus, PlannedState, Sequence, TERMINAL_EVENT_TYPES};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastDay {
    /// The UTC day, `YYYY-MM-DD`.
    pub day: String,
    pub messages: usize,
    /// Messages per step that day, by step id.
    pub by_step: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub from: String,
    pub days: Vec<ForecastDay>,
    /// Pending steps already due at `from`: they go out at once.
    pub overdue: usize,
    /// The busiest day's messages, the number a daily quota must cover.
    pub peak: usize,
}

/// The messages of `sequence` due over the `days` UTC days starting at `at`'s day.
pub fn forecast(sequence: &Sequence, enrollments: Vec<Enrollment>, at: DateTime<Utc>, days: i64) -> Result<Forecast> {
    if !days.is_positive() {
        bail!("--days must be a whole number of days above zero, got {days}");
    }
    let first = at.date_naive();
    let mut counts: BTreeMap<chrono::NaiveDate, BTreeMap<String, usize>> =
        (first.iter_days().take(days as usize)).map(|day| (day, BTreeMap::new())).collect();
    let mut overdue = Vec::new();
    for input in enrollments {
        let enrollment = normalize_enrollment(input)?;
        let stopped = enrollment.events.iter().any(|event| TERMINAL_EVENT_TYPES.contains(&event.kind.as_str()));
        if !matches!(enrollment.status, EnrollmentStatus::Active) || stopped {
            continue;
        }
        for step in plan_enrollment(sequence, enrollment)?.into_iter().filter(|step| matches!(step.state, PlannedState::Pending)) {
            let due = parse_date(&step.due_at, "step.dueAt")?;
            if due < at {
                overdue.push(step.step_id);
                continue;
            }
            if let Some(day) = counts.get_mut(&due.date_naive()) {
                *day.entry(step.step_id).or_default() += std::slice::from_ref(&due).len();
            }
        }
    }
    let days: Vec<ForecastDay> = counts
        .into_iter()
        .map(|(day, by_step)| ForecastDay { day: day.to_string(), messages: by_step.values().sum(), by_step })
        .collect();
    let peak = days.iter().map(|day| day.messages).max().into_iter().sum();
    Ok(Forecast { from: first.to_string(), days, overdue: overdue.len(), peak })
}
