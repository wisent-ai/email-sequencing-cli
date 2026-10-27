//! `email-sequencing status` through the built binary: an enrollment paused
//! with a reason keeps the change as an event, resumes, is cancelled, and
//! then refuses every further change; completing by hand and setting the
//! status it has are refused.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::{Command, Output};

fn workspace(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).expect("the test's directory");
    dir
}

/// `status` on the enrollment in `file`, with `args` after it.
fn status(file: &PathBuf, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["status", "--enrollment"])
        .arg(file)
        .args(args)
        .output()
        .expect("the binary runs")
}

/// The enrollment `status` printed, written back to `file` for the next step.
fn kept(file: &PathBuf, answer: Output) -> Value {
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    std::fs::write(file, &answer.stdout).expect("the enrollment written back");
    serde_json::from_slice(&answer.stdout).expect("JSON on stdout")
}

fn refusal(answer: Output) -> String {
    assert!(!answer.status.success(), "the change was refused");
    String::from_utf8_lossy(&answer.stderr).into_owned()
}

#[test]
fn an_enrollment_is_paused_resumed_and_cancelled_by_hand() {
    let dir = workspace("status-moves");
    let file = dir.join("enrollment.json");
    let enrolled = chrono::Utc::now().to_rfc3339();
    std::fs::write(&file, json!({"id": "ada", "sequenceId": "welcome", "contact": {"id": "ada"}, "enrolledAt": enrolled}).to_string())
        .expect("the enrollment");

    let paused = kept(&file, status(&file, &["--set", "paused", "--reason", "on leave"]));
    assert_eq!(paused["status"], "paused");
    let event = paused["events"].as_array().and_then(|events| events.last()).expect("the status event");
    assert_eq!(event["type"], "paused");
    assert_eq!(event["reason"], "on leave");

    assert!(refusal(status(&file, &["--set", "paused"])).contains("enrollment ada is paused already"));
    assert!(refusal(status(&file, &["--set", "completed"])).contains("an enrollment completes when a reply"));
    assert!(refusal(status(&file, &["--set", "sleeping"])).contains("is none of active, paused or cancelled"));

    assert_eq!(kept(&file, status(&file, &["--set", "active"]))["status"], "active");
    assert_eq!(kept(&file, status(&file, &["--set", "cancelled"]))["status"], "cancelled");
    assert!(refusal(status(&file, &["--set", "active"])).contains("enrollment ada is cancelled; enroll the contact again"));
}
