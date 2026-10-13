pub struct PosixPath;

impl PosixPath {
    const SEPARATOR: u8 = b'/';

    #[must_use]
    pub fn is_absolute(path: &str) -> bool {
        path.starts_with('/')
    }

    #[must_use]
    pub fn normalize(path: &str) -> String {
        if path.is_empty() {
            return ".".to_owned();
        }

        let is_absolute = Self::is_absolute(path);
        let trailing_separator = path.ends_with('/');
        let mut normalized_path = Self::normalize_string(path, !is_absolute);

        if normalized_path.is_empty() {
            if is_absolute {
                return "/".to_owned();
            }

            return if trailing_separator { "./" } else { "." }.to_owned();
        }

        if trailing_separator {
            normalized_path.push('/');
        }

        if is_absolute {
            normalized_path.insert(0, '/');
        }

        normalized_path
    }

    #[must_use]
    pub fn join(paths: &[&str]) -> String {
        let parts: Vec<&str> = paths
            .iter()
            .copied()
            .filter(|path| !path.is_empty())
            .collect();

        if parts.is_empty() {
            return ".".to_owned();
        }

        Self::normalize(&parts.join("/"))
    }

    #[must_use]
    pub fn resolve(paths: &[&str]) -> String {
        let mut path_so_far = String::new();
        let mut is_absolute = false;

        for path in paths.iter().rev() {
            if is_absolute {
                break;
            }

            if path.is_empty() {
                continue;
            }

            path_so_far = format!("{path}/{path_so_far}");
            is_absolute = Self::is_absolute(path);
        }

        let normalized_body = Self::normalize_string(&path_so_far, !is_absolute);

        if is_absolute {
            format!("/{normalized_body}")
        } else if normalized_body.is_empty() {
            ".".to_owned()
        } else {
            normalized_body
        }
    }

    #[must_use]
    pub fn dirname(path: &str) -> &str {
        if path.is_empty() {
            return ".";
        }

        let bytes = path.as_bytes();
        let has_root = bytes[0] == Self::SEPARATOR;
        let mut end = None;
        let mut matched_slash = true;

        for index in (1..bytes.len()).rev() {
            if bytes[index] == Self::SEPARATOR {
                if !matched_slash {
                    end = Some(index);
                    break;
                }
            } else {
                matched_slash = false;
            }
        }

        match end {
            None if has_root => "/",
            None => ".",
            Some(1) if has_root => "//",
            Some(end) => &path[..end],
        }
    }

    #[must_use]
    pub fn basename(path: &str) -> &str {
        let bytes = path.as_bytes();
        let mut start = 0;
        let mut end = None;
        let mut matched_slash = true;

        for index in (0..bytes.len()).rev() {
            if bytes[index] == Self::SEPARATOR {
                if !matched_slash {
                    start = index + 1;
                    break;
                }
            } else if end.is_none() {
                matched_slash = false;
                end = Some(index + 1);
            }
        }

        end.map_or("", |end| &path[start..end])
    }

    #[must_use]
    pub fn extname(path: &str) -> &str {
        let bytes = path.as_bytes();
        let mut start_dot = None;
        let mut start_part = 0;
        let mut end = None;
        let mut matched_slash = true;
        let mut pre_dot_state = 0;

        for index in (0..bytes.len()).rev() {
            let byte = bytes[index];

            if byte == Self::SEPARATOR {
                if !matched_slash {
                    start_part = index + 1;
                    break;
                }

                continue;
            }

            if end.is_none() {
                matched_slash = false;
                end = Some(index + 1);
            }

            if byte == b'.' {
                if start_dot.is_none() {
                    start_dot = Some(index);
                } else if pre_dot_state != 1 {
                    pre_dot_state = 1;
                }
            } else if start_dot.is_some() {
                pre_dot_state = -1;
            }
        }

        let (Some(start_dot), Some(end)) = (start_dot, end) else {
            return "";
        };

        if pre_dot_state == 0
            || (pre_dot_state == 1 && start_dot == end - 1 && start_dot == start_part + 1)
        {
            return "";
        }

        &path[start_dot..end]
    }

    fn normalize_string(path: &str, allow_above_root: bool) -> String {
        let bytes = path.as_bytes();
        let mut normalized_text = String::new();
        let mut last_segment_length = 0;
        let mut last_slash: Option<usize> = None;
        let mut dots = 0;
        let mut code = 0;

        for index in 0..=bytes.len() {
            if index < bytes.len() {
                code = bytes[index];
            } else if code == Self::SEPARATOR {
                break;
            } else {
                code = Self::SEPARATOR;
            }

            if code != Self::SEPARATOR {
                if code == b'.' && dots != -1 {
                    dots += 1;
                } else {
                    dots = -1;
                }

                continue;
            }

            let segment_start = last_slash.map_or(0, |slash| slash + 1);

            if segment_start == index || dots == 1 {
            } else if dots == 2 {
                let parent_segment = normalized_text.len() >= 2
                    && last_segment_length == 2
                    && normalized_text.ends_with("..");

                if !parent_segment {
                    if normalized_text.len() > 2 {
                        match normalized_text.len().checked_sub(last_segment_length + 1) {
                            Some(last_slash_index) => {
                                normalized_text.truncate(last_slash_index);
                                last_segment_length = match normalized_text.rfind('/') {
                                    Some(slash) => normalized_text.len() - 1 - slash,
                                    None => normalized_text.len(),
                                };
                            }
                            None => {
                                normalized_text.clear();
                                last_segment_length = 0;
                            }
                        }

                        last_slash = Some(index);
                        dots = 0;
                        continue;
                    } else if !normalized_text.is_empty() {
                        normalized_text.clear();
                        last_segment_length = 0;
                        last_slash = Some(index);
                        dots = 0;
                        continue;
                    }
                }

                if allow_above_root {
                    normalized_text.push_str(if normalized_text.is_empty() {
                        ".."
                    } else {
                        "/.."
                    });
                    last_segment_length = 2;
                }
            } else {
                if !normalized_text.is_empty() {
                    normalized_text.push('/');
                }

                normalized_text.push_str(&path[segment_start..index]);
                last_segment_length = index - segment_start;
            }

            last_slash = Some(index);
            dots = 0;
        }

        normalized_text
    }
}
