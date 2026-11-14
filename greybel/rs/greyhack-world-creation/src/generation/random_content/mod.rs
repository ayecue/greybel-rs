//! Random files and mails of generated computers (`Router.GenerateRandomContent`).

pub(crate) mod content;
pub(crate) mod content_file;
pub(crate) mod conversation_mail;
pub(crate) mod draft;

pub(crate) use content::RandomContent;
pub(crate) use content_file::ContentFile;
pub(crate) use conversation_mail::ConversationMail;
pub(crate) use draft::add_to_draft;

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
