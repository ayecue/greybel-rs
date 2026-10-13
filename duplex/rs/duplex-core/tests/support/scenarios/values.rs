use std::collections::BTreeMap;

use duplex_core::{
    BinaryEncode, BinaryWriter, ErrorCode, Message, Protocol, Requirement, Role, VersionInfo,
    Violation,
};
use duplex_test_support::conformance::calculator::{
    Add, Calculation, Confirm, Count, Divide, DivisionFailure, Log, Phase, Progress, Quotient,
    Reset, Sum,
};
use duplex_test_support::conformance::login::{Credentials, Denial, Login, Session, Whoami};
use duplex_test_support::conformance::vault::{Reveal, Vault};
use duplex_test_support::frames::{self, Frame, Slice, Version};
use serde_json::{Value, json};

const DECLARED_FAILURE: u32 = 0;
const CORE: Version = Version { major: 1, minor: 0 };

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Calculator,
    Login,
    Vault,
}

impl Subject {
    pub fn of(scenario: &Value) -> Result<Self> {
        match scenario.get("protocol").and_then(Value::as_str) {
            None | Some("calculator") => Ok(Self::Calculator),
            Some("login") => Ok(Self::Login),
            Some("vault") => Ok(Self::Vault),
            Some(other) => Err(format!("no protocol `{other}`")),
        }
    }

    fn channel_id(self) -> u32 {
        match self {
            Self::Calculator => <Calculation as Protocol>::CHANNEL_ID,
            Self::Login => <Login as Protocol>::CHANNEL_ID,
            Self::Vault => <Vault as Protocol>::CHANNEL_ID,
        }
    }

    fn version(self) -> Version {
        let version = match self {
            Self::Calculator => <Calculation as Protocol>::VERSION,
            Self::Login => <Login as Protocol>::VERSION,
            Self::Vault => <Vault as Protocol>::VERSION,
        };

        Version {
            major: version.major,
            minor: version.minor,
        }
    }

    fn versions(self) -> &'static [VersionInfo] {
        match self {
            Self::Calculator => <Calculation as Protocol>::VERSIONS,
            Self::Login => <Login as Protocol>::VERSIONS,
            Self::Vault => <Vault as Protocol>::VERSIONS,
        }
    }

    fn type_id(self, name: &str) -> Result<u32> {
        match (self, name) {
            (Self::Calculator, name) => CalculatorMessage::id(name),
            (Self::Login, "Whoami") => Ok(<Whoami as Message>::ID),
            (Self::Vault, "Reveal") => Ok(<Reveal as Message>::ID),
            (_, name) => Err(format!("no message `{name}`")),
        }
    }

    fn body(self, name: &str, fields: &Value, minor: u16) -> Result<Vec<u8>> {
        match (self, name) {
            (Self::Calculator, name) => CalculatorMessage::named(name, fields)?.body(minor),
            (Self::Login, "Whoami") => bytes(&Whoami {}, minor),
            (Self::Vault, "Reveal") => bytes(
                &Reveal {
                    name: text(fields, "name")?.into(),
                },
                minor,
            ),
            (_, name) => Err(format!("no message `{name}`")),
        }
    }

    fn reply(self, name: &str, value: &Value, minor: u16) -> Result<Vec<u8>> {
        match (self, name) {
            (Self::Calculator, name) => reply(name, value, minor),
            (Self::Login, "Whoami") => bytes(&value.as_str().ok_or("no name")?.to_owned(), minor),
            (Self::Vault, "Reveal") => bytes(&value.as_str().ok_or("no secret")?.to_owned(), minor),
            (_, name) => Err(format!("`{name}` is no request")),
        }
    }

    fn failure(self, name: &str, value: &Value, minor: u16) -> Result<Vec<u8>> {
        match self {
            Self::Calculator => failure(name, value, minor),
            Self::Login | Self::Vault => Err(format!("no failure of `{name}`")),
        }
    }

    fn payload(self, frame: &str, fields: &Value) -> Result<Vec<u8>> {
        if let Some(raw) = fields.get("payloadBytes") {
            return hex(raw.as_str().ok_or("no hex")?);
        }

        let Some(payload) = fields.get("payload") else {
            return Ok(Vec::new());
        };

        match (self, frame) {
            (Self::Login, "Open") => bytes(&Self::credentials(payload)?, BinaryWriter::NEWEST),
            (Self::Login, "Ack") => bytes(
                &Session {
                    greeting: text(payload, "greeting")?,
                },
                BinaryWriter::NEWEST,
            ),
            (Self::Login, "Refuse") => bytes(&Self::denial(payload)?, BinaryWriter::NEWEST),
            _ => Err(format!(
                "the {self:?} protocol carries nothing in `{frame}`"
            )),
        }
    }

    pub fn credentials(value: &Value) -> Result<Credentials> {
        Ok(Credentials {
            user: text(value, "user")?,
            token: text(value, "token")?,
        })
    }

    pub fn denial(value: &Value) -> Result<Denial> {
        match value.as_str() {
            Some("UnknownUser") => Ok(Denial::UnknownUser),
            Some("WrongToken") => Ok(Denial::WrongToken),
            _ => Err(format!("no denial {value}")),
        }
    }

    pub fn denial_json(denial: Denial) -> Value {
        json!(format!("{denial:?}"))
    }
}

