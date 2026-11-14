//! NPC people, employment, schedules, alarms, and mail services.

pub mod alarm;
mod events;
mod hooks;
pub mod jobs;
pub mod mail;
pub mod people;
pub mod person;
pub mod rental_admin;
pub mod schedule;

pub use alarm::{AlarmFailure, PhoneAlarm};
pub use events::{AdminAbsence, AdminArrival, NpcJobChange};
pub use hooks::{NetworkCreation, NetworkRestore, NpcLoginCapture, SmtpAccess};
pub use jobs::{FireFailure, HireFailure, JobService};
pub use mail::{Mail, MailRefusal, SendMailFailure};
pub use person::NpcService;
pub(crate) use person::NpcServiceSupport;
pub use rental_admin::RentalAdminFailure;
pub use schedule::Schedules;
