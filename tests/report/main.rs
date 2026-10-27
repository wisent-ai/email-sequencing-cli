//! `email-sequencing report` through the built binary: two enrollments of
//! one sequence, one replied after the first step and one bounced after the
//! second, counted per step and per status; an enrollment of another
//! sequence and an event naming a step the sequence lacks are refused.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

/// A directory of this run's input files under the build directory.
fn workspace(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).expect("the test's directory");
    dir
}

fn write(dir: &PathBuf, file: &str, value: &Value) -> PathBuf {
    let path = dir.join(file);
    std::fs::write(&path, value.to_string()).expect("an input file");
    path
}

fn report(sequence: &PathBuf, enrollments: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["report", "--sequence"])
        .arg(sequence)
        .arg("--enrollments")
        .arg(enrollments)
        .output()
        .expect("the binary runs")
}

fn enrollment(id: &str, sequence: &str, status: &str, events: Value) -> Value {
    let enrolled = chrono::Utc::now().to_rfc3339();
    json!({"id": id, "sequenceId": sequence, "contact": {"id": id}, "enrolledAt": enrolled, "status": status, "events": events})
}

#[test]
fn a_report_counts_each_step_and_what_followed_it() {
    let dir = workspace("report-counts");
    let sequence = write(
        &dir,
        "sequence.json",
        &json!({"id": "welcome", "steps": [{"id": "intro", "template": "Hi"}, {"id": "nudge", "template": "Still there?"}]}),
    );
    let now = chrono::Utc::now();
    let at = |offset: chrono::TimeDelta| (now + offset).to_rfc3339();
    let minute = chrono::TimeDelta::minutes(std::slice::from_ref(&now).len() as i64);
    let replied = enrollment(
        "ada",
        "welcome",
        "completed",
        json!([{"type": "sent", "at": at(minute), "stepId": "intro"}, {"type": "replied", "at": at(minute + minute)}]),
    );
    let bounced = enrollment(
        "bo",
        "welcome",
        "completed",
        json!([
            {"type": "sent", "at": at(minute), "stepId": "intro"},
            {"type": "sent", "at": at(minute + minute), "stepId": "nudge"},
            {"type": "bounced", "at": at(minute + minute + minute)}
        ]),
    );
    let fresh = enrollment("cy", "welcome", "active", json!([]));
    let enrollments = write(&dir, "enrollments.json", &json!([replied, bounced, fresh]));

    let answer = report(&sequence, &enrollments);
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let body: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let one = std::slice::from_ref(&body).len();
    assert_eq!(body["enrollments"], json!(one + one + one));
    assert_eq!(body["statuses"]["completed"], json!(one + one));
    assert_eq!(body["statuses"]["active"], json!(one));
    assert_eq!(body["steps"][one - one]["stepId"], "intro");
    assert_eq!(body["steps"][one - one]["sent"], json!(one + one), "both were sent the intro");
    assert_eq!(body["steps"][one - one]["replied"], json!(one), "Ada replied after the intro");
    assert_eq!(body["steps"][one]["bounced"], json!(one), "Bo's address bounced after the nudge");
    assert_eq!(body["beforeAnyStep"]["replied"], json!(one - one));

    let stranger = write(&dir, "stranger.json", &json!([enrollment("di", "other", "active", json!([]))]));
    let refused = report(&sequence, &stranger);
    assert!(!refused.status.success(), "an enrollment of another sequence is refused");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("enrollment di is in sequence other, not welcome"));

    let unknown = write(
        &dir,
        "unknown.json",
        &json!([enrollment("ed", "welcome", "active", json!([{"type": "sent", "at": at(minute), "stepId": "gone"}]))]),
    );
    let refused = report(&sequence, &unknown);
    assert!(!refused.status.success(), "a step the sequence lacks is refused");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("step gone, which sequence welcome does not hold"));
}
