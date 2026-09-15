//! What is due: every step of one enrollment with its state, and the steps
//! across many enrollments that are due right now.

use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};

use std::collections::HashSet;

use super::{
    Enrollment, EnrollmentStatus, PlannedState, PlannedStep, Sequence, TERMINAL_EVENT_TYPES,
};
use super::enrollment::normalize_enrollment;
use super::normalize::{format_date, parse_date};

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
