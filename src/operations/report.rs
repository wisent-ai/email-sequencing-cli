//! How a sequence is doing across its enrollments: how many people stand in
//! each status, and per step how many messages were sent or skipped and how
//! many replies, bounces and unsubscribes followed it. A reply, bounce or
//! unsubscribe counts against the step sent last before it in that
//! enrollment, or against none when nothing was sent yet.

use anyhow::{Result, bail};
use serde::Serialize;

use crate::sequences::enrollment::normalize_enrollment;
use crate::sequences::{Enrollment, EnrollmentStatus, Sequence, TERMINAL_EVENT_TYPES};

/// What one step of the sequence brought.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReport {
    pub step_id: String,
    pub sent: usize,
    pub skipped: usize,
    pub replied: usize,
    pub bounced: usize,
    pub unsubscribed: usize,
}

/// How many enrollments stand in each status.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCounts {
    pub active: usize,
    pub paused: usize,
    pub completed: usize,
    pub cancelled: usize,
}

/// The whole report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceReport {
    pub sequence_id: String,
    pub enrollments: usize,
    pub statuses: StatusCounts,
    /// Every step in the sequence's order, a step nobody reached included.
    pub steps: Vec<StepReport>,
    /// Replies, bounces and unsubscribes that came before any message was
    /// sent: from outside the sequence.
    pub before_any_step: StepReport,
}

/// The report over `enrollments`, each read the way `record` reads one;
/// refused for an enrollment of another sequence or an event naming a step
/// the sequence does not hold.
pub fn report(sequence: &Sequence, enrollments: Vec<Enrollment>) -> Result<SequenceReport> {
    let mut steps: Vec<StepReport> =
        sequence.steps.iter().map(|step| StepReport { step_id: step.id.clone(), ..Default::default() }).collect();
    let mut before_any_step = StepReport { step_id: String::new(), ..Default::default() };
    let mut statuses = StatusCounts::default();
    let count = enrollments.len();
    for input in enrollments {
        let enrollment = normalize_enrollment(input)?;
        if enrollment.sequence_id != sequence.id {
            bail!(
                "enrollment {} is in sequence {}, not {}; report one sequence's enrollments at a time",
                enrollment.id,
                enrollment.sequence_id,
                sequence.id
            );
        }
        let held = match enrollment.status {
            EnrollmentStatus::Active => &mut statuses.active,
            EnrollmentStatus::Paused => &mut statuses.paused,
            EnrollmentStatus::Completed => &mut statuses.completed,
            EnrollmentStatus::Cancelled => &mut statuses.cancelled,
        };
        *held += std::slice::from_ref(&enrollment).len();
        let mut last_sent: Option<usize> = None;
        for event in &enrollment.events {
            let one = std::slice::from_ref(event).len();
            if let Some(step_id) = &event.step_id {
                let Some(position) = steps.iter().position(|step| &step.step_id == step_id) else {
                    bail!(
                        "enrollment {} records a {} event for step {step_id}, which sequence {} does not hold",
                        enrollment.id,
                        event.kind,
                        sequence.id
                    );
                };
                match event.kind.as_str() {
                    "sent" => {
                        steps[position].sent += one;
                        last_sent = Some(position);
                    }
                    "skipped" => steps[position].skipped += one,
                    _ => {}
                }
            }
            if TERMINAL_EVENT_TYPES.contains(&event.kind.as_str()) {
                let target = match last_sent {
                    Some(position) => &mut steps[position],
                    None => &mut before_any_step,
                };
                match event.kind.as_str() {
                    "replied" => target.replied += one,
                    "bounced" => target.bounced += one,
                    _ => target.unsubscribed += one,
                }
            }
        }
    }
    Ok(SequenceReport { sequence_id: sequence.id.clone(), enrollments: count, statuses, steps, before_any_step })
}
