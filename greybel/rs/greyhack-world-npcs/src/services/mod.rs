//! NPC people, employment, schedules, alarms, and mail services.

pub mod alarm;
mod events;
pub mod jobs;
mod login_capture;
pub mod mail;
mod network_creation;
pub mod people;
pub mod person;
pub mod rental_admin;
pub mod schedule;
mod smtp_access;

pub use alarm::{AlarmFailure, PhoneAlarm};
pub use events::{AdminAbsence, AdminArrival, NpcJobChange};
pub use jobs::{FireFailure, HireFailure, JobService};
pub use login_capture::NpcLoginCapture;
pub use mail::{Mail, MailRefusal, SendMailFailure};
pub use network_creation::{NetworkCreation, NetworkRestore};
pub use person::NpcService;
pub(crate) use person::NpcServiceSupport;
pub use rental_admin::RentalAdminFailure;
pub use schedule::ScheduleService;
pub use smtp_access::SmtpAccess;
