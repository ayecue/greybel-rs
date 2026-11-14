//! JSON records stored by police, university, and company SQL services. Models `ContentFichas`,
//! `Ficha`, `FichaPolicial`, and `FichaEstudiantes` with English field names.

use greyhack_behaviour::{ServiceId, enum_def};
use serde::{Deserialize, Serialize};

enum_def! {
    /// Record type (`Ficha.TypeFichas`).
    #[derive(Default, Serialize, Deserialize)]
    #[serde(into = "i64", try_from = "i64")]
    pub enum RecordKind {
        #[default]
        Unknown = 0,
        Student = 1,
        Police = 2,
        Employee = 3,
    }
}

enum_def! {
    /// Police-record offense (`FichaPolicial.Crimen`).
    #[derive(Default, Serialize, Deserialize)]
    #[serde(into = "i64", try_from = "i64")]
    pub enum Crime {
        #[default]
        Robbery = 0,
        Murder = 1,
        Fraud = 2,
        Exhibitionism = 3,
        Drugs = 4,
        Tax = 5,
        Aggression = 6,
        Kidnapping = 7,
        Hacking = 8,
    }
}

enum_def! {
    /// Student-record subject (`FichaEstudiantes.TipoAsignatura`).
    #[derive(Default, Serialize, Deserialize)]
    #[serde(into = "i64", try_from = "i64")]
    pub enum Subject {
        #[default]
        Mathematics = 0,
        Physics = 1,
        Programming = 2,
        Economy = 3,
    }
}

/// Records database (`ContentFichas`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordDatabase {
    pub records: Vec<Record>,
}

impl RecordDatabase {
    /// Parses a database using `FichaJsonConverter` rules. Record `kind` selects populated lists;
    /// unknown kinds and malformed content return `None`.
    #[must_use]
    pub fn parse(content: &str) -> Option<Self> {
        let mut database: Self = serde_json::from_str(content).ok()?;

        for record in &mut database.records {
            if record.kind == RecordKind::Unknown {
                return None;
            }

            record.conform(record.kind);
        }

        Some(database)
    }

    /// Serialized database contents.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("records serialize")
    }

    /// Replaces the record for the same person (`ContentFichas.UpdateFicha`). Returns false if its
    /// ID is absent.
    pub fn update(&mut self, record: Record) -> bool {
        let Some(current) = self
            .records
            .iter_mut()
            .find(|current| current.person_id == record.person_id)
        else {
            return false;
        };

        *current = record;

        true
    }
}

/// Record (`Ficha` and subclasses). Police records use `offenses`, student records use `grades`,
/// and other kinds use neither.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub kind: RecordKind,
    pub person_id: String,
    pub full_name: String,
    pub age: i32,
    pub phone: String,
    pub mail_address: String,
    pub face_parts: Vec<FacePart>,
    pub schedule: String,
    pub license_plate: String,
    pub local_ip: String,
    /// Job status from `NpcInfo.WorkStatus`.
    pub work_performance: i32,
    /// Job role (`JobRole`).
    pub job_role: i32,
    pub late_work_strikes: i32,
    pub dead: bool,
    pub fired: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offenses: Option<Vec<Offense>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grades: Option<Vec<Grade>>,
}

impl Record {
    /// Parses a record submitted to `service` using `FileHelperServer.UpdateFichaServerRpc` rules.
    /// The service selects populated lists regardless of record `kind`. Malformed content returns
    /// `None`.
    #[must_use]
    pub fn parse_for(service: ServiceId, content: &str) -> Option<Self> {
        let mut record: Self = serde_json::from_str(content).ok()?;

        record.conform(match service {
            ServiceId::Criminals => RecordKind::Police,
            ServiceId::Students => RecordKind::Student,
            _ => RecordKind::Employee,
        });

        Some(record)
    }

    /// Stores only lists associated with `kind`, matching deserialization into that record class.
    /// Missing lists become empty and other lists are omitted.
    fn conform(&mut self, kind: RecordKind) {
        let offenses = self.offenses.take();
        let grades = self.grades.take();

        match kind {
            RecordKind::Police => self.offenses = Some(offenses.unwrap_or_default()),
            RecordKind::Student => self.grades = Some(grades.unwrap_or_default()),
            RecordKind::Employee | RecordKind::Unknown => {}
        }
    }
}

/// Police-record offense (`FichaPolicial.Delito`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Offense {
    pub id: i32,
    pub crime: Crime,
    pub start_year: i32,
}

/// Student grade for one subject (`FichaEstudiantes.Asignatura`); `mark` passes at 5.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Grade {
    pub subject: Subject,
    pub mark: f32,
}

/// One component of a record face image (`InfoFace.Face`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FacePart {
    pub name: String,
    pub scale: Vector,
    pub position: Vector,
}

impl Default for FacePart {
    fn default() -> Self {
        Self {
            name: String::new(),
            scale: Vector { x: 1.0, y: 1.0 },
            position: Vector::default(),
        }
    }
}

/// Two-dimensional vector (Unity `Vector2`).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
#[path = "tests/records.rs"]
pub(crate) mod tests;
