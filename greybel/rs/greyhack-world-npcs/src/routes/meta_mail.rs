//! Operations on logged-in mail accounts (`MetaMailIntrinsics`).

use greyhack_world_economy::Connectivity;
use greyhack_world_router::RouteTable;

use crate::objects::{MailRef, ObjectTableExt};
use crate::routes::Result;
use crate::routes::protocol::{
    Email, Failure, InboxContent, IntoProtocol as _, MailDraft, MailFailure, MailRead, MailReads,
    MetaMailDelete, MetaMailDeleteAll, MetaMailFetch, MetaMailRead, MetaMailSend, Outcome,
};
use crate::{ComputerId, MailRefusal, SendMailFailure};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaMailFetch| {
            let world = &*ctx.modules;
            let mut objects = ctx.trees.objects(call.tree);
            let mut mail = objects.meta_mail_mut(message.mail)?;
            let result = fetch(world, &mut mail, &ctx.terminal_computers(call)).await;

            objects.set_meta_mail(message.mail, mail)?;

            result
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaMailRead<'static>| {
            let world = &*ctx.modules;
            let mut objects = ctx.trees.objects(call.tree);
            let mut mail = objects.meta_mail_mut(message.mail)?;
            let result = read(
                world,
                &mut mail,
                &ctx.terminal_computers(call),
                message.mail_ids.as_deref(),
            )
            .await;

            objects.set_meta_mail(message.mail, mail)?;

            result
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaMailSend| {
            let world = &*ctx.modules;

            send(
                world,
                &ctx.trees.objects(call.tree).meta_mail(message.mail)?,
                &ctx.terminal_computers(call),
                &message.draft,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaMailDelete<'static>| {
            let world = &*ctx.modules;
            let mut objects = ctx.trees.objects(call.tree);
            let mut mail = objects.meta_mail_mut(message.mail)?;
            let result = delete(
                world,
                &mut mail,
                &ctx.terminal_computers(call),
                message.mail_ids.as_deref(),
            )
            .await;

            objects.set_meta_mail(message.mail, mail)?;

            result
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaMailDeleteAll| {
            let world = &*ctx.modules;
            let mut objects = ctx.trees.objects(call.tree);
            let mut mail = objects.meta_mail_mut(message.mail)?;
            let result = delete_all(world, &mut mail, &ctx.terminal_computers(call)).await;

            objects.set_meta_mail(message.mail, mail)?;

            result
        },
    );
}

/// Returns a mail failure when the object's computer is missing or offline for `terminal_computers`
/// (`MetaMailIntrinsics`).
async fn raise_connection_failure(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    mail: &MailRef,
    terminal_computers: &[ComputerId],
) -> Result<Option<MailFailure>> {
    Ok(
        match world
            .module::<greyhack_world_economy::EconomyModule>()
            .coins
            .connectivity(mail.access.computer, terminal_computers)
            .await?
        {
            Connectivity::Missing => Some(MailFailure::ComputerUnavailable),
            Connectivity::Offline => Some(MailFailure::NoInternet),
            Connectivity::Online => None,
        },
    )
}

/// Refreshes the stored inbox (`HelperScript.HelperMailUser.RefreshAccount`) or returns the refresh failure.
async fn refresh(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    mail: &mut MailRef,
) -> Result<Option<MailFailure>> {
    Ok(
        match world
            .module::<crate::NpcModule>()
            .mail
            .refresh_mail(&mail.address)
            .await?
        {
            Ok(inbox) => {
                mail.emails = inbox;

                None
            }
            Err(MailRefusal::Missing) => Some(MailFailure::RefreshFailed),
            Err(MailRefusal::AccountInactive) => Some(MailFailure::AccountInactive),
        },
    )
}

/// Checks connection state and refreshes the account before an inbox operation.
async fn ready(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
) -> Result<Option<MailFailure>> {
    if let Some(failure) = raise_connection_failure(world, mail, terminal_computers).await? {
        return Ok(Some(failure));
    }

    refresh(world, mail).await
}

/// Converts world `email` into protocol data.
fn email(email: &crate::Email) -> Email {
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
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
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
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
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
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    mail: &MailRef,
    terminal_computers: &[ComputerId],
    draft: &MailDraft,
) -> Result<Outcome> {
    if matches!(draft, MailDraft::Invalid) {
        return Ok(Outcome::Null);
    }

    if let Some(failure) = raise_connection_failure(world, mail, terminal_computers).await? {
        return Ok(raise_mail_failure(failure));
    }

    Ok(match draft {
        MailDraft::Invalid => Outcome::Null,
        MailDraft::Refusal(refusal) => Outcome::Failure(Failure::Refusal(refusal.clone())),
        MailDraft::Email {
            to,
            subject,
            message,
        } => match world
            .module::<crate::NpcModule>()
            .mail
            .send_mail(&mail.address, to, subject, message)
            .await?
        {
            Ok(()) => Outcome::True,
            Err(SendMailFailure::NoAccount) => raise_mail_failure(MailFailure::NoAccount),
            Err(SendMailFailure::Refusal(refusal)) => {
                Outcome::Failure(Failure::Refusal(refusal.into_protocol()))
            }
            Err(SendMailFailure::AccountInactive) => {
                raise_mail_failure(MailFailure::AccountInactive)
            }
            Err(SendMailFailure::Undeliverable) => raise_mail_failure(MailFailure::NotDelivered),
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
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
    mail_ids: Option<&[String]>,
) -> Result<Outcome> {
    if let Some(failure) = ready(world, mail, terminal_computers).await? {
        return Ok(raise_mail_failure(failure));
    }

    let Some(mail_ids) = mail_ids else {
        return Ok(Outcome::Null);
    };
    let had_mail = world
        .module::<crate::NpcModule>()
        .mail
        .delete_mails(&mail.address, mail_ids)
        .await?;

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
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    mail: &mut MailRef,
    terminal_computers: &[ComputerId],
) -> Result<Outcome> {
    if let Some(failure) = ready(world, mail, terminal_computers).await? {
        return Ok(raise_mail_failure(failure));
    }

    world
        .module::<crate::NpcModule>()
        .mail
        .clear_inbox(&mail.address)
        .await?;
    mail.emails.clear();

    Ok(Outcome::True)
}

/// Wraps mail `failure` as an operation outcome.
const fn raise_mail_failure(failure: MailFailure) -> Outcome {
    Outcome::Failure(Failure::Mail(failure))
}
