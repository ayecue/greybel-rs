use greyhack_behaviour::{LogAction, LogEntry};

pub fn entry(action: LogAction, ip: &str) -> LogEntry {
    LogEntry {
        action,
        ip: ip.to_owned(),
        fecha: "01/Dec/2000 - 06:00".to_owned(),
        puerto: 22,
        file: String::new(),
        bounce_ip: String::new(),
        player_net_id: "abc".to_owned(),
        tutorial: false,
        token_trace: "token".to_owned(),
    }
}
