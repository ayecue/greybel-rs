//! Generates world-independent network properties using `PlayerUtils`: Wi-Fi name, web domain, and
//! map position.

pub(crate) use greyhack_behaviour::web_domain;
use greyhack_behaviour::{ALPHANUMERIC, LOWERCASE_ALPHANUMERIC, ServiceId};

use crate::{
    Crime, Gender, Grade, Offense, Record, RecordDatabase, RecordKind, Rng, Subject, Word,
};

/// Generates 15–30 police or student records for `Router.CreaFichasPoliciales` and
/// `Router.CrearFichaEstudiantes`.
pub(crate) fn records(rng: &mut Rng, service: ServiceId, domain: &str) -> Option<RecordDatabase> {
    let kind = match service {
        ServiceId::Criminals => RecordKind::Police,
        ServiceId::Students => RecordKind::Student,
        _ => return None,
    };
    let count = rng.range(15, 30);
    let records = (0..count)
        .map(|_| {
            let surname = Word::Surname.generate(rng);
            let first_name = Gender::pick(rng).first_name(rng);
            let phone = rng.phone();
            let age = rng.range(18, 80) as i32;
            let person_id = rng.next_u64().to_string();
            let mut record = Record {
                phone,
                mail_address: format!("{surname}@{domain}"),
                full_name: format!("{first_name} {surname}"),
                age,
                person_id,
                kind,
                ..Record::default()
            };

            if kind == RecordKind::Police {
                record.offenses = Some(
                    (0..rng.range(1, 4) as i32)
                        .map(|id| Offense {
                            id,
                            crime: *rng.pick(Crime::ALL),
                            start_year: rng.range(1990, 1999) as i32,
                        })
                        .collect(),
                );
            } else {
                let mut subjects = Subject::ALL.to_vec();

                record.grades = Some(
                    (0..rng.range(1, 3))
                        .map(|_| Grade {
                            subject: rng.take(&mut subjects),
                            mark: rng.range(0, 1000) as f32 / 100.0,
                        })
                        .collect(),
                );
            }

            record
        })
        .collect();

    Some(RecordDatabase { records })
}

/// Generates an eight-character alphanumeric, domain-suffixed bank account ID
/// (`OS.GeneraBankAccount`).
pub(crate) fn bank_account(rng: &mut Rng, domain: &str) -> String {
    let id = rng.text(ALPHANUMERIC, 8);

    format!("{id}-{domain}")
}

/// Generates a company ESSID. Three of seven outcomes append an underscore and 3–5 uppercase
/// alphanumeric characters (`ServerMap.GeneraWifiName`).
pub(crate) fn wifi_name(rng: &mut Rng) -> String {
    let roll = rng.range(0, 6);
    let name = Word::Corporation.generate(rng);

    if roll < 3 {
        let length = rng.range(3, 5);

        format!(
            "{name}_{}",
            rng.text(LOWERCASE_ALPHANUMERIC, length).to_uppercase()
        )
    } else {
        name
    }
}
