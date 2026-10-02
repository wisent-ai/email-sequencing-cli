use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use email_sequencing_cli::{
    Enrollment, EnrollmentEvent, EnrollmentSeed, SequenceInput, create_enrollment, next_actions,
    normalize_sequence, parse_optional_date, plan_enrollment, record_event, render_template,
};
use serde_json::{Map, Value};
use wisent_errors::cli_output::{output, read_json};

#[derive(Parser)]
#[command(
    name = "email-sequencing",
    version,
    about = "Local-first email sequence planning and execution-state CLI",
    after_help = "Commands never send email. Every command prints JSON, or with --text one `path: value` line per field from the same data. Exit 2: the invocation is wrong; exit 1: an input could not be read or was refused."
)]
struct Cli {
    /// Print one `path: value` line per field instead of JSON.
    #[arg(long, global = true)]
    text: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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
    /// Add one event (sent, skipped, replied, bounced, unsubscribed) to an enrollment and print it.
    Record {
        /// The enrollment JSON file.
        #[arg(long)]
        enrollment: PathBuf,
        /// The event JSON file.
        #[arg(long)]
        event: PathBuf,
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
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let text = cli.text;
    match cli.command {
        Command::Enroll {
            sequence,
            enrollment,
            at,
        } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            let enrollment = read_json::<EnrollmentSeed>(&enrollment)?;
            output(&create_enrollment(
                &sequence,
                enrollment,
                parse_optional_date(at.as_deref(), "--at")?,
            )?, text)
        }
        Command::Plan {
            sequence,
            enrollments,
            at,
        } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            let enrollments = read_json::<Vec<Enrollment>>(&enrollments)?;
            let at = parse_optional_date(at.as_deref(), "--at")?.unwrap_or_else(Utc::now);
            output(&next_actions(&sequence, enrollments, at)?, text)
        }
        Command::Schedule {
            sequence,
            enrollment,
        } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            output(&plan_enrollment(
                &sequence,
                read_json::<Enrollment>(&enrollment)?,
            )?, text)
        }
        Command::Record { enrollment, event } => output(&record_event(
            read_json::<Enrollment>(&enrollment)?,
            read_json::<EnrollmentEvent>(&event)?,
        )?, text),
        Command::Render {
            template,
            variables,
        } => {
            let template = fs::read_to_string(&template)
                .with_context(|| format!("failed to read {}", template.display()))?;
            let variables = read_json::<Map<String, Value>>(&variables)?;
            output(&render_template(&template, &variables)?, text)
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
