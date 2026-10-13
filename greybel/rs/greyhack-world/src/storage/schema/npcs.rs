//! NPCs, vacancies, and rental administrators.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "-- an NPC whose `dead_at` world second passed is dead (`NpcInfo.IsDead`); `job_role` is the game's `JobRole` number,
-- `work_performance` its `NpcInfo.WorkStatus` number, `company_align` its `NpcInfo.CompanyAlign` number and
-- `day_zero_contact` the mail address of its `NpcInfo.dayZeroContact`,
-- `hire_time` the world second of its hiring, `forced_login_until` the one its forced login lasts to; its schedule
-- (`NpcInfo.schedule`) in seconds from midnight: when its two work blocks start (the second beyond 24 hours on the
-- night shift) and when it walks to the appliance
CREATE TABLE IF NOT EXISTS npcs (
    person_id            INTEGER PRIMARY KEY REFERENCES persons(id) ON DELETE CASCADE,
    user_id              INTEGER NOT NULL UNIQUE REFERENCES computer_users(id) ON DELETE CASCADE,
    online               INTEGER NOT NULL,
    dead_at              INTEGER,
    job_role             INTEGER NOT NULL DEFAULT 0,
    license_plate        TEXT,
    fired                INTEGER NOT NULL DEFAULT 0,
    hire_time            INTEGER,
    forced_login_until   INTEGER,
    schedule_work1_start INTEGER NOT NULL,
    schedule_work2_start INTEGER NOT NULL,
    walk_to_appliance    INTEGER NOT NULL,
    wake_up_time         INTEGER,
    late_strikes         INTEGER NOT NULL DEFAULT 0,
    day_alarm            INTEGER NOT NULL DEFAULT 0,
    alarm_hour           INTEGER NOT NULL DEFAULT 0,
    alarm_minute         INTEGER NOT NULL DEFAULT 0,
    alarm_modified       INTEGER NOT NULL DEFAULT 0,
    alarm_deleted        INTEGER NOT NULL DEFAULT 0,
    work_performance     INTEGER NOT NULL DEFAULT 2,
    company_align        INTEGER NOT NULL DEFAULT 0,
    day_zero_contact     TEXT
);

-- an open job position of a network's company (`NpcHiring.availJobs`): the computer of the NPC who left it, by role,
-- filled in the order opened
CREATE TABLE IF NOT EXISTS vacancies (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    network_id  INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    job_role    INTEGER NOT NULL,
    computer_id INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE
);

-- the administrator a rented server's network hired (`NetworkLan.superAdminRental`), an NPC of another network
CREATE TABLE IF NOT EXISTS rental_admins (
    network_id INTEGER PRIMARY KEY REFERENCES networks(id) ON DELETE CASCADE,
    person_id  INTEGER NOT NULL REFERENCES persons(id) ON DELETE CASCADE
);

",
    tables: &[
    "npcs",
    "vacancies",
    "rental_admins",
    ],
};
