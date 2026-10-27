//! Email sequences: what a sequence is, who is enrolled, what happened to
//! them, what is due next, and what a rendered message says.
//!
//! The shapes and the operations on one enrollment live in `sequences`,
//! those across enrollments in `operations`; this file is the surface the
//! CLI and any other caller uses, so the crate's API is one list.

pub mod forecast;
pub mod operations;
pub mod optout;
pub mod sequences;

pub use sequences::{
    Contact, ContactInput, Enrollment, EnrollmentEvent, EnrollmentSeed, EnrollmentStatus,
    PlannedState, PlannedStep, RenderedTemplate, Sequence, SequenceInput, SequenceStep,
    SequenceStepInput,
};
pub use sequences::enrollment::{create_enrollment, normalize_enrollment, record_event};
pub use sequences::normalize::{normalize_sequence, parse_optional_date};
pub use sequences::plan::{next_actions, plan_enrollment};
pub use forecast::{Forecast, ForecastDay, forecast};
pub use optout::{OptOut, opt_out};
pub use operations::bulk::{EnrollmentEventEntry, enroll_all, record_all, seeds_from_sheet};
pub use operations::outbox::{OutgoingMessage, StepReadiness, outbox, preview, readiness};
pub use operations::report::{SequenceReport, StatusCounts, StepReport, report};
pub use operations::status::{Stopped, parse_status, set_status, stopped};
pub use sequences::template::render_template;
