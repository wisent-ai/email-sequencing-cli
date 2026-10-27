//! Messages written out for their contact. The outbox: every message due at a
//! moment across a sequence's enrollments, as `plan` finds them. The preview:
//! every step of one enrollment, sent, skipped or still to come, the way its
//! contact reads it, so a sequence can be read through before anyone is
//! enrolled for real. Each message carries the address it goes to, its subject
//! and body with the contact's variables filled, and the variables the contact
//! lacks, so a message that cannot go yet says why. Nothing is sent and nothing
//! is recorded; `record` keeps a `sent` event once the operator's own mailer
//! sent it.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::sequences::plan::{next_actions, plan_enrollment};
use crate::sequences::template::render_template;
use crate::sequences::{Enrollment, PlannedState, PlannedStep, Sequence};

/// One message, ready for a mailer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingMessage {
    pub enrollment_id: String,
    pub contact_id: String,
    pub step_id: String,
    pub due_at: String,
    /// `sent`, `skipped` or `pending`; the outbox holds pending ones only.
    pub state: PlannedState,
    /// The contact's address; `null` when the enrollment holds none.
    pub to: Option<String>,
    pub subject: Option<String>,
    pub body: String,
    /// Variables the subject or body names that the contact does not hold.
    pub missing: Vec<String>,
    /// Whether the message can go: an address and every variable held.
    pub ready: bool,
}

/// `step` written out for the contact `enrollment` holds.
fn write_out(step: PlannedStep, enrollment: &Enrollment) -> Result<OutgoingMessage> {
    let variables = &enrollment.contact.variables;
    let body = render_template(&step.template, variables)?;
    let mut missing = body.missing;
    let subject = match &step.subject {
        Some(subject) => {
            let rendered = render_template(subject, variables)?;
            missing.extend(rendered.missing);
            Some(rendered.text)
        }
        None => None,
    };
    missing.sort();
    missing.dedup();
    let to = enrollment.contact.email.clone().filter(|address| !address.trim().is_empty());
    Ok(OutgoingMessage {
        ready: to.is_some() && missing.is_empty(),
        enrollment_id: step.enrollment_id,
        contact_id: step.contact_id,
        step_id: step.step_id,
        due_at: step.due_at,
        state: step.state,
        to,
        subject,
        body: body.text,
        missing,
    })
}

/// Every message due at `at`, soonest first.
pub fn outbox(sequence: &Sequence, enrollments: Vec<Enrollment>, at: DateTime<Utc>) -> Result<Vec<OutgoingMessage>> {
    let contacts: HashMap<String, Enrollment> =
        enrollments.iter().map(|enrollment| (enrollment.id.trim().to_owned(), enrollment.clone())).collect();
    let mut messages = Vec::new();
    for step in next_actions(sequence, enrollments, at)? {
        let Some(enrollment) = contacts.get(&step.enrollment_id) else {
            anyhow::bail!("the plan names enrollment {}, which the enrollments file does not hold", step.enrollment_id);
        };
        messages.push(write_out(step, enrollment)?);
    }
    Ok(messages)
}

/// Every step of `enrollment`, in the sequence's order, written out for its
/// contact whatever its state.
pub fn preview(sequence: &Sequence, enrollment: Enrollment) -> Result<Vec<OutgoingMessage>> {
    plan_enrollment(sequence, enrollment.clone())?
        .into_iter()
        .map(|step| write_out(step, &enrollment))
        .collect()
}

/// One step, and the enrollments its message cannot go to yet.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReadiness {
    pub step_id: String,
    /// Enrollments whose contact holds no address.
    pub without_address: Vec<String>,
    /// Per variable the step names, the enrollments whose contact lacks it.
    pub missing: std::collections::BTreeMap<String, Vec<String>>,
}

/// Every step of `sequence` written out for every enrollment, whatever its
/// state or due time, and what would stop each message: the readiness to
/// check before a sequence starts sending, not the messages due now.
pub fn readiness(sequence: &Sequence, enrollments: Vec<Enrollment>) -> Result<Vec<StepReadiness>> {
    let mut steps: Vec<StepReadiness> = sequence
        .steps
        .iter()
        .map(|step| StepReadiness { step_id: step.id.clone(), without_address: Vec::new(), missing: Default::default() })
        .collect();
    for enrollment in enrollments {
        for message in preview(sequence, enrollment)? {
            let Some(step) = steps.iter_mut().find(|step| step.step_id == message.step_id) else {
                anyhow::bail!("the plan names step {}, which sequence {} does not hold", message.step_id, sequence.id);
            };
            if message.to.is_none() {
                step.without_address.push(message.enrollment_id.clone());
            }
            for variable in message.missing {
                step.missing.entry(variable).or_default().push(message.enrollment_id.clone());
            }
        }
    }
    Ok(steps)
}
