use crate::atoms::Outcome;
use crate::atoms::atomic_write::write_atomically;

use super::super::Atom;
use std::io::Write;
use std::{fs::File, path::PathBuf};

#[derive(Debug)]
pub struct Download {
    pub url: String,
    pub to: PathBuf,
}

impl std::fmt::Display for Download {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HttpDownload from {} to {}", self.url, self.to.display())
    }
}

impl Atom for Download {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // Initial implementation will return false if the local file
        // doesn't exist. I'd like to include a SHA to verify the
        // correct version exists; or perhaps a TTL when omitted?

        Ok(Outcome {
            side_effects: vec![],
            should_run: !PathBuf::from(&self.to).exists(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let content = reqwest::blocking::get(&self.url)?
            .error_for_status()?
            .bytes()?;

        write_atomically(&self.to, |tmp| Ok(File::create(tmp)?.write_all(&content)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    /// Serves one canned HTTP response on a loopback port, then closes the
    /// connection, so a test controls the status line and can cut the body
    /// short of its Content-Length.
    fn serve_once(response: &'static [u8]) -> String {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            stream.write_all(response).unwrap();
        });
        format!("http://{addr}/tool")
    }

    /// Asserts a failed download left nothing in `dir`, so `plan()` runs the
    /// step again next time.
    fn assert_failed_download_left_no_trace(atom: &Download, dir: &std::path::Path) {
        assert!(!atom.to.exists(), "file left at {}", atom.to.display());
        assert!(atom.plan().unwrap().should_run);
        let leftovers: Vec<_> = std::fs::read_dir(dir).unwrap().collect();
        assert!(leftovers.is_empty(), "leftover files: {leftovers:?}");
    }

    #[test]
    fn download_of_http_error_status_leaves_no_file() {
        let tmp = tempdir().unwrap();
        let url = serve_once(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found",
        );
        let mut atom = Download {
            url,
            to: tmp.path().join("tool"),
        };
        let msg = atom.execute().err().unwrap().to_string();
        assert!(msg.contains("404"), "msg was: {msg}");
        assert_failed_download_left_no_trace(&atom, tmp.path());
    }

    #[test]
    fn download_cut_short_of_content_length_leaves_no_file() {
        let tmp = tempdir().unwrap();
        let url = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\nonly part",
        );
        let mut atom = Download {
            url,
            to: tmp.path().join("tool"),
        };
        assert!(atom.execute().is_err());
        assert_failed_download_left_no_trace(&atom, tmp.path());
    }

    #[test]
    fn display_format() {
        let atom = Download {
            url: "https://example.com/file.zip".to_string(),
            to: std::path::PathBuf::from("/tmp/file.zip"),
        };
        let display = format!("{atom}");
        assert!(display.contains("example.com"));
        assert!(display.contains("file.zip"));
    }

    #[test]
    fn plan_should_not_run_when_file_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let existing = tmp.path().join("existing.txt");
        std::fs::write(&existing, b"content").unwrap();

        let atom = Download {
            url: "https://example.com/file".to_string(),
            to: existing,
        };
        assert!(!atom.plan().unwrap().should_run);
    }

    #[test]
    fn it_can() {
        let tmpdir = tempdir().unwrap();
        let to_file = tmpdir.path().join("download");

        let mut atom = Download {
            url: String::from(
                "https://www.google.com/images/branding/googlelogo/2x/googlelogo_color_272x92dp.png",
            ),
            to: to_file,
        };

        assert_eq!(true, atom.plan().unwrap().should_run);

        let result = atom.execute();
        assert_eq!(true, result.is_ok());
        assert_eq!(false, atom.plan().unwrap().should_run);
    }
}
