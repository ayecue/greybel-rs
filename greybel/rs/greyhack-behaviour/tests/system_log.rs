use greyhack_behaviour::{LogAction, MAX_LOG_ENTRIES, SystemLog, previous_address};

use crate::support::system_log::entry;

#[test]
fn should_serialize_the_game_format() {
    // Arrange
    let mut log = SystemLog::default();

    log.add(entry(LogAction::Shell, "1.2.3.4"));

    // Act
    let json = log.to_json();

    // Assert
    assert_eq!(
        json,
        r#"{"contentLog":[{"action":2,"ip":"1.2.3.4","fecha":"01/Dec/2000 - 06:00","puerto":22,"file":"","bounceIp":"","playerNetID":"abc","tutorial":false,"tokenTrace":"token"}]}"#
    );
}

#[test]
fn should_read_back_what_it_serialized() {
    // Arrange
    let mut log = SystemLog::default();

    log.add(entry(LogAction::Open, "1.2.3.4"));
    log.add(entry(LogAction::Remove, "5.6.7.8"));

    // Act
    let parsed = SystemLog::parse(&log.to_json());

    // Assert
    assert_eq!(parsed, Some(log));
}

#[test]
fn should_read_an_empty_content_as_an_empty_log() {
    assert_eq!(SystemLog::parse(""), Some(SystemLog::default()));
    assert_eq!(SystemLog::parse("not a log"), None);
}

#[test]
fn should_keep_the_newest_entries() {
    // Arrange
    let mut log = SystemLog::default();

    // Act
    for index in 0..MAX_LOG_ENTRIES + 3 {
        log.add(entry(LogAction::Open, &index.to_string()));
    }

    // Assert
    assert_eq!(log.content_log.len(), MAX_LOG_ENTRIES);
    assert_eq!(log.content_log[0].ip, "3");
    assert_eq!(
        log.content_log.last().unwrap().ip,
        (MAX_LOG_ENTRIES + 2).to_string()
    );
}

#[test]
fn should_name_the_address_a_shell_came_from() {
    // A connection to another address names the computer it came from; one to that address names where that one did.
    assert_eq!(previous_address("1.1.1.1", "2.2.2.2", "9.9.9.9"), "1.1.1.1");
    assert_eq!(previous_address("1.1.1.1", "1.1.1.1", "9.9.9.9"), "9.9.9.9");
}
