use std::rc::Rc;

use crate::binary::BinaryWriter;
use crate::channel::VersionTable;
use crate::definition::{ErrorCode, Role};
use crate::migration::MigrationStep;
use crate::protocol::MessageSegment;

#[derive(Default)]
pub(crate) struct MigrationTable(Vec<Rc<dyn MigrationStep>>);

impl MigrationTable {
    pub(crate) fn add(&mut self, migration: Rc<dyn MigrationStep>) {
        self.0
            .retain(|stored_step| stored_step.from().major != migration.from().major);
        self.0.push(migration);
        self.0.sort_by_key(|migration| migration.from().major);
    }

    pub(crate) fn path(&self, major: u16, native: u16) -> Option<MigrationPath> {
        let links: Vec<Rc<dyn MigrationStep>> = (major..native)
            .map(|from| {
                self.0
                    .iter()
                    .find(|migration| migration.from().major == from)
                    .cloned()
            })
            .collect::<Option<_>>()?;

        (!links.is_empty()).then(|| MigrationPath(links.into()))
    }

    pub(crate) fn majors(&self, native: u16) -> Vec<u16> {
        let mut majors: Vec<u16> = (0..native)
            .filter(|&major| self.path(major, native).is_some())
            .collect();

        majors.push(native);

        majors
    }
}

#[derive(Clone)]
pub(crate) struct MigrationPath(Rc<[Rc<dyn MigrationStep>]>);

impl MigrationPath {
    pub(crate) fn versions(&self) -> VersionTable {
        let first = &self.0[0];

        VersionTable {
            version: first.from(),
            versions: first.versions(),
        }
    }

    pub(crate) fn up(
        &self,
        segment: MessageSegment,
        receiver: Role,
        type_id: u32,
        body: &[u8],
        minor: u16,
    ) -> Result<(u32, Vec<u8>), ErrorCode> {
        debug_assert!(segment.should_migrate_up(receiver));

        let (first, remainder) = self
            .0
            .split_first()
            .expect("a migration path has at least one step");
        let mut rewritten = first.migrate(
            segment,
            receiver,
            type_id,
            body,
            minor,
            BinaryWriter::NEWEST,
        )?;

        for link in remainder {
            let (type_id, body) = rewritten;

            rewritten = link.migrate(
                segment,
                receiver,
                type_id,
                &body,
                BinaryWriter::NEWEST,
                BinaryWriter::NEWEST,
            )?;
        }

        Ok(rewritten)
    }

    pub(crate) fn down(
        &self,
        segment: MessageSegment,
        receiver: Role,
        type_id: u32,
        body: &[u8],
        minor: u16,
    ) -> Result<(u32, Vec<u8>), ErrorCode> {
        debug_assert!(!segment.should_migrate_up(receiver));

        let last = self.0.len() - 1;
        let mut links = self.0.iter().rev().enumerate();
        let (_, first) = links
            .next()
            .expect("a migration path has at least one step");
        let write_minor = |step: usize| {
            if step == last {
                minor
            } else {
                BinaryWriter::NEWEST
            }
        };
        let mut rewritten = first.migrate(
            segment,
            receiver,
            type_id,
            body,
            BinaryWriter::NEWEST,
            write_minor(0),
        )?;

        for (step, link) in links {
            let (type_id, body) = rewritten;

            rewritten = link.migrate(
                segment,
                receiver,
                type_id,
                &body,
                BinaryWriter::NEWEST,
                write_minor(step),
            )?;
        }

        Ok(rewritten)
    }

    pub(crate) fn oldest(&self, receiver: Role, type_id: u32) -> Result<u32, ErrorCode> {
        self.0.iter().rev().try_fold(type_id, |type_id, link| {
            link.older_message(receiver, type_id)
                .ok_or(ErrorCode::UnsupportedVersion)
        })
    }
}
