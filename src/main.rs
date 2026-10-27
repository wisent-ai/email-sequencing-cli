mod command;

use std::fs;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use command::{Cli, Command};
use email_sequencing_cli::{
    Enrollment, EnrollmentEvent, EnrollmentEventEntry, EnrollmentSeed, SequenceInput, create_enrollment, next_actions,
    normalize_sequence, parse_optional_date, parse_status, plan_enrollment, record_event, render_template, report,
    enroll_all, forecast, opt_out, outbox, preview, readiness, record_all, seeds_from_sheet, set_status, stopped,
};
use serde_json::{Map, Value};
use wisent_errors::cli_output::{output, read_json};

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
            // `--at` names the moment; the documented moment without it is now.
            let at = match parse_optional_date(at.as_deref(), "--at")? {
                Some(moment) => moment,
                None => Utc::now(),
            };
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
        Command::Preview { sequence, enrollment } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            output(&preview(&sequence, read_json::<Enrollment>(&enrollment)?)?, text)
        }
        Command::Check { sequence, enrollments } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            output(&readiness(&sequence, read_json::<Vec<Enrollment>>(&enrollments)?)?, text)
        }
        Command::Stopped { enrollments } => output(&stopped(read_json::<Vec<Enrollment>>(&enrollments)?)?, text),
        Command::Record { enrollment, event } => output(&record_event(
            read_json::<Enrollment>(&enrollment)?,
            read_json::<EnrollmentEvent>(&event)?,
        )?, text),
        Command::RecordAll { enrollments, events } => output(&record_all(
            read_json::<Vec<Enrollment>>(&enrollments)?,
            read_json::<Vec<EnrollmentEventEntry>>(&events)?,
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
        Command::Report { sequence, enrollments } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            output(&report(&sequence, read_json::<Vec<Enrollment>>(&enrollments)?)?, text)
        }
        Command::Status { enrollment, set, at, reason } => {
            let at = match parse_optional_date(at.as_deref(), "--at")? {
                Some(moment) => moment,
                None => Utc::now(),
            };
            output(&set_status(read_json::<Enrollment>(&enrollment)?, parse_status(&set)?, at, reason)?, text)
        }
        Command::EnrollAll { sequence, seeds, at } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            // A `.csv` file is a contact sheet; anything else is the JSON array of seeds.
            let seeds = match seeds.extension() {
                Some(extension) if extension.eq_ignore_ascii_case("csv") => seeds_from_sheet(
                    &std::fs::read_to_string(&seeds)
                        .map_err(|error| anyhow::anyhow!("could not read {}: {error}", seeds.display()))?,
                )?,
                _ => read_json::<Vec<EnrollmentSeed>>(&seeds)?,
            };
            output(&enroll_all(&sequence, seeds, parse_optional_date(at.as_deref(), "--at")?)?, text)
        }
        Command::Outbox { sequence, enrollments, at } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            let enrollments = read_json::<Vec<Enrollment>>(&enrollments)?;
            let at = match parse_optional_date(at.as_deref(), "--at")? {
                Some(moment) => moment,
                None => Utc::now(),
            };
            output(&outbox(&sequence, enrollments, at)?, text)
        }
        Command::Forecast { sequence, enrollments, days, at } => {
            let sequence = normalize_sequence(read_json::<SequenceInput>(&sequence)?)?;
            let enrollments = read_json::<Vec<Enrollment>>(&enrollments)?;
            let at = match parse_optional_date(at.as_deref(), "--at")? {
                Some(moment) => moment,
                None => Utc::now(),
            };
            output(&forecast(&sequence, enrollments, at, days)?, text)
        }
        Command::OptOut { enrollments, contact, reason, at } => {
            let enrollments = read_json::<Vec<Enrollment>>(&enrollments)?;
            let at = match parse_optional_date(at.as_deref(), "--at")? {
                Some(moment) => moment,
                None => Utc::now(),
            };
            output(&opt_out(enrollments, &contact, at, &reason)?, text)
        }
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
