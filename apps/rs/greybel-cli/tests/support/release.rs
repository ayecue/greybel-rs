//! A release feed on a local HTTP server, and copies of `greybel` to update.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use super::Project;

/// Serves `latest.json` and one archive whose `greybel` program is `program`, until the test ends.
pub struct ReleaseFeedServer {
    pub url: String,
}

impl ReleaseFeedServer {
    /// A feed whose newest release is `version`, with an archive for this build's target. `sha256` overrides the
    /// archive's SHA-256 in `latest.json`.
    pub fn start(version: &str, program: &str, sha256: Option<&str>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let url = format!("http://{}", listener.local_addr().expect("its address"));
        let target = env!("GREYBEL_TARGET");
        let (name, archive) = Self::archive(program);
        let archive_url = format!("{url}/{version}/{name}");
        let sha256 = sha256.map_or_else(
            || {
                Sha256::digest(&archive)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            },
            str::to_owned,
        );
        let latest = serde_json::json!({
            "version": version,
            "tag": format!("greybel/v{version}"),
            "assets": {
                target: { "url": archive_url, "sha256": sha256, "size": archive.len() }
            }
        });
        let files: HashMap<String, Vec<u8>> = HashMap::from([
            ("/latest.json".to_owned(), latest.to_string().into_bytes()),
            (format!("/{version}/{name}"), archive),
        ]);

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };
                let mut request = String::new();
                let _ = BufReader::new(&stream).read_line(&mut request);
                let path = request.split_whitespace().nth(1).unwrap_or_default();
                let (status, body) = files.get(path).map_or(("404 Not Found", &[][..]), |body| {
                    ("200 OK", body.as_slice())
                });
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(body);
            }
        });

        Self { url }
    }

    /// The archive file name and contents: a `.zip` on Windows, a `.tar.gz` elsewhere.
    fn archive(program: &str) -> (String, Vec<u8>) {
        if cfg!(windows) {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));

            zip.start_file(
                "greybel-9.9.9/greybel.exe",
                zip::write::SimpleFileOptions::default(),
            )
            .expect("an entry");
            zip.write_all(program.as_bytes()).expect("its content");

            (
                "greybel.zip".to_owned(),
                zip.finish().expect("the archive").into_inner(),
            )
        } else {
            let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
                Vec::new(),
                flate2::Compression::fast(),
            ));
            let mut header = tar::Header::new_gnu();

            header.set_size(program.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, "greybel-9.9.9/greybel", program.as_bytes())
                .expect("an entry");

            (
                "greybel.tar.gz".to_owned(),
                tar.into_inner()
                    .expect("the archive")
                    .finish()
                    .expect("compressed"),
            )
        }
    }
}

/// A copy of `greybel` in `project`, which `update` replaces.
pub fn greybel_copy(project: &Project) -> PathBuf {
    let copy = project.path(if cfg!(windows) {
        "greybel.exe"
    } else {
        "greybel"
    });

    std::fs::copy(env!("CARGO_BIN_EXE_greybel"), &copy).expect("copies greybel");

    copy
}

/// Runs the copy at `program` with `arguments` against `feed`.
pub fn run_update(program: &Path, arguments: &[&str], feed: &ReleaseFeedServer) -> (bool, String) {
    let output = Command::new(program)
        .args(arguments)
        .env("GREYBEL_UPDATE_URL", &feed.url)
        .output()
        .expect("greybel runs");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr),
    )
}
