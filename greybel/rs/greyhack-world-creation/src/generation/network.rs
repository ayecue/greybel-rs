//! Generates world-independent network properties using `PlayerUtils`: Wi-Fi name, web domain, and
//! map position.

pub(crate) use greyhack_behaviour::web_domain;
use greyhack_behaviour::{Gender, LOWERCASE_ALPHANUMERIC, Rng, ServiceId, Word, WorldSeed};
use greyhack_world_services::{Crime, Grade, Offense, Record, RecordDatabase, RecordKind, Subject};
use mono_random::Random;

/// Generates 15–30 police or student records for `Router.CreaFichasPoliciales` and
/// `Router.CrearFichaEstudiantes`.
#[expect(
    clippy::cast_precision_loss,
    reason = "Router.CrearFichaEstudiantes computes student marks with C# floats"
)]
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
            let age = i32::try_from(rng.range(18, 80)).expect("the age range fits i32");
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
                    (0..i32::try_from(rng.range(1, 4)).expect("the range fits i32"))
                        .map(|id| Offense {
                            id,
                            crime: *rng.pick(Crime::ALL),
                            start_year: i32::try_from(rng.range(1990, 1999))
                                .expect("the year range fits i32"),
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

/// Generates public Wi-Fi names from one world secret (`ServerMap.RouterNode`,
/// `NetworkLan.RouterNet.ConfigWifi`).
pub struct WifiGenerator<'a> {
    seed: &'a WorldSeed,
}

impl<'a> WifiGenerator<'a> {
    /// Creates a generator from the world secret.
    #[must_use]
    pub const fn new(seed: &'a WorldSeed) -> Self {
        Self { seed }
    }

    /// Returns the BSSID and ESSID for `addresses`: a main router's public address alone, or its
    /// public address followed by a LAN router's local address.
    #[must_use]
    pub fn names(&self, addresses: &[&str]) -> (String, String) {
        wifi(self.seed, addresses)
    }
}

/// Generates the BSSID and ESSID of public Wi-Fi from its world-seed context. The MAC's six byte
/// draws precede the name draws (`Networking.GeneraMacAddress`, `ServerMap.GeneraWifiName`).
pub(crate) fn wifi(seed: &WorldSeed, addresses: &[&str]) -> (String, String) {
    let mut random = Random::new(seed.derive_int("wifi-public-v1", addresses));
    let bssid = (0..6)
        .map(|_| format!("{:02X}", random.next_below(256)))
        .collect::<Vec<_>>()
        .join(":");
    let roll = random.next_below(7);
    let name = Word::Corporation.generate_with_index(|length| random.next_below(length));
    let essid = if roll < 3 {
        let length = random.next_below(3) + 3;
        let suffix = (0..length)
            .map(|_| {
                char::from(LOWERCASE_ALPHANUMERIC[random.next_below(LOWERCASE_ALPHANUMERIC.len())])
            })
            .collect::<String>()
            .to_uppercase();

        format!("{name}_{suffix}")
    } else {
        name
    };

    (bssid, essid)
}
