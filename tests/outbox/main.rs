//! `email-sequencing outbox` through the built binary: the message due for
//! a contact holding every variable is ready, its subject and body filled;
//! one for a contact without an address or a variable is written out but
//! not ready, and says what it lacks; a paused enrollment has nothing due.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn the_messages_due_are_written_out_for_their_contacts() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("outbox");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let sequence = dir.join("sequence.json");
    std::fs::write(
        &sequence,
        json!({"id": "welcome", "steps": [{"id": "intro", "subject": "Hi {{name}}", "template": "Welcome to {{company.name}}, {{name}}."}]})
            .to_string(),
    )
    .expect("the sequence");
    let enrolled = (chrono::Utc::now() - chrono::TimeDelta::days(std::slice::from_ref(&dir).len() as i64)).to_rfc3339();
    let enrollment = |id: &str, email: Option<&str>, variables: Value, status: &str| {
        json!({"id": id, "sequenceId": "welcome", "contact": {"id": id, "email": email, "variables": variables}, "enrolledAt": enrolled, "status": status})
    };
    let enrollments = dir.join("enrollments.json");
    std::fs::write(
        &enrollments,
        json!([
            enrollment("ada", Some("ada@falcon.example"), json!({"name": "Ada", "company": {"name": "Falcon"}}), "active"),
            enrollment("bo", None, json!({"name": "Bo"}), "active"),
            enrollment("cy", Some("cy@falcon.example"), json!({"name": "Cy", "company": {"name": "Falcon"}}), "paused")
        ])
        .to_string(),
    )
    .expect("the enrollments");

    let answer = Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["outbox", "--sequence"])
        .arg(&sequence)
        .arg("--enrollments")
        .arg(&enrollments)
        .output()
        .expect("the binary runs");
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let messages: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let messages = messages.as_array().expect("the messages");
    let ids: Vec<&str> = messages.iter().map(|message| message["enrollmentId"].as_str().expect("an id")).collect();
    assert!(ids.contains(&"ada") && ids.contains(&"bo") && !ids.contains(&"cy"), "the paused one has nothing due: {ids:?}");

    let ada = messages.iter().find(|message| message["enrollmentId"] == "ada").expect("Ada's message");
    assert_eq!(ada["to"], "ada@falcon.example");
    assert_eq!(ada["subject"], "Hi Ada");
    assert_eq!(ada["body"], "Welcome to Falcon, Ada.");
    assert_eq!(ada["ready"], json!(true));

    let bo = messages.iter().find(|message| message["enrollmentId"] == "bo").expect("Bo's message");
    assert_eq!(bo["to"], Value::Null, "Bo holds no address");
    assert_eq!(bo["missing"], json!(["company.name"]), "nor a company");
    assert_eq!(bo["ready"], json!(false));
}

#[test]
fn a_preview_writes_out_every_step_of_one_enrollment() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("preview");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let sequence = dir.join("sequence.json");
    std::fs::write(
        &sequence,
        json!({"id": "welcome", "steps": [
            {"id": "intro", "subject": "Hi {{name}}", "template": "Welcome, {{name}}."},
            {"id": "nudge", "afterHours": std::slice::from_ref(&dir).len(), "template": "Still there, {{name}}? Ask {{owner}}."}
        ]})
        .to_string(),
    )
    .expect("the sequence");
    let enrollment = dir.join("enrollment.json");
    std::fs::write(
        &enrollment,
        json!({"id": "ada", "sequenceId": "welcome", "enrolledAt": chrono::Utc::now().to_rfc3339(),
               "contact": {"id": "ada", "email": "ada@falcon.example", "variables": {"name": "Ada"}},
               "events": [{"type": "sent", "stepId": "intro", "at": chrono::Utc::now().to_rfc3339()}]})
        .to_string(),
    )
    .expect("the enrollment");

    let answer = Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["preview", "--sequence"])
        .arg(&sequence)
        .arg("--enrollment")
        .arg(&enrollment)
        .output()
        .expect("the binary runs");
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let messages: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let steps: Vec<&str> = messages.as_array().expect("the steps").iter().map(|step| step["stepId"].as_str().expect("a step")).collect();
    assert_eq!(steps, vec!["intro", "nudge"], "every step, in the sequence's order");
    let intro = messages.as_array().and_then(|all| all.first()).expect("the first step");
    assert_eq!(intro["state"], "sent");
    assert_eq!(intro["subject"], "Hi Ada");
    let nudge = messages.as_array().and_then(|all| all.last()).expect("the last step");
    assert_eq!(nudge["state"], "pending", "not due yet, written out all the same");
    assert_eq!(nudge["body"], "Still there, Ada? Ask .");
    assert_eq!(nudge["missing"], json!(["owner"]));
    assert_eq!(nudge["ready"], json!(false));
}

