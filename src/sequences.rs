//! What a sequence is, who is enrolled in it, what happened to them, and
//! what the planner answers with.
//!
//! The operations are the submodules: normalising a sequence, enrolling a
//! contact and recording what happened, planning what is due, and rendering
//! a template.

pub mod enrollment;
pub mod normalize;
pub mod plan;
pub mod template;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub(crate) const TERMINAL_EVENT_TYPES: &[&str] = &["replied", "bounced", "unsubscribed"];
pub(crate) const STEP_EVENT_TYPES: &[&str] = &["sent", "skipped"];

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
