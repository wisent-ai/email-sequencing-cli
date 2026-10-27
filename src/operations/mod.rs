//! What is done across enrollments rather than to one sequence's shape:
//! enrolling many at once, moving one between statuses by hand, writing out
//! the messages due, and reporting how a sequence is doing.

pub mod bulk;
pub mod outbox;
pub mod report;
pub mod status;
