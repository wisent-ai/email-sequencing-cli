//! A person who asks to hear no more is taken off every sequence at once:
//! every enrollment of the contact, named by contact id or email (case set
//! aside), still active or paused is cancelled as `status --set cancelled`
//! cancels one, the reason kept on each. Enrollments already completed or
//! cancelled are left as they are and named, so nothing reopens.

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::operations::status::set_status;
use crate::sequences::enrollment::normalize_enrollment;
use crate::sequences::{Enrollment, EnrollmentStatus};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptOut {
    /// The enrollments this cancelled, in the set's order.
    pub cancelled: Vec<String>,
    /// The contact's enrollments that were already completed or cancelled.
    pub already_ended: Vec<String>,
    /// The whole set, to write back.
    pub enrollments: Vec<Enrollment>,
}

/// `enrollments` with every live enrollment of the contact `who` (an id or
/// an email) cancelled at `at`; refused when no enrollment names the contact.
pub fn opt_out(enrollments: Vec<Enrollment>, who: &str, at: DateTime<Utc>, reason: &str) -> Result<OptOut> {
    let who = who.trim();
    if who.is_empty() {
        bail!("name the contact to take off every sequence: its id or its email");
    }
    if reason.trim().is_empty() {
        bail!("say why the contact is taken off, such as the unsubscribe it sent; the reason is kept on every enrollment");
    }
    let mut cancelled = Vec::new();
    let mut already_ended = Vec::new();
    let mut result = Vec::with_capacity(enrollments.len());
    for input in enrollments {
        let enrollment = normalize_enrollment(input)?;
        let email = enrollment.contact.email.as_deref().map(str::to_lowercase);
        let theirs = enrollment.contact.id == who || email.as_deref() == Some(who.to_lowercase().as_str());
        if !theirs {
            result.push(enrollment);
            continue;
        }
        if matches!(enrollment.status, EnrollmentStatus::Completed | EnrollmentStatus::Cancelled) {
            already_ended.push(enrollment.id.clone());
            result.push(enrollment);
            continue;
        }
        cancelled.push(enrollment.id.clone());
        result.push(set_status(enrollment, EnrollmentStatus::Cancelled, at, Some(reason.trim().to_owned()))?);
    }
    if cancelled.is_empty() && already_ended.is_empty() {
        bail!("no enrollment names contact {who}; nothing was changed");
    }
    Ok(OptOut { cancelled, already_ended, enrollments: result })
}
