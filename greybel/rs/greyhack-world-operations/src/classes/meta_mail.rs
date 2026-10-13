//! Operations on logged-in mail accounts (`MetaMailIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{ComputerId, Connectivity, MailRefusal, SendMailFailure, World};

use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::MailRef;
use crate::protocol::{
    Email, Failure, GameServer, InboxContent, MailDraft, MailFailure, MailRead, MailReads,
    MetaMailDelete, MetaMailDeleteAll, MetaMailFetch, MetaMailRead, MetaMailSend, Outcome,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: MetaMailFetch, caller| {
            fetch(
                world,
                objects.meta_mail_mut(message.mail)?,
                &caller.terminal_computers,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: MetaMailRead<'static>, caller| {
            read(
                world,
                objects.meta_mail_mut(message.mail)?,
                &caller.terminal_computers,
                message.mail_ids.as_deref(),
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: MetaMailSend, caller| {
            send(
                world,
                objects.meta_mail(message.mail)?,
                &caller.terminal_computers,
                &message.draft,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: MetaMailDelete<'static>, caller| {
            delete(
                world,
                objects.meta_mail_mut(message.mail)?,
                &caller.terminal_computers,
                message.mail_ids.as_deref(),
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: MetaMailDeleteAll, caller| {
            delete_all(
                world,
                objects.meta_mail_mut(message.mail)?,
                &caller.terminal_computers,
            )
            .await
        },
    );
}

/// Returns a mail failure when the object's computer is missing or offline for `terminal_computers`
/// (`MetaMailIntrinsics`).
async fn unusable(
    world: &World,
    mail: &MailRef,
    terminal_computers: &[ComputerId],
) -> Result<Option<MailFailure>> {
    Ok(
        match world
            .connectivity(mail.access.computer, terminal_computers)
            .await?
        {
            Connectivity::Gone => Some(MailFailure::ComputerUnavailable),
            Connectivity::Offline => Some(MailFailure::NoInternet),
            Connectivity::Online => None,
        },
    )
}

/// Refreshes the stored inbox (`HelperScript.HelperMailUser.RefreshAccount`) or returns the refresh failure.
async fn refresh(world: &World, mail: &mut MailRef) -> Result<Option<MailFailure>> {
    Ok(match world.refresh_mail(&mail.address).await? {
        Ok(inbox) => {
            mail.emails = inbox;

            None
        }
        Err(MailRefusal::Missing) => Some(MailFailure::RefreshFailed),
        Err(MailRefusal::AccountInactive) => Some(MailFailure::AccountInactive),
    })
}

/// Checks connection state and refreshes the account before an inbox operation.
async fn ready(
    world: &World,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
) -> Result<Option<MailFailure>> {
    if let Some(failure) = unusable(world, mail, terminal_computers).await? {
        return Ok(Some(failure));
    }

    refresh(world, mail).await
}

/// Converts world `email` into protocol data.
fn email(email: &greyhack_world::Email) -> Email {
    Email {
        id: email.id.clone(),
        from: email.from.clone(),
        subject: email.subject.clone(),
        body: email.body.clone(),
        replies: email.replies.clone(),
    }
}

/// Refreshes and returns the account inbox.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn fetch(
    world: &World,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
) -> Result<InboxContent> {
    if let Some(failure) = ready(world, mail, terminal_computers).await? {
        return Ok(InboxContent::Failure(Failure::Mail(failure)));
    }

    Ok(InboxContent::Emails(
        mail.emails.iter().map(email).collect(),
    ))
}

/// Refreshes the inbox and returns messages selected by `mail_ids`; non-text IDs return null.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn read(
    world: &World,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
    mail_ids: Option<&[String]>,
) -> Result<MailReads> {
    if let Some(failure) = ready(world, mail, terminal_computers).await? {
        return Ok(MailReads::Failure(Failure::Mail(failure)));
    }

    let Some(mail_ids) = mail_ids else {
        return Ok(MailReads::Null);
    };

    Ok(MailReads::Mails(
        mail_ids
            .iter()
            .map(|id| {
                mail.emails
                    .iter()
                    .find(|candidate| candidate.id == *id)
                    .map_or(MailRead::NotFound, |candidate| {
                        MailRead::Found(email(candidate))
                    })
            })
            .collect(),
    ))
}

/// Sends `draft` without refreshing the account first (`MetaMailIntrinsics.send`).
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn send(
    world: &mut World,
    mail: &MailRef,
    terminal_computers: &[ComputerId],
    draft: &MailDraft,
) -> Result<Outcome> {
    if matches!(draft, MailDraft::Invalid) {
        return Ok(Outcome::Null);
    }

    if let Some(failure) = unusable(world, mail, terminal_computers).await? {
        return Ok(mail_failure(failure));
    }

    Ok(match draft {
        MailDraft::Invalid => Outcome::Null,
        MailDraft::Refusal(refusal) => Outcome::Failure(Failure::Refusal(refusal.clone())),
        MailDraft::Email {
            to,
            subject,
            message,
        } => match world.send_mail(&mail.address, to, subject, message).await? {
            Ok(()) => Outcome::True,
            Err(SendMailFailure::NoAccount) => mail_failure(MailFailure::NoAccount),
            Err(SendMailFailure::Refusal(refusal)) => {
                Outcome::Failure(Failure::Refusal(refusal.into()))
            }
            Err(SendMailFailure::AccountInactive) => mail_failure(MailFailure::AccountInactive),
            Err(SendMailFailure::Undeliverable) => mail_failure(MailFailure::NotDelivered),
        },
    })
}

/// Refreshes the inbox and deletes `mail_ids`. Returns true if any message existed, or null if none did
/// or an ID is not text.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete(
    world: &mut World,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
    mail_ids: Option<&[String]>,
) -> Result<Outcome> {
    if let Some(failure) = ready(world, mail, terminal_computers).await? {
        return Ok(mail_failure(failure));
    }

    let Some(mail_ids) = mail_ids else {
        return Ok(Outcome::Null);
    };
    let had_mail = world.delete_mails(&mail.address, mail_ids).await?;

    mail.emails.retain(|email| !mail_ids.contains(&email.id));

    Ok(if had_mail {
        Outcome::True
    } else {
        Outcome::Null
    })
}

/// Refreshes the account and deletes all inbox messages.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete_all(
    world: &mut World,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
) -> Result<Outcome> {
    if let Some(failure) = ready(world, mail, terminal_computers).await? {
        return Ok(mail_failure(failure));
    }

    world.clear_inbox(&mail.address).await?;
    mail.emails.clear();

    Ok(Outcome::True)
}

/// Wraps mail `failure` as an operation outcome.
const fn mail_failure(failure: MailFailure) -> Outcome {
    Outcome::Failure(Failure::Mail(failure))
}
