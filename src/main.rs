use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use email_sequencing_cli::{
    Enrollment, EnrollmentEvent, EnrollmentSeed, SequenceInput, create_enrollment, next_actions,
    normalize_sequence, parse_optional_date, plan_enrollment, record_event, render_template,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

#[derive(Parser)]
#[command(
    name = "email-sequencing",
    version,
    about = "Local-first email sequence planning and execution-state CLI",
    after_help = "Commands print JSON to stdout and never send email."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Enroll {
        #[arg(long)]
        sequence: PathBuf,
        #[arg(long)]
        enrollment: PathBuf,
        #[arg(long)]
        at: Option<String>,
    },
    Plan {
        #[arg(long)]
        sequence: PathBuf,
        #[arg(long)]
        enrollments: PathBuf,
        #[arg(long)]
        at: Option<String>,
    },
    Schedule {
        #[arg(long)]
        sequence: PathBuf,
        #[arg(long)]
        enrollment: PathBuf,
    },
    Record {
        #[arg(long)]
        enrollment: PathBuf,
        #[arg(long)]
        event: PathBuf,
    },
    Render {
        #[arg(long)]
        template: PathBuf,
        #[arg(long)]
        variables: PathBuf,
    },
}

fn read_json<T: DeserializeOwned>(path: &PathBuf) -> Result<T> {
    let input = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&input).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn output<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn run() -> Result<()> {
    match Cli::parse().command {
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
            )?)
        }
        Command::Plan {
            sequence,
            enrollments,
            at,
        } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            let enrollments = read_json::<Vec<Enrollment>>(&enrollments)?;
            let at = parse_optional_date(at.as_deref(), "--at")?.unwrap_or_else(Utc::now);
            output(&next_actions(&sequence, enrollments, at)?)
        }
        Command::Schedule {
            sequence,
            enrollment,
        } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            output(&plan_enrollment(
                &sequence,
                read_json::<Enrollment>(&enrollment)?,
            )?)
        }
        Command::Record { enrollment, event } => output(&record_event(
            read_json::<Enrollment>(&enrollment)?,
            read_json::<EnrollmentEvent>(&event)?,
        )?),
        Command::Render {
            template,
            variables,
        } => {
            let template = fs::read_to_string(&template)
                .with_context(|| format!("failed to read {}", template.display()))?;
            let variables = read_json::<Map<String, Value>>(&variables)?;
            output(&render_template(&template, &variables)?)
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
