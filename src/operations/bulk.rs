//! Enrolling many contacts at once: every seed of a list started on the
//! same sequence the way `enroll` starts one, all of them or none. Two seeds
//! with the same enrollment id, or the same contact, are refused naming both,
//! so a list read twice cannot put one person on the sequence twice.

use std::collections::HashMap;

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};

use crate::sequences::enrollment::create_enrollment;
use crate::sequences::{Enrollment, EnrollmentSeed, Sequence};

/// Every seed enrolled in `sequence`, in the order given.
pub fn enroll_all(sequence: &Sequence, seeds: Vec<EnrollmentSeed>, at: Option<DateTime<Utc>>) -> Result<Vec<Enrollment>> {
    if seeds.is_empty() {
        bail!("the seeds file holds no enrollment; give a JSON array of at least one seed");
    }
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut contacts: HashMap<String, usize> = HashMap::new();
    let mut enrolled = Vec::new();
    for (index, seed) in seeds.into_iter().enumerate() {
        let enrollment = create_enrollment(sequence, seed, at)?;
        if let Some(first) = ids.insert(enrollment.id.clone(), index) {
            bail!("seeds[{first}] and seeds[{index}] are both enrollment {}; give each enrollment its own id", enrollment.id);
        }
        if let Some(first) = contacts.insert(enrollment.contact.id.clone(), index) {
            bail!(
                "seeds[{first}] and seeds[{index}] both enroll contact {}; a contact is enrolled in a sequence once",
                enrollment.contact.id
            );
        }
        enrolled.push(enrollment);
    }
    Ok(enrolled)
}

/// The cells of `text`, row by row, as RFC 4180 reads them.
fn rows(text: &str) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match (quoted, character) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            (true, '"') => quoted = false,
            (true, other) => cell.push(other),
            (false, '"') if cell.is_empty() => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut cell)),
            (false, '\r') if chars.peek() == Some(&'\n') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            (false, other) => cell.push(other),
        }
    }
    if quoted {
        bail!("the sheet ends inside a quoted cell; a closing quote is missing");
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    Ok(rows)
}

/// Seeds from a CSV sheet, one per row: the header names each column by the
/// seed field it fills, nested ones joined with a dot (`id`, `contact.id`,
/// `contact.email`, `contact.variables.name`, `contact.variables.company.name`,
/// `enrolledAt`), so a contact list exported from a spreadsheet or a CRM
/// becomes the seeds `enroll-all` takes. An empty cell is a field the row does
/// not set; every value is text. A malformed row is refused naming its line.
pub fn seeds_from_sheet(text: &str) -> Result<Vec<EnrollmentSeed>> {
    let mut all = rows(text)?.into_iter();
    let Some(header) = all.next() else {
        bail!("the sheet is empty; its first row must name the columns, such as id, contact.id, contact.email");
    };
    let mut seeds = Vec::new();
    for (index, cells) in all.enumerate() {
        // Lines count from one, and the header is the first.
        let one = std::slice::from_ref(&header).len();
        let line = index + one + one;
        if cells.len() != header.len() {
            bail!("row {line} has {} cells under a header of {}", cells.len(), header.len());
        }
        let mut record = serde_json::Map::new();
        for (column, cell) in header.iter().zip(&cells).filter(|(_, cell)| !cell.trim().is_empty()) {
            let mut parts: Vec<&str> = column.trim().split('.').collect();
            let Some(last) = parts.pop() else {
                bail!("row {line}: an empty column name");
            };
            let mut current = &mut record;
            for part in parts {
                let entry = current.entry(part.to_owned()).or_insert_with(|| serde_json::json!({}));
                let serde_json::Value::Object(next) = entry else {
                    bail!("row {line}: column {column} nests under {part}, which another column set to a value");
                };
                current = next;
            }
            current.insert(last.to_owned(), serde_json::Value::String(cell.trim().to_owned()));
        }
        let seed: EnrollmentSeed = serde_json::from_value(serde_json::Value::Object(record))
            .map_err(|error| anyhow::anyhow!("row {line} is not a seed: {error}"))?;
        seeds.push(seed);
    }
    Ok(seeds)
}

/// One event for one enrollment, as a mailer's send log or a reply feed lists them.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentEventEntry {
    pub enrollment_id: String,
    pub event: crate::sequences::EnrollmentEvent,
}

/// `enrollments` with every entry's event recorded on its enrollment as
/// `record` records one, in the order given, all or none: an entry naming an
/// enrollment the set does not hold, or an event `record` refuses, is refused
/// naming the entry, and nothing is recorded. The whole set comes back in
/// its order, to write back.
pub fn record_all(enrollments: Vec<Enrollment>, entries: Vec<EnrollmentEventEntry>) -> Result<Vec<Enrollment>> {
    if entries.is_empty() {
        bail!("the events file holds no entry; give a JSON array of {{enrollmentId, event}}");
    }
    let mut positions: HashMap<String, usize> = HashMap::new();
    for (index, enrollment) in enrollments.iter().enumerate() {
        if let Some(first) = positions.insert(enrollment.id.clone(), index) {
            bail!("enrollments[{first}] and enrollments[{index}] are both {}; give each enrollment its own id", enrollment.id);
        }
    }
    let mut slots: Vec<Option<Enrollment>> = enrollments.into_iter().map(Some).collect();
    for (index, entry) in entries.into_iter().enumerate() {
        let Some(&position) = positions.get(&entry.enrollment_id) else {
            bail!("events[{index}] names enrollment {}, which the set does not hold; nothing was recorded", entry.enrollment_id);
        };
        let Some(current) = slots[position].take() else {
            bail!("enrollment {} was lost while events[{index}] was recorded", entry.enrollment_id);
        };
        let recorded = crate::sequences::enrollment::record_event(current, entry.event)
            .map_err(|error| anyhow::anyhow!("events[{index}] for enrollment {}: {error:#}; nothing was recorded", entry.enrollment_id))?;
        slots[position] = Some(recorded);
    }
    slots
        .into_iter()
        .enumerate()
        .map(|(index, slot)| match slot {
            Some(enrollment) => Ok(enrollment),
            None => Err(anyhow::anyhow!("enrollments[{index}] was lost while the events were recorded")),
        })
        .collect()
}
