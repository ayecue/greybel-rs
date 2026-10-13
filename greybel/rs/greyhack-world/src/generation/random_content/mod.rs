//! Random files and mails of generated computers (`Router.GenerateRandomContent`).

mod content;
mod content_file;
mod conversation_mail;
mod draft;

pub(crate) use content::RandomContent;
pub(crate) use content_file::ContentFile;
pub(crate) use conversation_mail::ConversationMail;
pub(crate) use draft::add_to_draft;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