const PHASES: [(&str, Phase); 3] = [
    ("Pending", Phase::Pending),
    ("Active", Phase::Active),
    ("Final", Phase::Final),
];

fn bytes<T: BinaryEncode + ?Sized>(value: &T, minor: u16) -> Result<Vec<u8>> {
    let mut writer = BinaryWriter::new();

    writer.set_minor(minor);
    value
        .write(&mut writer)
        .map_err(|error| error.to_string())?;

    Ok(writer.into_bytes())
}

pub fn entry(value: &Value) -> Result<(&str, &Value)> {
    match value.as_object() {
        Some(object) if object.len() == 1 => {
            let (key, value) = object
                .iter()
                .next()
                .expect("the one-entry object yields its entry");

            Ok((key.as_str(), value))
        }
        _ => Err(format!("not one entry: {value}")),
    }
}

pub fn number(value: &Value, key: &str) -> Result<i64> {
    value[key]
        .as_i64()
        .ok_or_else(|| format!("no integer `{key}` in {value}"))
}

pub fn id(value: &Value, key: &str) -> Result<u32> {
    u32::try_from(number(value, key)?).map_err(|error| error.to_string())
}

fn text(value: &Value, key: &str) -> Result<String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("no text `{key}` in {value}"))
}

pub fn hex(text: &str) -> Result<Vec<u8>> {
    (0..text.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(text.get(index..index + 2).unwrap_or(""), 16)
                .map_err(|error| error.to_string())
        })
        .collect()
}

pub fn code(value: &Value) -> Result<ErrorCode> {
    let name = value.as_str().ok_or_else(|| format!("no code: {value}"))?;

    (1..=u32::from(u8::MAX))
        .filter_map(|number| ErrorCode::try_from(number).ok())
        .find(|code| format!("{code:?}") == name)
        .ok_or_else(|| format!("no error code `{name}`"))
}

pub fn role(value: &Value) -> Result<Role> {
    match value.as_str() {
        Some("client") => Ok(Role::Client),
        Some("provider") => Ok(Role::Provider),
        _ => Err(format!("no role: {value}")),
    }
}

