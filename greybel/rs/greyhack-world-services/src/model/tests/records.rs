use greyhack_behaviour::ServiceId;

use super::{Crime, Grade, Offense, Record, RecordDatabase, RecordKind, Subject};

#[path = "support/records.rs"]
pub(crate) mod support;
use support::DATABASE;

#[test]
fn should_read_and_write_a_records_database_unchanged() {
    let database = RecordDatabase::parse(DATABASE).unwrap();

    assert_eq!(database.records[0].kind, RecordKind::Police);
    assert_eq!(
        database.records[0].offenses,
        Some(vec![Offense {
            id: 0,
            crime: Crime::Hacking,
            start_year: 1991,
        }])
    );
    assert_eq!(database.records[0].face_parts[0].scale.x, 1.5);
    assert_eq!(
        database.records[1].grades,
        Some(vec![Grade {
            subject: Subject::Programming,
            mark: 4.5,
        }])
    );
    assert_eq!(database.to_json(), DATABASE);
}

#[test]
fn should_reject_a_database_with_a_record_of_no_kind() {
    let database = RecordDatabase::parse(r#"{"records":[{"person_id":"7"}]}"#);

    assert_eq!(database, None);
}

#[test]
fn should_read_an_update_as_the_service_record_kind() {
    let record = Record::parse_for(
        ServiceId::Criminals,
        r#"{"kind":1,"person_id":"8","grades":[{"subject":2,"mark":4.5}]}"#,
    )
    .unwrap();

    assert_eq!(record.kind, RecordKind::Student);
    assert_eq!(record.offenses, Some(Vec::new()));
    assert_eq!(record.grades, None);
}

#[test]
fn should_replace_only_the_record_of_the_same_person() {
    let mut database = RecordDatabase::parse(DATABASE).unwrap();
    let mut record = database.records[1].clone();

    record.age = 21;

    assert!(database.update(record.clone()));
    assert_eq!(database.records[1], record);

    record.person_id = "9".to_owned();
    assert!(!database.update(record));
}
