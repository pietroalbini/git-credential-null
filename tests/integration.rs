use std::path::{Component, Path, PathBuf, Prefix, PrefixComponent};
use std::process::Command;
use wait_timeout::ChildExt;

static BIN: &str = "git-credential-null";

#[test]
fn test_output_get_command() {
    let out = Command::new(binary(BIN)).args(&["get"]).output().unwrap();
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    assert_eq!(b"quit=1\n", out.stdout.as_slice());
}

#[test]
fn test_output_other_commands() {
    for arg in &["store", "erase", "this-doesnt-exist"] {
        let out = Command::new(binary(BIN)).args(&[arg]).output().unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
        assert!(out.stderr.is_empty());
    }
}

#[test]
fn test_git_clone_without_password() {
    assert!(test_clone_works(false));
}

#[test]
fn test_git_clone_with_password() {
    assert!(!test_clone_works(true));
}

fn test_clone_works(auth: bool) -> bool {
    let git_repo = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .arg("init")
        .arg(git_repo.path())
        .output()
        .unwrap()
        .status
        .success());
    assert!(Command::new("git")
        .arg("-c")
        .arg("commit.gpgsign=false")
        .arg("-c")
        .arg("user.name=test")
        .arg("-c")
        .arg("user.email=test@example.com")
        .arg("commit")
        .arg("-m")
        .arg("initial commit")
        .arg("--allow-empty")
        .current_dir(git_repo.path())
        .output()
        .unwrap()
        .status
        .success());
    assert!(Command::new("git")
        .arg("update-server-info")
        .current_dir(git_repo.path())
        .output()
        .unwrap()
        .status
        .success());

    let port = http_server(git_repo.path().join(".git"), auth);

    let clone_dest = tempfile::tempdir().unwrap();
    let mut child = Command::new("git")
        .arg("-c")
        .arg("credential.helper=")
        .arg("-c")
        .arg(format!(
            "credential.helper={}",
            binary(BIN).display().to_string().replace('\\', "/")
        ))
        .arg("clone")
        .arg(format!("http://localhost:{}", port))
        .arg(clone_dest.path())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    match child
        .wait_timeout(std::time::Duration::from_secs(1))
        .unwrap()
    {
        Some(status) => status.success(),
        None => panic!("git clone is hanging"),
    }
}

fn binary(name: &str) -> PathBuf {
    let mut binary_path = std::env::current_exe().unwrap();
    loop {
        if let Some(parent) = binary_path.parent() {
            if parent.is_dir() && parent.file_name().unwrap() == "target" {
                break;
            }
        } else {
            panic!("can't find the target directory");
        }
        binary_path.pop();
    }
    normalize_path(&binary_path.join(format!("{}{}", name, std::env::consts::EXE_SUFFIX)))
}

fn http_server(serve: PathBuf, auth: bool) -> u16 {
    let server = tiny_http::Server::http("localhost:0").unwrap();
    let port = server.server_addr().port();

    std::thread::spawn(move || loop {
        let rq = match server.recv() {
            Ok(rq) => rq,
            Err(_) => break,
        };

        let url = rq.url().split('?').next().unwrap()[1..].to_string();
        let file = std::fs::File::open(serve.join(url));

        if auth {
            let rep = tiny_http::Response::new_empty(tiny_http::StatusCode(401));
            let _ = rq.respond(rep.with_header(tiny_http::Header {
                field: "WWW-Authenticate".parse().unwrap(),
                value: "Basic realm=\"Dummy\"".parse().unwrap(),
            }));
        } else if file.is_ok() {
            let rep = tiny_http::Response::from_file(file.unwrap());
            let _ = rq.respond(rep);
        } else {
            let rep = tiny_http::Response::new_empty(tiny_http::StatusCode(404));
            let _ = rq.respond(rep);
        }
    });

    port
}

fn strip_verbatim_from_prefix(prefix: &PrefixComponent<'_>) -> Option<PathBuf> {
    Some(match prefix.kind() {
        Prefix::Verbatim(s) => Path::new(s).to_owned(),
        Prefix::VerbatimDisk(drive) => [format!(r"{}:\", drive as char)].iter().collect(),
        Prefix::VerbatimUNC(_, _) => unimplemented!(),
        _ => return None,
    })
}

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut p = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    // `fs::canonicalize` returns an extended-length path on Windows. Such paths not supported by
    // many programs, including rustup. We strip the `\\?\` prefix of the canonicalized path, but
    // this changes the meaning of some path components, and imposes a length of around 260
    // characters.
    if cfg!(windows) {
        const MAX_PATH_LEN: usize = 260 - 12;

        let mut components = p.components();
        let first_component = components.next().unwrap();

        if let Component::Prefix(prefix) = first_component {
            if let Some(mut modified_path) = strip_verbatim_from_prefix(&prefix) {
                modified_path.push(components.as_path());
                p = modified_path;
            }
        }

        if p.as_os_str().len() >= MAX_PATH_LEN {
            panic!(
                "Canonicalized path is too long for Windows: {:?}",
                p.as_os_str(),
            );
        }
    }
    p
}
