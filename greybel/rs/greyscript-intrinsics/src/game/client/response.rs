//! Converts game-server responses into script values, formatting failures for the receiving intrinsic.

use std::fmt::Write as _;

use greybel_interpreter::{Value, Vm};

use super::failure::failure_value;
use crate::error::IntrinsicError;
use crate::protocol::{
    ApplianceModel, CoinName, CredentialsInfo, FileChange, FileContent, MailRead, MailReads,
    MissionAcceptance, PackageList, PatchState, TradeOutcome, UserGroups, Whois,
};

/// Implements `response` for server responses that are null, a text value, or a failure.
macro_rules! text_response {
    ($($response:ident::$variant:ident, $doc:literal;)*) => {
        $(
            impl $response {
                #[doc = $doc]
                pub(in crate::game) fn response(
                    self,
                    intrinsic: &'static str,
                ) -> Result<Value, IntrinsicError> {
                    match self {
                        Self::Null => Ok(Value::Null),
                        Self::$variant(text) => Ok(Value::from(text)),
                        Self::Failure(failure) => failure_value(intrinsic, &failure),
                    }
                }
            }
        )*
    };
}

text_response! {
    FileContent::Content, "Returns the file content or the failure reason.";
    CoinName::Name, "Returns the coin name or the failure reason.";
    ApplianceModel::Model, "Returns the appliance model or the failure reason.";
    MissionAcceptance::Accepted, "Returns the address of the first network of the mission, or the failure reason.";
}

impl CredentialsInfo {
    /// Formats the workstation credentials as the NPC's role and full name, or returns the failure
    /// reason (`TrafficNetIntrinsics.get_credentials_info`).
    pub(in crate::game) fn response(
        self,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        match self {
            Self::Null => Ok(Value::Null),
            Self::Info { role, name } => Ok(Value::from(format!("{role} {name}"))),
            Self::Failure(failure) => failure_value(intrinsic, &failure),
        }
    }
}

impl FileChange {
    /// Returns `""` on success or the failure reason.
    pub(in crate::game) fn response(
        self,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        match self {
            Self::Null => Ok(Value::Null),
            Self::Done => Ok(Value::from("")),
            Self::Failure(failure) => failure_value(intrinsic, &failure),
        }
    }
}

impl PackageList {
    /// Formats packages as listed by `search` or `show` (`AptClientIntrinsics`).
    pub(in crate::game) fn response(
        self,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        let packages = match self {
            Self::Packages(packages) => packages,
            Self::Failure(failure) => return failure_value(intrinsic, &failure),
        };

        Ok(Value::from(
            packages
                .iter()
                .map(|package| match (&package.description, intrinsic) {
                    (None, _) => format!("<b>{}</b>\nMissing description\n\n", package.name),
                    (Some(description), "search") => {
                        format!("<b>{}</b>\n\n{description}", package.name)
                    }
                    (Some(description), _) => {
                        format!("<b>{}</b>\n{description}\n\n", package.name)
                    }
                })
                .collect::<String>(),
        ))
    }
}

impl UserGroups {
    /// Formats one group per line.
    pub(in crate::game) fn response(
        self,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        match self {
            Self::Null => Ok(Value::Null),
            Self::Groups(groups) => Ok(Value::from(groups.iter().fold(
                String::new(),
                |mut text, group| {
                    let _ = writeln!(text, "{group}");

                    text
                },
            ))),
            Self::Failure(failure) => failure_value(intrinsic, &failure),
        }
    }
}

impl MailReads {
    /// Formats `read` results as message text or `Mail not found` per requested mail (`MetaMailIntrinsics.read`).
    pub(in crate::game) fn response(
        self,
        vm: &mut Vm,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        match self {
            Self::Null => Ok(Value::Null),
            Self::Mails(mails) => {
                let texts = mails
                    .into_iter()
                    .map(|mail| {
                        Value::from(match mail {
                            MailRead::Found(mail) => {
                                let mut text =
                                    format!("\nFrom: {}\nSubject: {}\n", mail.from, mail.subject);

                                for message in std::iter::once(&mail.body).chain(&mail.replies) {
                                    text.push_str(message);
                                    text.push_str("\n\n");
                                }

                                text
                            }
                            MailRead::NotFound => "Mail not found".to_owned(),
                        })
                    })
                    .collect();

                Ok(Value::List(vm.heap_mut().add_list(texts)))
            }
            Self::Failure(failure) => failure_value(intrinsic, &failure),
        }
    }
}

impl Whois {
    /// Formats the network owner as displayed by `whois`.
    pub(in crate::game) fn response(self) -> Value {
        Value::from(match self {
            Self::UnknownAddress => "Address not found".to_owned(),
            Self::NoInfo => "No Info available".to_owned(),
            Self::Contact {
                domain,
                name,
                mail,
                phone,
            } => format!(
                "Domain name: {domain}\nAdministrative contact: {name}\nEmail address: {mail}\nPhone: {phone}"
            ),
        })
    }
}

impl TradeOutcome {
    /// Formats a `buy_coin` or `sell_coin` order result (`BlockchainSystem.BuyCoin`, `SellCoin`).
    pub(in crate::game) fn response(
        self,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        Ok(match self {
            Self::Null => Value::Null,
            Self::Complete => Value::TRUE,
            Self::Pending if intrinsic == "sell_coin" => Value::from(
                "Not enough buyers found. Transaction is pending until a buyer is found.",
            ),
            Self::Pending => Value::from(
                "Not enough sellers found. Transaction is pending until a seller is found.",
            ),
            Self::Partial => Value::from(
                "A partial transaction has occurred. The data of your offer has been updated",
            ),
            Self::Failure(failure) => return failure_value(intrinsic, &failure),
        })
    }
}

impl PatchState {
    /// Returns library patch status or its self-patch date.
    pub(in crate::game) fn response(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Unpatched => Value::FALSE,
            Self::Patched => Value::TRUE,
            Self::AutoPatchAt(date) => Value::from(date),
        }
    }
}
