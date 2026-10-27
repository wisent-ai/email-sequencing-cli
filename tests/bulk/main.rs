//! `email-sequencing enroll-all` through the built binary: a list of seeds
//! enrolled at one moment, a list naming one contact twice refused with
//! both seeds named, and a contact sheet (`.csv`) enrolled with its nested
//! variables, a ragged row refused by its line.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::{Command, Output};

fn enroll_all(sequence: &PathBuf, seeds: &PathBuf, at: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
        .args(["enroll-all", "--sequence"])
        .arg(sequence)
        .arg("--seeds")
        .arg(seeds)
        .args(["--at", at])
        .output()
        .expect("the binary runs")
}

#[test]
fn a_list_of_contacts_is_enrolled_at_once() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("enroll-all");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let sequence = dir.join("sequence.json");
    std::fs::write(&sequence, json!({"id": "welcome", "steps": [{"id": "intro", "template": "Hi"}]}).to_string()).expect("the sequence");
    let seeds = dir.join("seeds.json");
    std::fs::write(
        &seeds,
        json!([{"id": "e-ada", "contact": {"id": "ada"}}, {"id": "e-bo", "contact": {"id": "bo"}}]).to_string(),
    )
    .expect("the seeds");
    let at = chrono::Utc::now().to_rfc3339();

    let answer = enroll_all(&sequence, &seeds, &at);
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let enrolled: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let ids: Vec<&str> = enrolled.as_array().expect("the enrollments").iter().map(|item| item["id"].as_str().expect("an id")).collect();
    assert_eq!(ids, vec!["e-ada", "e-bo"]);
    let one = std::slice::from_ref(&enrolled).len();
    assert_eq!(enrolled[one - one]["sequenceId"], "welcome");
    assert_eq!(enrolled[one - one]["status"], "active");

    let twice = dir.join("twice.json");
    std::fs::write(
        &twice,
        json!([{"id": "e-ada", "contact": {"id": "ada"}}, {"id": "e-ada-again", "contact": {"id": "ada"}}]).to_string(),
    )
    .expect("the seeds");
    let refused = enroll_all(&sequence, &twice, &at);
    assert!(!refused.status.success(), "one contact enrolled twice is refused");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("both enroll contact ada"));
}

#[test]
fn a_contact_sheet_is_enrolled_with_its_variables() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("enroll-all-sheet");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let sequence = dir.join("sequence.json");
    std::fs::write(&sequence, json!({"id": "welcome", "steps": [{"id": "intro", "template": "Hi {{name}}"}]}).to_string())
        .expect("the sequence");
    let sheet = dir.join("contacts.csv");
    std::fs::write(
        &sheet,
        "id,contact.id,contact.email,contact.variables.name,contact.variables.company.name\r\n\
         e-ada,ada,ada@example.org,Ada,\"Analytical, Ltd\"\r\n\
         e-bo,bo,,Bo,\r\n",
    )
    .expect("the sheet");
    let at = chrono::Utc::now().to_rfc3339();

    let answer = enroll_all(&sequence, &sheet, &at);
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let enrolled: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let ada = &enrolled.as_array().expect("the enrollments").iter().find(|item| item["id"] == "e-ada").expect("e-ada")["contact"];
    assert_eq!(ada["email"], "ada@example.org");
    assert_eq!(ada["variables"]["company"]["name"], "Analytical, Ltd", "a quoted comma stays in its cell");
    let bo = &enrolled.as_array().expect("the enrollments").iter().find(|item| item["id"] == "e-bo").expect("e-bo")["contact"];
    assert!(bo["email"].is_null(), "an empty cell leaves the field unset: {bo}");

    let ragged = dir.join("ragged.csv");
    std::fs::write(&ragged, "id,contact.id\r\ne-ada,ada,extra\r\n").expect("the sheet");
    let refused = enroll_all(&sequence, &ragged, &at);
    assert!(!refused.status.success(), "a row longer than the header is refused");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("row 2"), "{}", String::from_utf8_lossy(&refused.stderr));
}

#[test]
fn a_send_log_is_recorded_across_enrollments_all_or_none() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("record-all");
    std::fs::create_dir_all(&dir).expect("the test's directory");
    let sequence = dir.join("sequence.json");
    std::fs::write(&sequence, json!({"id": "welcome", "steps": [{"id": "intro", "template": "Hi"}]}).to_string()).expect("the sequence");
    let seeds = dir.join("seeds.json");
    std::fs::write(&seeds, json!([{"id": "e-ada", "contact": {"id": "ada"}}, {"id": "e-bo", "contact": {"id": "bo"}}]).to_string())
        .expect("the seeds");
    let at = chrono::Utc::now().to_rfc3339();
    let enrolled = enroll_all(&sequence, &seeds, &at);
    assert!(enrolled.status.success(), "{}", String::from_utf8_lossy(&enrolled.stderr));
    let enrollments = dir.join("enrollments.json");
    std::fs::write(&enrollments, &enrolled.stdout).expect("the enrollments");

    let record_all = |events: Value| {
        let file = dir.join("events.json");
        std::fs::write(&file, events.to_string()).expect("the events");
        Command::new(env!("CARGO_BIN_EXE_email-sequencing-cli"))
            .args(["record-all", "--enrollments"])
            .arg(&enrollments)
            .arg("--events")
            .arg(&file)
            .output()
            .expect("the binary runs")
    };
    let sent = |id: &str| json!({"enrollmentId": id, "event": {"type": "sent", "at": at, "stepId": "intro"}});

    let answer = record_all(json!([sent("e-bo")]));
    assert!(answer.status.success(), "{}", String::from_utf8_lossy(&answer.stderr));
    let set: Value = serde_json::from_slice(&answer.stdout).expect("JSON on stdout");
    let events = |id: &str| set.as_array().expect("the set").iter().find(|item| item["id"] == id).expect(id)["events"].as_array().expect("events").len();
    assert!(events("e-bo") > events("e-ada"), "the send is on e-bo and not on e-ada: {set}");

    let refused = record_all(json!([sent("e-ada"), sent("e-zed")]));
    assert!(!refused.status.success(), "an entry for an enrollment the set does not hold is refused");
    assert!(String::from_utf8_lossy(&refused.stderr).contains("events[1] names enrollment e-zed"), "{}", String::from_utf8_lossy(&refused.stderr));
    assert!(refused.stdout.is_empty(), "nothing is printed when nothing is recorded");
}
