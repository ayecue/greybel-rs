//! NPC, job, open-position, and rented-server administrator storage.

use greyhack_world_core::Database;
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all, fetch_opt};
use turso::params;

use crate::{
    AlarmConfig, CompanyAlign, ComputerId, JobRole, NetworkId, Npc, NpcSchedule, PersonId, Result,
    WorkPerformance,
};

impl FromRow for Npc {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            person: PersonId(row.get("person_id")?),
            computer: ComputerId(row.get("computer_id")?),
            user: row.get("user")?,
            name: row.get("name")?,
            online: row.get("online")?,
            dead_at: row.get("dead_at")?,
            job_role: {
                let code: u8 = row.get("job_role")?;

                JobRole::from_code(code).ok_or_else(|| {
                    turso::Error::ConversionFailure(format!("{code} is no job role"))
                })?
            },
            license_plate: row.get("license_plate")?,
            fired: row.get("fired")?,
            hire_time: row.get("hire_time")?,
            forced_login_until: row.get("forced_login_until")?,
            schedule: NpcSchedule::from_schedule(
                row.get("schedule_work1_start")?,
                row.get("schedule_work2_start")?,
                row.get("walk_to_appliance")?,
            ),
            wake_up_time: row.get("wake_up_time")?,
            late_strikes: row.get("late_strikes")?,
            day_alarm: row.get("day_alarm")?,
            alarm: AlarmConfig {
                hour: row.get("alarm_hour")?,
                minute: row.get("alarm_minute")?,
                is_modified: row.get("alarm_modified")?,
                is_deleted: row.get("alarm_deleted")?,
            },
            work_performance: {
                let code: u8 = row.get("work_performance")?;

                WorkPerformance::from_code(code).ok_or_else(|| {
                    turso::Error::ConversionFailure(format!("{code} is no work performance"))
                })?
            },
            company_align: {
                let code: u8 = row.get("company_align")?;

                CompanyAlign::from_code(code).ok_or_else(|| {
                    turso::Error::ConversionFailure(format!("{code} is no company alignment"))
                })?
            },
            day_zero_contact: row.get("day_zero_contact")?,
        })
    }
}

/// Open job position and its computer.
struct Vacancy {
    id: i64,
    computer: ComputerId,
}

impl FromRow for Vacancy {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            computer: ComputerId(row.get("computer_id")?),
        })
    }
}

/// Queries of [`Database`] for npcs.
/// Queries of the tables owned by this area.
pub(crate) struct NpcRepository {
    db: Database,
}

