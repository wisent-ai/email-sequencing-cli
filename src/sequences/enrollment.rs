//! Enrolling a contact and keeping the enrollment true: creating one from a
//! seed, reading one as given, and recording an event that may end it.

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};

use super::{Contact, Enrollment, EnrollmentEvent, EnrollmentSeed, EnrollmentStatus, Sequence};
use super::normalize::{format_date, optional, parse_date, required};
use super::{STEP_EVENT_TYPES, TERMINAL_EVENT_TYPES};

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
