//! `email-sequencing forecast` through the built binary: a two-step sequence,
//! the second step a day after the first; one enrollment enrolled a day ago
//! (its first step overdue, its second due today) and one enrolled today (its
//! first step today, its second tomorrow); a replied enrollment counts for
//! nothing. Then a window of no days is refused.

use chrono::{NaiveTime, TimeDelta, Utc};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::{Command, Output};

fn forecast(sequence: &PathBuf, enrollments: &PathBuf, at: &str, days: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["forecast", "--sequence"])
        .arg(sequence)
        .arg("--enrollments")
        .arg(enrollments)
        .args(["--at", at, "--days", days])
        .output()
        .expect("the binary runs")
}

#[test]
fn the_coming_days_count_the_messages_due_on_each() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forecast");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let day = TimeDelta::days(std::slice::from_ref(&dir).len() as i64);
    let one = day.num_days();
    let sequence = dir.join("sequence.json");
    std::fs::write(
        &sequence,
        json!({"id": "welcome", "steps": [
            {"id": "intro", "template": "Hi"},
            {"id": "follow", "afterHours": day.num_hours(), "template": "Still there?"}
        ]})
        .to_string(),
    )
    .expect("the sequence");
    // A minute into today's UTC day: every step due later today is due after it.
    let midnight = Utc::now().date_naive().and_time(NaiveTime::MIN).and_utc();
    let at = midnight + TimeDelta::minutes(one);
    let enrollment = |id: &str, enrolled: chrono::DateTime<Utc>, events: Value| {
        json!({"id": id, "sequenceId": "welcome", "contact": {"id": id}, "enrolledAt": enrolled.to_rfc3339(), "status": "active", "events": events})
    };
    let enrollments = dir.join("enrollments.json");
    std::fs::write(
        &enrollments,
        json!([
            enrollment("yesterday", at - day + TimeDelta::hours(one), json!([])),
            enrollment("today", at + TimeDelta::hours(one), json!([])),
            enrollment("replied", at, json!([{"type": "replied", "at": at.to_rfc3339()}]))
        ])
        .to_string(),
    )
    .expect("the enrollments");

    let window = (one + one).to_string();
    let answer = forecast(&sequence, &enrollments, &at.to_rfc3339(), &window);
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let result: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let counted: Vec<(String, i64)> = result["days"]
        .as_array()
        .expect("days")
        .iter()
        .map(|entry| (entry["day"].as_str().expect("a day").to_owned(), entry["messages"].as_i64().expect("a count")))
        .collect();
    let today = midnight.date_naive().to_string();
    let tomorrow = (midnight + day).date_naive().to_string();
    assert_eq!(counted, vec![(today, one + one), (tomorrow, one)], "two today, one tomorrow, nothing for the replied one");
    assert_eq!(result["overdue"], json!(one), "yesterday's first step is already due");
    assert_eq!(result["peak"], json!(one + one), "today is the busiest day");

    let refused = forecast(&sequence, &enrollments, &at.to_rfc3339(), &(one - one).to_string());
    assert!(!refused.status.success(), "a window of no days is refused");
}
