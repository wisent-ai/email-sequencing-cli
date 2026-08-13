use std::collections::{BTreeSet, HashSet};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const TERMINAL_EVENT_TYPES: &[&str] = &["replied", "bounced", "unsubscribed"];
const STEP_EVENT_TYPES: &[&str] = &["sent", "skipped"];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceInput {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub steps: Vec<SequenceStepInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceStepInput {
    pub id: String,
    #[serde(default)]
    pub after_hours: f64,
    #[serde(default)]
    pub subject: Option<String>,
    pub template: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    pub id: String,
    pub name: Option<String>,
    pub steps: Vec<SequenceStep>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceStep {
    pub id: String,
    pub after_hours: f64,
    pub subject: Option<String>,
    pub template: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentSeed {
    pub id: String,
    pub contact: ContactInput,
    #[serde(default)]
    pub enrolled_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    pub id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub variables: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub variables: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub at: String,
    #[serde(default)]
    pub step_id: Option<String>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnrollmentStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
}

impl Default for EnrollmentStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enrollment {
    pub id: String,
    pub sequence_id: String,
    pub contact: Contact,
    pub enrolled_at: String,
    #[serde(default)]
    pub status: EnrollmentStatus,
    #[serde(default)]
    pub events: Vec<EnrollmentEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlannedState {
    Sent,
    Skipped,
    Pending,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedStep {
    pub enrollment_id: String,
    pub contact_id: String,
    pub sequence_id: String,
    pub step_id: String,
    pub due_at: String,
    pub state: PlannedState,
    pub subject: Option<String>,
    pub template: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedTemplate {
    pub text: String,
    pub missing: Vec<String>,
}

fn required(value: &str, label: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("{label} must be a non-empty string");
    }
    Ok(normalized.to_owned())
}

fn optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let normalized = item.trim();
        (!normalized.is_empty()).then(|| normalized.to_owned())
    })
}

fn parse_date(value: &str, label: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label} must be a valid ISO-8601 date"))
        .map(|date| date.with_timezone(&Utc))
}

fn format_date(value: DateTime<Utc>) -> String {
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

pub fn create_enrollment(
    sequence: &Sequence,
    input: EnrollmentSeed,
    at: Option<DateTime<Utc>>,
) -> Result<Enrollment> {
    let enrolled_at = match at {
        Some(date) => date,
        None => match input.enrolled_at.as_deref() {
            Some(value) => parse_date(value, "enrollment.enrolledAt")?,
            None => Utc::now(),
        },
    };
    Ok(Enrollment {
        id: required(&input.id, "enrollment.id")?,
        sequence_id: sequence.id.clone(),
        contact: Contact {
            id: required(&input.contact.id, "enrollment.contact.id")?,
            email: optional(input.contact.email),
            variables: input.contact.variables,
        },
        enrolled_at: format_date(enrolled_at),
        status: EnrollmentStatus::Active,
        events: Vec::new(),
    })
}

pub fn normalize_enrollment(mut input: Enrollment) -> Result<Enrollment> {
    input.id = required(&input.id, "enrollment.id")?;
    input.sequence_id = required(&input.sequence_id, "enrollment.sequenceId")?;
    input.contact.id = required(&input.contact.id, "enrollment.contact.id")?;
    input.contact.email = optional(input.contact.email);
    input.enrolled_at = format_date(parse_date(&input.enrolled_at, "enrollment.enrolledAt")?);
    for (index, event) in input.events.iter_mut().enumerate() {
        event.kind = required(&event.kind, &format!("enrollment.events[{index}].type"))?;
        event.at = format_date(parse_date(
            &event.at,
            &format!("enrollment.events[{index}].at"),
        )?);
        event.step_id = optional(event.step_id.take());
        event.message_id = optional(event.message_id.take());
        event.reason = optional(event.reason.take());
        if STEP_EVENT_TYPES.contains(&event.kind.as_str()) && event.step_id.is_none() {
            bail!("{} events require stepId", event.kind);
        }
    }
    input.events.sort_by(|left, right| left.at.cmp(&right.at));
    Ok(input)
}

pub fn record_event(input: Enrollment, event: EnrollmentEvent) -> Result<Enrollment> {
    let mut enrollment = normalize_enrollment(input)?;
    enrollment.events.push(event);
    enrollment = normalize_enrollment(enrollment)?;
    if enrollment
        .events
        .iter()
        .any(|item| TERMINAL_EVENT_TYPES.contains(&item.kind.as_str()))
    {
        enrollment.status = EnrollmentStatus::Completed;
    }
    Ok(enrollment)
}

pub fn plan_enrollment(sequence: &Sequence, input: Enrollment) -> Result<Vec<PlannedStep>> {
    let enrollment = normalize_enrollment(input)?;
    if sequence.id != enrollment.sequence_id {
        bail!("sequence and enrollment sequenceId do not match");
    }
    let sent: HashSet<&str> = enrollment
        .events
        .iter()
        .filter(|event| event.kind == "sent")
        .filter_map(|event| event.step_id.as_deref())
        .collect();
    let skipped: HashSet<&str> = enrollment
        .events
        .iter()
        .filter(|event| event.kind == "skipped")
        .filter_map(|event| event.step_id.as_deref())
        .collect();
    let mut due_at = parse_date(&enrollment.enrolled_at, "enrollment.enrolledAt")?;
    let mut planned = Vec::with_capacity(sequence.steps.len());
    for step in &sequence.steps {
        let milliseconds = step.after_hours * 3_600_000.0;
        if milliseconds > i64::MAX as f64 {
            bail!("step delay is too large");
        }
        due_at += Duration::milliseconds(milliseconds.round() as i64);
        let state = if sent.contains(step.id.as_str()) {
            PlannedState::Sent
        } else if skipped.contains(step.id.as_str()) {
            PlannedState::Skipped
        } else {
            PlannedState::Pending
        };
        planned.push(PlannedStep {
            enrollment_id: enrollment.id.clone(),
            contact_id: enrollment.contact.id.clone(),
            sequence_id: sequence.id.clone(),
            step_id: step.id.clone(),
            due_at: format_date(due_at),
            state,
            subject: step.subject.clone(),
            template: step.template.clone(),
        });
    }
    Ok(planned)
}

pub fn next_actions(
    sequence: &Sequence,
    enrollments: Vec<Enrollment>,
    at: DateTime<Utc>,
) -> Result<Vec<PlannedStep>> {
    let mut actions = Vec::new();
    for input in enrollments {
        let enrollment = normalize_enrollment(input)?;
        if !matches!(enrollment.status, EnrollmentStatus::Active)
            || enrollment
                .events
                .iter()
                .any(|event| TERMINAL_EVENT_TYPES.contains(&event.kind.as_str()))
        {
            continue;
        }
        if let Some(action) = plan_enrollment(sequence, enrollment)?
            .into_iter()
            .find(|step| matches!(step.state, PlannedState::Pending))
            && parse_date(&action.due_at, "step.dueAt")? <= at
        {
            actions.push(action);
        }
    }
    actions.sort_by(|left, right| {
        left.due_at
            .cmp(&right.due_at)
            .then_with(|| left.enrollment_id.cmp(&right.enrollment_id))
    });
    Ok(actions)
}

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

pub fn parse_optional_date(value: Option<&str>, label: &str) -> Result<Option<DateTime<Utc>>> {
    value.map(|item| parse_date(item, label)).transpose()
}
