use std::collections::HashMap;

pub const CORE: u8 = 1;
const CONTINUATION: u8 = 0x80;
const LAST_FRAME: u8 = 0x80;
const ROUTE_BYTES: usize = 9;
const DEPRECATION: u8 = 1;
const PAYLOAD: u8 = 0x7f;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slice {
    pub core: u8,
    pub channel: u32,
    pub from: u8,
    pub connection: u32,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Unaliased,
    Announced(u32),
    Aliased(u32),
}

#[derive(Debug, Default)]
pub struct AliasTable {
    routes: HashMap<u32, (u8, u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Message {
        type_id: u32,
        id: Option<u32>,
        body: Vec<u8>,
    },
    ResponseEnd {
        id: u32,
        body: Vec<u8>,
    },
    Failure {
        id: u32,
        code: u32,
        body: Vec<u8>,
    },
    Listen,
    Open {
        id: u32,
        core: Version,
        protocol: Version,
        fingerprint: u32,
        payload: Vec<u8>,
    },
    Ack {
        id: u32,
        connection: u32,
        core: Version,
        protocol: Version,
        fingerprint: u32,
        deprecation: Option<String>,
        payload: Vec<u8>,
    },
    Refuse {
        id: u32,
        reason: u32,
        core: Version,
        protocol: Version,
        majors: Vec<u16>,
        payload: Vec<u8>,
    },
    Ping,
    Pong,
    Close {
        reason: u32,
    },
    Cancel {
        id: u32,
    },
    ResponseStart {
        id: u32,
        body: Vec<u8>,
    },
    ResponseItem {
        id: u32,
        body: Vec<u8>,
    },
    ResponseCredit {
        id: u32,
        count: u32,
    },
    MessageItem {
        id: u32,
        body: Vec<u8>,
    },
    MessageEnd {
        id: u32,
        body: Vec<u8>,
    },
    MessageCredit {
        id: u32,
        count: u32,
    },
    Fragment {
        ends: bool,
        bytes: Vec<u8>,
    },
    Unknown {
        kind: u8,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

impl Version {
    fn encode(self, bytes: &mut Vec<u8>) {
        Slice::varuint(bytes, u64::from(self.major));
        Slice::varuint(bytes, u64::from(self.minor));
    }
}

impl Slice {
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Self {
        let mut slices = AliasTable::default().read(bytes);

        assert_eq!(slices.len(), 1, "a batch of one slice");

        slices.remove(0)
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        Self::encode_slices(self.core, &[(Key::Unaliased, self.clone())])
    }

    #[must_use]
    pub fn encode_slices(core: u8, slices: &[(Key, Self)]) -> Vec<u8> {
        let mut bytes = vec![core];

        Self::varuint(&mut bytes, slices.len() as u64);
        for (index, (key, slice)) in slices.iter().enumerate() {
            match *key {
                Key::Unaliased => {
                    bytes.push(0);
                    slice.encode_route(&mut bytes);
                }
                Key::Announced(alias) => {
                    Self::varuint(&mut bytes, u64::from(alias * 2 - 1));
                    slice.encode_route(&mut bytes);
                }
                Key::Aliased(alias) => Self::varuint(&mut bytes, u64::from(alias * 2)),
            }

            let frames = slice.encode_frames();

            if index + 1 < slices.len() {
                Self::varuint(&mut bytes, frames.len() as u64);
            }

            bytes.extend_from_slice(&frames);
        }

        bytes
    }

    fn encode_route(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.from);
        bytes.extend_from_slice(&self.channel.to_le_bytes());
        bytes.extend_from_slice(&self.connection.to_le_bytes());
    }

    fn encode_frames(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for (index, frame) in self.frames.iter().enumerate() {
            let (kind, payload) = frame.encode();

            if index + 1 == self.frames.len() {
                bytes.push(kind | LAST_FRAME);
            } else {
                bytes.push(kind);
                Self::varuint(&mut bytes, payload.len() as u64);
            }

            bytes.extend_from_slice(&payload);
        }

        bytes
    }

    pub fn varuint(bytes: &mut Vec<u8>, mut value: u64) {
        while value >= u64::from(CONTINUATION) {
            bytes.push((value.to_le_bytes()[0] & PAYLOAD) | CONTINUATION);
            value >>= 7;
        }

        bytes.push(value.to_le_bytes()[0]);
    }
}

impl AliasTable {
    pub fn read(&mut self, bytes: &[u8]) -> Vec<Slice> {
        self.read_with_keys(bytes)
            .into_iter()
            .map(|(_, slice)| slice)
            .collect()
    }

    pub fn read_with_keys(&mut self, bytes: &[u8]) -> Vec<(Key, Slice)> {
        let mut reader = Reader { bytes, position: 0 };
        let core = reader.byte();
        let count = reader.varuint_length();

        (0..count)
            .map(|index| {
                let key = reader.varuint_u32();
                let (key, route) = match key {
                    0 => (Key::Unaliased, reader.route()),
                    key if key % 2 == 1 => {
                        let alias = key.div_ceil(2);
                        let route = reader.route();

                        self.routes.insert(alias, route);

                        (Key::Announced(alias), route)
                    }
                    key => {
                        let alias = key / 2;
                        let route = *self
                            .routes
                            .get(&alias)
                            .unwrap_or_else(|| panic!("the alias {alias} was never announced"));

                        (Key::Aliased(alias), route)
                    }
                };
                let length = if index + 1 == count {
                    bytes.len() - reader.position
                } else {
                    reader.varuint_length()
                };
                let frames = Frame::decode_all(&bytes[reader.position..reader.position + length]);

                reader.position += length;

                let (from, channel, connection) = route;

                (
                    key,
                    Slice {
                        core,
                        channel,
                        from,
                        connection,
                        frames,
                    },
                )
            })
            .collect()
    }
}

impl Frame {
    fn decode_all(bytes: &[u8]) -> Vec<Self> {
        let mut reader = Reader { bytes, position: 0 };
        let mut frames = Vec::new();

        while reader.position < bytes.len() {
            let kind = reader.byte();
            let length = if kind & LAST_FRAME == 0 {
                reader.varuint_length()
            } else {
                bytes.len() - reader.position
            };
            let payload = &bytes[reader.position..reader.position + length];

            reader.position += length;
            frames.push(Self::decode(kind & !LAST_FRAME, payload));
        }

        frames
    }

    fn decode(kind: u8, payload: &[u8]) -> Self {
        let mut reader = Reader {
            bytes: payload,
            position: 0,
        };

        match kind {
            0 => {
                let key = reader.varuint_u32();
                let id = (key & 1 == 1).then(|| reader.varuint_u32());

                Self::Message {
                    type_id: key >> 1,
                    id,
                    body: reader.tail().to_vec(),
                }
            }
            1 => Self::ResponseEnd {
                id: reader.varuint_u32(),
                body: reader.tail().to_vec(),
            },
            2 => Self::Failure {
                id: reader.varuint_u32(),
                code: reader.varuint_u32(),
                body: reader.tail().to_vec(),
            },
            4 => Self::Listen,
            5 => Self::Open {
                id: reader.fixed32(),
                core: reader.version(),
                protocol: reader.version(),
                fingerprint: reader.fixed32(),
                payload: reader.tail().to_vec(),
            },
            6 => {
                let deprecated = reader.byte() & DEPRECATION != 0;

                Self::Ack {
                    id: reader.fixed32(),
                    connection: reader.fixed32(),
                    core: reader.version(),
                    protocol: reader.version(),
                    fingerprint: reader.fixed32(),
                    deprecation: deprecated.then(|| reader.text()),
                    payload: reader.tail().to_vec(),
                }
            }
            7 => Self::Refuse {
                id: reader.fixed32(),
                reason: reader.varuint_u32(),
                core: reader.version(),
                protocol: reader.version(),
                majors: (0..reader.varuint())
                    .map(|_| reader.varuint_u16())
                    .collect(),
                payload: reader.tail().to_vec(),
            },
            8 => Self::Ping,
            9 => Self::Pong,
            10 => Self::Close {
                reason: reader.varuint_u32(),
            },
            3 => Self::Cancel {
                id: reader.varuint_u32(),
            },
            11 => Self::ResponseStart {
                id: reader.varuint_u32(),
                body: reader.tail().to_vec(),
            },
            12 => Self::ResponseItem {
                id: reader.varuint_u32(),
                body: reader.tail().to_vec(),
            },
            13 => Self::ResponseCredit {
                id: reader.varuint_u32(),
                count: reader.varuint_u32(),
            },
            14 => Self::MessageItem {
                id: reader.varuint_u32(),
                body: reader.tail().to_vec(),
            },
            15 => Self::MessageEnd {
                id: reader.varuint_u32(),
                body: reader.tail().to_vec(),
            },
            16 => Self::MessageCredit {
                id: reader.varuint_u32(),
                count: reader.varuint_u32(),
            },
            17 => Self::Fragment {
                ends: reader.byte() == 1,
                bytes: reader.tail().to_vec(),
            },
            _ => Self::Unknown {
                kind,
                payload: payload.to_vec(),
            },
        }
    }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let (kind, payload) = self.encode();
        let mut bytes = vec![kind];

        Slice::varuint(&mut bytes, payload.len() as u64);
        bytes.extend_from_slice(&payload);

        bytes
    }

    fn encode(&self) -> (u8, Vec<u8>) {
        let mut payload = Vec::new();
        let kind = match self {
            Self::Message { type_id, id, body } => {
                let key = (type_id << 1) | u32::from(id.is_some());

                Slice::varuint(&mut payload, u64::from(key));
                if let Some(id) = id {
                    Slice::varuint(&mut payload, u64::from(*id));
                }

                payload.extend_from_slice(body);

                0
            }
            Self::ResponseEnd { id, body } => Self::with_body(&mut payload, *id, body, 1),
            Self::Failure { id, code, body } => {
                Slice::varuint(&mut payload, u64::from(*id));
                Slice::varuint(&mut payload, u64::from(*code));
                payload.extend_from_slice(body);

                2
            }
            Self::Cancel { id } => {
                Slice::varuint(&mut payload, u64::from(*id));

                3
            }
            Self::ResponseStart { id, body } => Self::with_body(&mut payload, *id, body, 11),
            Self::ResponseItem { id, body } => Self::with_body(&mut payload, *id, body, 12),
            Self::ResponseCredit { id, count } => Self::with_count(&mut payload, *id, *count, 13),
            Self::MessageItem { id, body } => Self::with_body(&mut payload, *id, body, 14),
            Self::MessageEnd { id, body } => Self::with_body(&mut payload, *id, body, 15),
            Self::MessageCredit { id, count } => Self::with_count(&mut payload, *id, *count, 16),
            Self::Listen => 4,
            Self::Open {
                id,
                core,
                protocol,
                fingerprint,
                payload: declared_payload,
            } => {
                payload.extend_from_slice(&id.to_le_bytes());
                core.encode(&mut payload);
                protocol.encode(&mut payload);
                payload.extend_from_slice(&fingerprint.to_le_bytes());
                payload.extend_from_slice(declared_payload);

                5
            }
            Self::Ack {
                id,
                connection,
                core,
                protocol,
                fingerprint,
                deprecation,
                payload: declared_payload,
            } => {
                payload.push(if deprecation.is_some() {
                    DEPRECATION
                } else {
                    0
                });
                payload.extend_from_slice(&id.to_le_bytes());
                payload.extend_from_slice(&connection.to_le_bytes());
                core.encode(&mut payload);
                protocol.encode(&mut payload);
                payload.extend_from_slice(&fingerprint.to_le_bytes());
                if let Some(note) = deprecation {
                    Slice::varuint(&mut payload, note.len() as u64);
                    payload.extend_from_slice(note.as_bytes());
                }

                payload.extend_from_slice(declared_payload);

                6
            }
            Self::Refuse {
                id,
                reason,
                core,
                protocol,
                majors,
                payload: declared_payload,
            } => {
                payload.extend_from_slice(&id.to_le_bytes());
                Slice::varuint(&mut payload, u64::from(*reason));
                core.encode(&mut payload);
                protocol.encode(&mut payload);
                Slice::varuint(&mut payload, majors.len() as u64);
                for major in majors {
                    Slice::varuint(&mut payload, u64::from(*major));
                }

                payload.extend_from_slice(declared_payload);

                7
            }
            Self::Ping => 8,
            Self::Pong => 9,
            Self::Close { reason } => {
                Slice::varuint(&mut payload, u64::from(*reason));

                10
            }
            Self::Fragment { ends, bytes } => {
                payload.push(u8::from(*ends));
                payload.extend_from_slice(bytes);

                17
            }
            Self::Unknown {
                kind,
                payload: unknown,
            } => {
                payload.extend_from_slice(unknown);

                *kind
            }
        };

        (kind, payload)
    }

    fn with_body(payload: &mut Vec<u8>, id: u32, body: &[u8], kind: u8) -> u8 {
        Slice::varuint(payload, u64::from(id));
        payload.extend_from_slice(body);

        kind
    }

    fn with_count(payload: &mut Vec<u8>, id: u32, count: u32, kind: u8) -> u8 {
        Slice::varuint(payload, u64::from(id));
        Slice::varuint(payload, u64::from(count));

        kind
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> u8 {
        let byte = self.bytes[self.position];

        self.position += 1;

        byte
    }

    fn varuint(&mut self) -> u64 {
        let mut value = 0;
        let mut shift = 0;

        loop {
            let byte = self.byte();

            value |= u64::from(byte & PAYLOAD) << shift;
            if byte & CONTINUATION == 0 {
                return value;
            }

            shift += 7;
        }
    }

    fn varuint_u32(&mut self) -> u32 {
        u32::try_from(self.varuint()).expect("the varuint fits 32 bits")
    }

    fn varuint_u16(&mut self) -> u16 {
        u16::try_from(self.varuint()).expect("the varuint fits 16 bits")
    }

    fn varuint_length(&mut self) -> usize {
        usize::try_from(self.varuint()).expect("the length fits the address space")
    }

    fn fixed32(&mut self) -> u32 {
        let bytes = self.bytes[self.position..self.position + 4]
            .try_into()
            .expect("four bytes");

        self.position += 4;

        u32::from_le_bytes(bytes)
    }

    fn route(&mut self) -> (u8, u32, u32) {
        let start = self.position;
        let from = self.byte();
        let channel = self.fixed32();
        let connection = self.fixed32();

        debug_assert_eq!(self.position - start, ROUTE_BYTES);

        (from, channel, connection)
    }

    fn version(&mut self) -> Version {
        Version {
            major: self.varuint_u16(),
            minor: self.varuint_u16(),
        }
    }

    fn text(&mut self) -> String {
        let length = self.varuint_length();
        let text = &self.bytes[self.position..self.position + length];

        self.position += length;

        String::from_utf8(text.to_vec()).expect("UTF-8")
    }

    fn tail(&mut self) -> &[u8] {
        let tail = &self.bytes[self.position..];

        self.position = self.bytes.len();

        tail
    }
}
