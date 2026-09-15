//! Email sequences: what a sequence is, who is enrolled, what happened to
//! them, what is due next, and what a rendered message says.
//!
//! The shapes and the operations live in `sequences`; this file is the
//! surface the CLI and any other caller uses, so the crate's API is one list.

pub mod sequences;

pub use sequences::{
    Contact, ContactInput, Enrollment, EnrollmentEvent, EnrollmentSeed, EnrollmentStatus,
    PlannedState, PlannedStep, RenderedTemplate, Sequence, SequenceInput, SequenceStep,
    SequenceStepInput,
};
pub use sequences::enrollment::{create_enrollment, normalize_enrollment, record_event};
pub use sequences::normalize::{normalize_sequence, parse_optional_date};
pub use sequences::plan::{next_actions, plan_enrollment};
pub use sequences::template::render_template;