pub enum CalculatorMessage {
    Log(Log<'static>),
    Add(Add),
    Divide(Divide),
    Reset(Reset),
    Confirm(Confirm<'static>),
    Progress(Progress<'static>),
    Count(Count),
    Sum(Sum),
}

impl CalculatorMessage {
    pub fn of(value: &Value) -> Result<Self> {
        let (name, fields) = entry(value)?;

        Self::named(name, fields)
    }

    fn named(name: &str, fields: &Value) -> Result<Self> {
        Ok(match name {
            "Log" => Self::Log(Log {
                text: text(fields, "text")?.into(),
            }),
            "Add" => Self::Add(Add {
                a: number(fields, "a")?,
                b: number(fields, "b")?,
            }),
            "Divide" => Self::Divide(Divide {
                dividend: number(fields, "dividend")?,
                divisor: number(fields, "divisor")?,
                precision: fields
                    .get("precision")
                    .map(|precision| {
                        precision
                            .as_u64()
                            .and_then(|precision| u8::try_from(precision).ok())
                            .ok_or_else(|| format!("{precision} is no precision"))
                    })
                    .transpose()?,
            }),
            "Reset" => Self::Reset(Reset {}),
            "Count" => Self::Count(Count {
                to: u32::try_from(number(fields, "to")?).map_err(|error| error.to_string())?,
            }),
            "Sum" => Self::Sum(Sum {}),
            "Confirm" => Self::Confirm(Confirm {
                question: text(fields, "question")?.into(),
            }),
            "Progress" => Self::Progress(Progress {
                phase: phase(&fields["phase"])?,
                done_share: None,
                steps: Vec::new().into(),
                counts: BTreeMap::new(),
                note: None,
                eta: fields.get("eta").and_then(Value::as_f64),
            }),
            _ => return Err(format!("no message `{name}`")),
        })
    }

    pub fn progress_json(progress: &Progress<'_>) -> Value {
        let phase = PHASES
            .iter()
            .find(|(_, phase)| *phase == progress.phase)
            .map_or("", |(name, _)| name);
        let mut fields = json!({ "phase": phase });

        if let Some(eta) = progress.eta {
            fields["eta"] = json!(eta);
        }

        json!({ "Progress": fields })
    }

    pub fn id(name: &str) -> Result<u32> {
        Ok(match name {
            "Log" => <Log<'static> as Message>::ID,
            "Add" => <Add as Message>::ID,
            "Divide" => <Divide as Message>::ID,
            "Reset" => <Reset as Message>::ID,
            "Confirm" => <Confirm<'static> as Message>::ID,
            "Progress" => <Progress<'static> as Message>::ID,
            "Count" => <Count as Message>::ID,
            "Sum" => <Sum as Message>::ID,
            _ => return Err(format!("no message `{name}`")),
        })
    }

    fn body(&self, minor: u16) -> Result<Vec<u8>> {
        match self {
            Self::Log(log) => bytes(log, minor),
            Self::Add(add) => bytes(add, minor),
            Self::Divide(divide) => bytes(divide, minor),
            Self::Reset(reset) => bytes(reset, minor),
            Self::Confirm(confirm) => bytes(confirm, minor),
            Self::Progress(progress) => bytes(progress, minor),
            Self::Count(count) => bytes(count, minor),
            Self::Sum(sum) => bytes(sum, minor),
        }
    }
}

pub fn quotient_json(quotient: &Quotient) -> Value {
    match quotient {
        Quotient::Exact { value } => json!({ "Exact": { "value": value } }),
        Quotient::WithRemainder { value, remainder } => {
            json!({ "WithRemainder": { "value": value, "remainder": remainder } })
        }
    }
}

pub fn phase_json(phase: Phase) -> Value {
    PHASES
        .iter()
        .find(|(_, known)| *known == phase)
        .map_or(Value::Null, |(name, _)| json!(name))
}

fn phase(value: &Value) -> Result<Phase> {
    PHASES
        .iter()
        .find(|(name, _)| value.as_str() == Some(name))
        .map(|(_, phase)| *phase)
        .ok_or_else(|| format!("no phase {value}"))
}

fn quotient(value: &Value) -> Result<Quotient> {
    let (name, fields) = entry(value)?;

    match name {
        "Exact" => Ok(Quotient::Exact {
            value: number(fields, "value")?,
        }),
        "WithRemainder" => Ok(Quotient::WithRemainder {
            value: number(fields, "value")?,
            remainder: number(fields, "remainder")?,
        }),
        _ => Err(format!("no quotient `{name}`")),
    }
}

fn reply(message: &str, value: &Value, minor: u16) -> Result<Vec<u8>> {
    match message {
        "Add" | "Sum" => bytes(&value.as_i64().ok_or("no sum")?, minor),
        "Divide" => bytes(&quotient(value)?, minor),
        "Confirm" => bytes(&value.as_bool().ok_or("no confirmation")?, minor),
        "Count" => bytes(&value.as_u64().ok_or("no sum")?, minor),
        _ => Err(format!("`{message}` has no response")),
    }
}

fn segment(kind: &str, message: &str, value: &Value, minor: u16) -> Result<Vec<u8>> {
    match (kind, message) {
        ("ResponseStart", "Count") => bytes(&phase(value)?, minor),
        ("ResponseItem", "Count") => {
            let item = value.as_u64().and_then(|item| u32::try_from(item).ok());

            bytes(&item.ok_or("no number")?, minor)
        }
        ("MessageItem", "Sum") => bytes(&value.as_i64().ok_or("no term")?, minor),
        _ => Err(format!("no `{kind}` of `{message}`")),
    }
}

fn failure(message: &str, value: &Value, minor: u16) -> Result<Vec<u8>> {
    match (message, value.as_str()) {
        ("Divide", Some("ByZero")) => bytes(&DivisionFailure::ByZero, minor),
        ("Divide", Some("Refusal")) => bytes(&DivisionFailure::Refusal, minor),
        _ => Err(format!("no failure of `{message}`: {value}")),
    }
}

fn frame(value: &Value, minor: u16, subject: Subject) -> Result<Frame> {
    let (kind, fields) = entry(value)?;

    Ok(match kind {
        "Message" => {
            let id = fields.get("id").map(|_| id(fields, "id")).transpose()?;

            if let Some(body) = fields["body"].as_str() {
                let name = fields["message"].as_str().ok_or("no message")?;

                Frame::Message {
                    type_id: subject.type_id(name)?,
                    id,
                    body: hex(body)?,
                }
            } else {
                let (name, message) = message_of(fields)?;

                Frame::Message {
                    type_id: subject.type_id(name)?,
                    id,
                    body: subject.body(name, message, minor)?,
                }
            }
        }
        "ResponseEnd" => {
            let (name, value) = message_of(fields)?;

            Frame::ResponseEnd {
                id: id(fields, "id")?,
                body: subject.reply(name, value, minor)?,
            }
        }
        "ResponseStart" | "ResponseItem" | "MessageItem" | "MessageEnd" => {
            let id = id(fields, "id")?;
            let body = match (fields["body"].as_str(), message_of(fields)) {
                (Some(body), _) => hex(body)?,
                (None, Ok((name, value))) => segment(kind, name, value, minor)?,
                (None, Err(_)) => Vec::new(),
            };

            match kind {
                "ResponseStart" => Frame::ResponseStart { id, body },
                "ResponseItem" => Frame::ResponseItem { id, body },
                "MessageItem" => Frame::MessageItem { id, body },
                _ => Frame::MessageEnd { id, body },
            }
        }
        "ResponseCredit" => Frame::ResponseCredit {
            id: id(fields, "id")?,
            count: id(fields, "count")?,
        },
        "MessageCredit" => Frame::MessageCredit {
            id: id(fields, "id")?,
            count: id(fields, "count")?,
        },
        "Cancel" => Frame::Cancel {
            id: id(fields, "id")?,
        },
        "Fragment" => {
            let encoded = frame(&fields["of"], minor, subject)?.to_bytes();
            let index = |name: &str, default: usize| -> Result<usize> {
                fields.get(name).map_or(Ok(default), |index| {
                    index
                        .as_u64()
                        .and_then(|index| usize::try_from(index).ok())
                        .filter(|&index| index <= encoded.len())
                        .ok_or_else(|| format!("no index `{name}` in {} bytes", encoded.len()))
                })
            };
            let (from, to) = (index("from", 0)?, index("to", encoded.len())?);

            Frame::Fragment {
                ends: to == encoded.len(),
                bytes: encoded[from..to].to_vec(),
            }
        }
        "Failure" => {
            let id = id(fields, "id")?;

            if let Some(reason) = fields.get("rejected") {
                Frame::Failure {
                    id,
                    code: u32::from(code(reason)?),
                    body: match fields.get("violation") {
                        Some(violation) => bytes(&violation_of(violation)?, minor)?,
                        None => Vec::new(),
                    },
                }
            } else {
                let (name, value) = message_of(fields)?;

                Frame::Failure {
                    id,
                    code: DECLARED_FAILURE,
                    body: subject.failure(name, value, minor)?,
                }
            }
        }
        "Listen" => Frame::Listen,
        "Ping" => Frame::Ping,
        "Pong" => Frame::Pong,
        "Open" => {
            let protocol = version_of(fields, "protocol", subject.version())?;

            Frame::Open {
                id: id(fields, "id")?,
                core: version_of(fields, "core", CORE)?,
                protocol,
                fingerprint: fingerprint_of(fields, protocol, subject)?,
                payload: subject.payload(kind, fields)?,
            }
        }
        "Ack" => {
            let protocol = version_of(fields, "protocol", subject.version())?;

            Frame::Ack {
                id: id(fields, "id")?,
                connection: id(fields, "connection")?,
                core: version_of(fields, "core", CORE)?,
                protocol,
                fingerprint: fingerprint_of(fields, protocol, subject)?,
                deprecation: fields["deprecation"].as_str().map(str::to_owned),
                payload: subject.payload(kind, fields)?,
            }
        }
        "Refuse" => Frame::Refuse {
            id: id(fields, "id")?,
            reason: u32::from(code(&fields["reason"])?),
            core: version_of(fields, "core", CORE)?,
            protocol: version_of(fields, "protocol", subject.version())?,
            majors: match fields.get("majors") {
                Some(majors) => majors
                    .as_array()
                    .ok_or("majors are a list")?
                    .iter()
                    .map(|major| {
                        major
                            .as_u64()
                            .and_then(|major| u16::try_from(major).ok())
                            .ok_or_else(|| format!("{major} is no major"))
                    })
                    .collect::<Result<_>>()?,
                None => vec![subject.version().major],
            },
            payload: subject.payload(kind, fields)?,
        },
        "Close" => Frame::Close {
            reason: u32::from(code(&fields["reason"])?),
        },
        "Unknown" => Frame::Unknown {
            kind: u8::try_from(number(fields, "kind")?).map_err(|error| error.to_string())?,
            payload: hex(&text(fields, "payload")?)?,
        },
        _ => return Err(format!("no frame `{kind}`")),
    })
}

fn version_of(fields: &Value, key: &str, default: Version) -> Result<Version> {
    let Some(version) = fields.get(key) else {
        return Ok(default);
    };
    let text = version
        .as_str()
        .ok_or_else(|| format!("{key} is a version"))?;
    let (major, minor) = text
        .split_once('.')
        .ok_or_else(|| format!("`{text}` is no version"))?;
    let segment = |segment: &str| {
        segment
            .parse()
            .map_err(|_| format!("`{text}` is no version"))
    };

    Ok(Version {
        major: segment(major)?,
        minor: segment(minor)?,
    })
}

fn fingerprint_of(fields: &Value, version: Version, subject: Subject) -> Result<u32> {
    if fields.get("fingerprint").is_some() {
        return id(fields, "fingerprint");
    }

    Ok(subject
        .versions()
        .get(usize::from(version.minor))
        .filter(|_| version.major == subject.version().major)
        .map_or(0, |version| version.fingerprint))
}

fn message_of(fields: &Value) -> Result<(&str, &Value)> {
    fields
        .as_object()
        .and_then(|object| object.iter().find(|(key, _)| key.as_str() != "id"))
        .map(|(key, value)| (key.as_str(), value))
        .ok_or_else(|| format!("no message in {fields}"))
}

pub fn batch(value: &Value, from: Role, subject: Subject) -> Result<Vec<u8>> {
    if let Some(raw) = value.get("bytes") {
        return hex(raw.as_str().ok_or("no hex")?);
    }

    let minor = match value.get("minor") {
        Some(minor) => {
            u16::try_from(minor.as_u64().ok_or("no minor")?).map_err(|error| error.to_string())?
        }
        None => BinaryWriter::NEWEST,
    };
    let frames = value["frames"]
        .as_array()
        .ok_or("no frames")?
        .iter()
        .map(|value| frame(value, minor, subject))
        .collect::<Result<_>>()?;
    let from = match value.get("from") {
        Some(from) => role(from)?,
        None => from,
    };
    let batch = Slice {
        core: match value.get("core") {
            Some(core) => {
                u8::try_from(core.as_u64().ok_or("no core")?).map_err(|error| error.to_string())?
            }
            None => frames::CORE,
        },
        channel: match value.get("channel") {
            Some(_) => id(value, "channel")?,
            None => subject.channel_id(),
        },
        from: u8::try_from(u32::from(from)).map_err(|error| error.to_string())?,
        connection: id(value, "connection")?,
        frames,
    };
    let mut bytes = batch.encode();

    if let Some(cut) = value.get("truncate") {
        let cut =
            usize::try_from(cut.as_u64().ok_or("no length")?).map_err(|error| error.to_string())?;

        bytes.truncate(bytes.len().saturating_sub(cut));
    }

    Ok(bytes)
}

fn violation_of(value: &Value) -> Result<Violation> {
    let path = value["path"]
        .as_array()
        .ok_or("a violation has a path")?
        .iter()
        .map(|position| {
            position
                .as_u64()
                .and_then(|position| u32::try_from(position).ok())
                .ok_or_else(|| format!("{position} is no position"))
        })
        .collect::<Result<_>>()?;
    let requirement = match value["requirement"].as_str() {
        Some("MinLength") => Requirement::MinLength,
        Some("MaxLength") => Requirement::MaxLength,
        Some("Minimum") => Requirement::Minimum,
        Some("Maximum") => Requirement::Maximum,
        Some("Pattern") => Requirement::Pattern,
        Some("Limit") => Requirement::Limit,
        other => return Err(format!("{other:?} is no requirement")),
    };

    Ok(Violation { path, requirement })
}
