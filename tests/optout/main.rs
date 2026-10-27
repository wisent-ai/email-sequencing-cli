//! `email-sequencing opt-out` through the built binary: one person on two
//! sequences, named by email in another case, is cancelled on the live one
//! and left on the one already cancelled; someone else's enrollment is not
//! touched; and a contact no enrollment names is refused.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::{Command, Output};

fn opt_out(enrollments: &PathBuf, contact: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["opt-out", "--enrollments"])
        .arg(enrollments)
        .args(["--contact", contact, "--reason", "unsubscribed by email"])
        .output()
        .expect("the binary runs")
}

#[test]
fn a_contact_is_taken_off_every_sequence_at_once() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("opt-out");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let at = chrono::Utc::now().to_rfc3339();
    let enrollment = |id: &str, sequence: &str, contact: &str, email: &str, status: &str| {
        json!({"id": id, "sequenceId": sequence, "contact": {"id": contact, "email": email}, "enrolledAt": at, "status": status})
    };
    let enrollments = dir.join("enrollments.json");
    std::fs::write(
        &enrollments,
        json!([
            enrollment("e-welcome", "welcome", "ada", "ada@example.org", "active"),
            enrollment("e-renewal", "renewal", "ada", "ada@example.org", "cancelled"),
            enrollment("e-bo", "welcome", "bo", "bo@example.org", "active")
        ])
        .to_string(),
    )
    .expect("the enrollments");

    let answer = opt_out(&enrollments, "ADA@Example.org");
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let result: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    assert_eq!(result["cancelled"], json!(["e-welcome"]));
    assert_eq!(result["alreadyEnded"], json!(["e-renewal"]));
    let status = |id: &str| result["enrollments"].as_array().expect("the set").iter().find(|item| item["id"] == id).expect(id)["status"].clone();
    assert_eq!(status("e-welcome"), json!("cancelled"));
    assert_eq!(status("e-bo"), json!("active"), "someone else's enrollment is not touched");

    let nobody = opt_out(&enrollments, "zed@example.org");
    assert!(!nobody.status.success(), "a contact no enrollment names is refused");
    assert!(String::from_utf8_lossy(&nobody.stderr).contains("no enrollment names contact zed@example.org"));
}
