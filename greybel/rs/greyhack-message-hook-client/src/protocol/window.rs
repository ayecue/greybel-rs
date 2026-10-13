use super::payload::decompress;

/// A message's tagged `pid` field: absent, one PID, a PID list, or a compressed JSON PID list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowPidKind<'a> {
    None,
    One(i32),
    List(Vec<i32>),
    /// PID list encoded as compressed JSON.
    Payload(&'a [u8]),
}

impl WindowPidKind<'_> {
    #[must_use]
    pub fn contains(&self, pid: i32) -> bool {
        match self {
            Self::None => false,
            Self::One(own) => *own == pid,
            Self::List(pids) => pids.contains(&pid),
            Self::Payload(_) => self.to_vec().contains(&pid),
        }
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<i32> {
        match self {
            Self::None => Vec::new(),
            Self::One(pid) => vec![*pid],
            Self::List(pids) => pids.clone(),
            Self::Payload(payload) => decompress(payload)
                .ok()
                .and_then(|json| serde_json::from_slice(&json).ok())
                .unwrap_or_default(),
        }
    }
}
