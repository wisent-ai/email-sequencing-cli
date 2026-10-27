//! Moving an enrollment between statuses by hand: pausing it while the
//! contact is away, resuming it, or cancelling it. `record` ends an
//! enrollment when a reply, bounce or unsubscribe arrives; this is the
//! operator's own decision, kept as an event so the enrollment says when and
//! why it changed.

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};

use crate::sequences::enrollment::normalize_enrollment;
use crate::sequences::normalize::format_date;
use crate::sequences::{Enrollment, EnrollmentEvent, EnrollmentStatus};

/// The status `wanted` names.
pub fn parse_status(wanted: &str) -> Result<EnrollmentStatus> {
    match wanted.trim() {
        "active" => Ok(EnrollmentStatus::Active),
        "paused" => Ok(EnrollmentStatus::Paused),
        "cancelled" => Ok(EnrollmentStatus::Cancelled),
        "completed" => bail!(
            "an enrollment completes when a reply, bounce or unsubscribe is recorded with `record`; set active, paused or cancelled"
        ),
        other => bail!("status {other:?} is none of active, paused or cancelled"),
    }
}

fn name(status: &EnrollmentStatus) -> &'static str {
    match status {
        EnrollmentStatus::Active => "active",
        EnrollmentStatus::Paused => "paused",
        EnrollmentStatus::Completed => "completed",
        EnrollmentStatus::Cancelled => "cancelled",
    }
}

/// The enrollment in `status` from `at`, with a `status` event saying so
/// and why; refused for a completed or cancelled enrollment, which nothing
/// reopens, and for the status it already has.
pub fn set_status(
    input: Enrollment,
    status: EnrollmentStatus,
    at: DateTime<Utc>,
    reason: Option<String>,
) -> Result<Enrollment> {
    let mut enrollment = normalize_enrollment(input)?;
    if matches!(enrollment.status, EnrollmentStatus::Completed | EnrollmentStatus::Cancelled) {
        bail!(
            "enrollment {} is {}; enroll the contact again to start the sequence over",
            enrollment.id,
            name(&enrollment.status)
        );
    }
    if name(&enrollment.status) == name(&status) {
        bail!("enrollment {} is {} already", enrollment.id, name(&status));
    }
    enrollment.events.push(EnrollmentEvent {
        kind: name(&status).to_owned(),
        at: format_date(at),
        step_id: None,
        message_id: None,
        reason: reason.map(|text| text.trim().to_owned()).filter(|text| !text.is_empty()),
    });
    enrollment.status = status;
    normalize_enrollment(enrollment)
}

/// A contact a sequence stopped writing to, and the event that stopped it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stopped {
    pub contact_id: String,
    pub email: Option<String>,
    pub enrollment_id: String,
    pub sequence_id: String,
    /// The reply, bounce or unsubscribe that ended the enrollment.
    pub event: EnrollmentEvent,
}

/// Every contact whose enrollment a reply, a bounce or an unsubscribe ended,
/// across all `enrollments`, the earliest stop first: the list a mailer
/// checks before writing to someone again from another sequence.
pub fn stopped(enrollments: Vec<Enrollment>) -> Result<Vec<Stopped>> {
    let mut found = Vec::new();
    for input in enrollments {
        let enrollment = normalize_enrollment(input)?;
        let ended = enrollment
            .events
            .iter()
            .find(|event| crate::sequences::TERMINAL_EVENT_TYPES.contains(&event.kind.as_str()));
        if let Some(event) = ended {
            found.push(Stopped {
                contact_id: enrollment.contact.id.clone(),
                email: enrollment.contact.email.clone(),
                enrollment_id: enrollment.id.clone(),
                sequence_id: enrollment.sequence_id.clone(),
                event: event.clone(),
            });
        }
    }
    found.sort_by(|left, right| left.event.at.cmp(&right.event.at).then_with(|| left.enrollment_id.cmp(&right.enrollment_id)));
    Ok(found)
}
