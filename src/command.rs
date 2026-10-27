//! The command line's words: every command and its options, as clap reads
//! them. `main` answers each.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "email-sequencing",
    version,
    about = "Local-first email sequence planning and execution-state CLI",
    after_help = "Commands never send email. Every command prints JSON, or with --text one `path: value` line per field from the same data. Exit 2: the invocation is wrong; exit 1: an input could not be read or was refused."
)]
pub(crate) struct Cli {
    /// Print one `path: value` line per field instead of JSON.
    #[arg(long, global = true)]
    pub(crate) text: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Start a contact on a sequence and print the new enrollment.
    Enroll {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// The enrollment seed JSON file (id and contact).
        #[arg(long)]
        enrollment: PathBuf,
        /// When the enrollment starts (RFC 3339); otherwise the seed's enrolledAt, otherwise now.
        #[arg(long)]
        at: Option<String>,
    },
    /// List the steps that are due across many enrollments.
    Plan {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// A JSON array of enrollments.
        #[arg(long)]
        enrollments: PathBuf,
        /// The moment due steps are computed for (RFC 3339); now when omitted.
        #[arg(long)]
        at: Option<String>,
    },
    /// Print every step of one enrollment with the time it falls due.
    Schedule {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// The enrollment JSON file.
        #[arg(long)]
        enrollment: PathBuf,
    },
    /// Write out every step of one enrollment for its contact, sent, skipped
    /// or still to come: the address, the subject and body filled from the
    /// contact's variables, and what is missing; nothing is sent.
    Preview {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// The enrollment JSON file.
        #[arg(long)]
        enrollment: PathBuf,
    },
    /// Before a sequence starts sending: per step, the enrollments whose
    /// contact has no address and, per variable the step names, those whose
    /// contact lacks it.
    Check {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// A JSON array of enrollments.
        #[arg(long)]
        enrollments: PathBuf,
    },
    /// Every contact a reply, a bounce or an unsubscribe stopped, across
    /// sequences: the list to check before writing to them again.
    Stopped {
        /// A JSON array of enrollments, of any sequences.
        #[arg(long)]
        enrollments: PathBuf,
    },
    /// Add one event (sent, skipped, replied, bounced, unsubscribed) to an enrollment and print it.
    Record {
        /// The enrollment JSON file.
        #[arg(long)]
        enrollment: PathBuf,
        /// The event JSON file.
        #[arg(long)]
        event: PathBuf,
    },
    /// Record many events across enrollments at once, a mailer's send log or
    /// a reply feed, all or none, and print the whole set.
    RecordAll {
        /// A JSON array of enrollments.
        #[arg(long)]
        enrollments: PathBuf,
        /// A JSON array of {enrollmentId, event}, each event as `record` reads one.
        #[arg(long)]
        events: PathBuf,
    },
    /// Fill a template's variables and print the result.
    Render {
        /// The template text file.
        #[arg(long)]
        template: PathBuf,
        /// A JSON object of variable values.
        #[arg(long)]
        variables: PathBuf,
    },
    /// How a sequence is doing: enrollments per status, and per step the
    /// messages sent and skipped and the replies, bounces and unsubscribes
    /// that followed them.
    Report {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// A JSON array of the sequence's enrollments.
        #[arg(long)]
        enrollments: PathBuf,
    },
    /// Pause, resume or cancel an enrollment by hand and print it, the
    /// change kept as an event with its moment and reason.
    Status {
        /// The enrollment JSON file.
        #[arg(long)]
        enrollment: PathBuf,
        /// The status to move it to: active, paused or cancelled.
        #[arg(long)]
        set: String,
        /// When it changed (RFC 3339); now when omitted.
        #[arg(long)]
        at: Option<String>,
        /// Why, kept on the event.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Start every contact of a list on a sequence, all or none, and print
    /// the new enrollments.
    EnrollAll {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// The seeds: a JSON array, each as `enroll` reads one, or a `.csv`
        /// sheet whose header names the seed fields (`id`, `contact.id`,
        /// `contact.email`, `contact.variables.<name>`, `enrolledAt`).
        #[arg(long)]
        seeds: PathBuf,
        /// When they were enrolled (RFC 3339); each seed's `enrolledAt`,
        /// else now, when omitted.
        #[arg(long)]
        at: Option<String>,
    },
    /// Write out every message due across a sequence's enrollments: the
    /// address, the subject and body filled from the contact's variables,
    /// and what is missing; nothing is sent.
    Outbox {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// A JSON array of enrollments.
        #[arg(long)]
        enrollments: PathBuf,
        /// The moment the messages are due by (RFC 3339); now when omitted.
        #[arg(long)]
        at: Option<String>,
    },
    /// Count the messages that fall due on each of the coming days, so a
    /// sender's daily quota can be checked ahead; nothing is sent.
    Forecast {
        /// The sequence JSON file.
        #[arg(long)]
        sequence: PathBuf,
        /// A JSON array of enrollments.
        #[arg(long)]
        enrollments: PathBuf,
        /// How many UTC days to count, the first being the day of --at.
        #[arg(long)]
        days: i64,
        /// The moment the forecast starts (RFC 3339); now when omitted.
        #[arg(long)]
        at: Option<String>,
    },
    /// Take a contact off every sequence at once: cancel each of its live
    /// enrollments, by contact id or email, the reason kept on each; prints
    /// what changed and the whole set to write back.
    OptOut {
        /// A JSON array of enrollments, of any sequences.
        #[arg(long)]
        enrollments: PathBuf,
        /// The contact: its id or its email.
        #[arg(long)]
        contact: String,
        /// Why, kept on every cancelled enrollment.
        #[arg(long)]
        reason: String,
        /// When (RFC 3339); now when omitted.
        #[arg(long)]
        at: Option<String>,
    },
}