impl NpcRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn npc(&self, person: PersonId) -> Result<Option<Npc>> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT npcs.person_id, computer_users.computer_id, computer_users.name AS user, persons.name, npcs.online, npcs.dead_at, \
             npcs.job_role, npcs.license_plate, npcs.fired, npcs.hire_time, npcs.forced_login_until, npcs.schedule_work1_start, npcs.schedule_work2_start, \
             npcs.walk_to_appliance, npcs.wake_up_time, npcs.late_strikes, npcs.day_alarm, \
             npcs.alarm_hour, npcs.alarm_minute, npcs.alarm_modified, npcs.alarm_deleted, \
             npcs.work_performance, npcs.company_align, npcs.day_zero_contact \
             FROM npcs JOIN computer_users ON computer_users.id = npcs.user_id \
             JOIN persons ON persons.id = npcs.person_id WHERE npcs.person_id = ?1",
            params![person.0],
        )
        .await?)
    }

    pub(crate) async fn npcs_on(&self, computer: ComputerId) -> Result<Vec<Npc>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT npcs.person_id, computer_users.computer_id, computer_users.name AS user, persons.name, npcs.online, npcs.dead_at, \
             npcs.job_role, npcs.license_plate, npcs.fired, npcs.hire_time, npcs.forced_login_until, npcs.schedule_work1_start, npcs.schedule_work2_start, \
             npcs.walk_to_appliance, npcs.wake_up_time, npcs.late_strikes, npcs.day_alarm, \
             npcs.alarm_hour, npcs.alarm_minute, npcs.alarm_modified, npcs.alarm_deleted, \
             npcs.work_performance, npcs.company_align, npcs.day_zero_contact \
             FROM npcs JOIN computer_users ON computer_users.id = npcs.user_id \
             JOIN persons ON persons.id = npcs.person_id WHERE computer_users.computer_id = ?1 ORDER BY npcs.person_id",
            params![computer.0],
        )
        .await?)
    }

    pub(crate) async fn insert_npc(
        &self,
        person: PersonId,
        computer: ComputerId,
        user: &str,
        online: bool,
        schedule: &NpcSchedule,
        work_performance: WorkPerformance,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO npcs (person_id, user_id, online, schedule_work1_start, schedule_work2_start, walk_to_appliance, \
             work_performance) \
             SELECT ?1, id, ?4, ?5, ?6, ?7, ?8 FROM computer_users WHERE computer_id = ?2 AND name = ?3",
            params![
                person.0,
                computer.0,
                user,
                online,
                schedule.work1_start().cast_signed(),
                schedule.work2_start().cast_signed(),
                schedule.walk_to_appliance().cast_signed(),
                work_performance as u8,
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn set_npc_online(&self, npc: PersonId, online: bool) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE npcs SET online = ?2 WHERE person_id = ?1",
            params![npc.0, online],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn save_npc_job(&self, npc: &Npc) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE npcs SET dead_at = ?2, job_role = ?3, license_plate = ?4, fired = ?5, hire_time = ?6, \
             schedule_work1_start = ?7, schedule_work2_start = ?8, walk_to_appliance = ?9, forced_login_until = ?10, \
             wake_up_time = ?11, late_strikes = ?12, day_alarm = ?13, alarm_hour = ?14, alarm_minute = ?15, \
             alarm_modified = ?16, alarm_deleted = ?17, work_performance = ?18, company_align = ?19, \
             day_zero_contact = ?20 WHERE person_id = ?1",
            params![
                npc.person.0,
                npc.dead_at.map(u64::cast_signed),
                npc.job_role.code(),
                npc.license_plate.as_deref(),
                npc.fired,
                npc.hire_time.map(u64::cast_signed),
                npc.schedule.work1_start().cast_signed(),
                npc.schedule.work2_start().cast_signed(),
                npc.schedule.walk_to_appliance().cast_signed(),
                npc.forced_login_until.map(u64::cast_signed),
                npc.wake_up_time.map(u64::cast_signed),
                npc.late_strikes,
                npc.day_alarm,
                npc.alarm.hour,
                npc.alarm.minute,
                npc.alarm.is_modified,
                npc.alarm.is_deleted,
                npc.work_performance as u8,
                npc.company_align.code(),
                npc.day_zero_contact.as_deref(),
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn npcs_in(&self, network: NetworkId) -> Result<Vec<Npc>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT npcs.person_id, computer_users.computer_id, computer_users.name AS user, persons.name, npcs.online, \
             npcs.dead_at, npcs.job_role, npcs.license_plate, npcs.fired, npcs.hire_time, npcs.forced_login_until, npcs.schedule_work1_start, npcs.schedule_work2_start, \
             npcs.walk_to_appliance, npcs.wake_up_time, npcs.late_strikes, npcs.day_alarm, \
             npcs.alarm_hour, npcs.alarm_minute, npcs.alarm_modified, npcs.alarm_deleted, \
             npcs.work_performance, npcs.company_align, npcs.day_zero_contact \
             FROM npcs JOIN computer_users ON computer_users.id = npcs.user_id \
             JOIN persons ON persons.id = npcs.person_id \
             \
             JOIN computers ON computers.id = computer_users.computer_id WHERE computers.network_id = ?1 \
             ORDER BY npcs.person_id",
            params![network.0],
        )
        .await?)
    }

    pub(crate) async fn open_job(
        &self,
        network: NetworkId,
        role: JobRole,
        computer: ComputerId,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO vacancies (network_id, job_role, computer_id) VALUES (?1, ?2, ?3)",
            params![network.0, role.code(), computer.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn close_job(
        &self,
        network: NetworkId,
        role: JobRole,
    ) -> Result<Option<ComputerId>> {
        let Some(vacancy) = fetch_opt::<Vacancy>(
            &self.db.connection(),
            "SELECT id, computer_id FROM vacancies WHERE network_id = ?1 AND job_role = ?2 ORDER BY id LIMIT 1",
            params![network.0, role.code()],
        )
        .await?
        else {
            return Ok(None);
        };

        execute(
            &self.db.connection(),
            "DELETE FROM vacancies WHERE id = ?1",
            params![vacancy.id],
        )
        .await?;

        Ok(Some(vacancy.computer))
    }
}