#[test]
fn a_check_names_who_each_step_cannot_reach_yet() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("check");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let sequence = dir.join("sequence.json");
    std::fs::write(
        &sequence,
        json!({"id": "welcome", "steps": [
            {"id": "intro", "template": "Hi {{name}}."},
            {"id": "offer", "template": "{{name}}, {{company.name}} gets a discount."}
        ]})
        .to_string(),
    )
    .expect("the sequence");
    let now = chrono::Utc::now().to_rfc3339();
    let enrollments = dir.join("enrollments.json");
    std::fs::write(
        &enrollments,
        json!([
            {"id": "ada", "sequenceId": "welcome", "enrolledAt": now, "contact": {"id": "ada", "email": "ada@falcon.example", "variables": {"name": "Ada", "company": {"name": "Falcon"}}}},
            {"id": "bo", "sequenceId": "welcome", "enrolledAt": now, "contact": {"id": "bo", "variables": {"name": "Bo"}}}
        ])
        .to_string(),
    )
    .expect("the enrollments");

    let answer = Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["check", "--sequence"])
        .arg(&sequence)
        .arg("--enrollments")
        .arg(&enrollments)
        .output()
        .expect("the binary runs");
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let steps: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    assert_eq!(
        steps,
        json!([
            {"stepId": "intro", "withoutAddress": ["bo"], "missing": {}},
            {"stepId": "offer", "withoutAddress": ["bo"], "missing": {"company.name": ["bo"]}}
        ])
    );
}

#[test]
fn the_stopped_contacts_are_those_a_reply_bounce_or_unsubscribe_ended() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stopped");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let now = chrono::Utc::now();
    let earlier = (now - chrono::TimeDelta::hours(std::slice::from_ref(&dir).len() as i64)).to_rfc3339();
    let enrolled = |id: &str, events: Value| {
        json!({"id": id, "sequenceId": "welcome", "enrolledAt": earlier, "contact": {"id": id, "email": format!("{id}@falcon.example")}, "events": events})
    };
    let enrollments = dir.join("enrollments.json");
    std::fs::write(
        &enrollments,
        json!([
            enrolled("ada", json!([{"type": "unsubscribed", "at": now.to_rfc3339()}])),
            enrolled("bo", json!([{"type": "bounced", "at": earlier}])),
            enrolled("cy", json!([{"type": "sent", "stepId": "intro", "at": earlier}]))
        ])
        .to_string(),
    )
    .expect("the enrollments");

    let answer = Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["stopped", "--enrollments"])
        .arg(&enrollments)
        .output()
        .expect("the binary runs");
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let stopped: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let ids: Vec<&str> = stopped.as_array().expect("the list").iter().map(|entry| entry["contactId"].as_str().expect("an id")).collect();
    assert_eq!(ids, vec!["bo", "ada"], "the earliest stop first; a contact only sent to is not stopped");
    assert_eq!(stopped[std::slice::from_ref(&dir).len()]["event"]["type"], "unsubscribed");
}
