use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const CHILD_ENV: &str = "RHA_EXE_IDENTITY_CHILD";
const CHILD_VALUE: &str = "1";
const DIGEST_PREFIX: &str = "RHA_EXE_IDENTITY_SHA256=";

struct ScratchDir(PathBuf);

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn unique_scratch_dir(repo: &Path) -> PathBuf {
    let base = repo.join("target/rha/exe-identity-unique");
    std::fs::create_dir_all(&base).expect("identity scratch root");
    let mut attempt = 0_u64;
    loop {
        let path = base.join(format!("{}-{attempt}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                attempt = attempt.checked_add(1).expect("identity scratch attempts");
            }
            Err(error) => panic!("identity scratch directory: {error}"),
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn executable_identity_survives_unlink_and_replacement() {
    if std::env::var(CHILD_ENV).ok().as_deref() == Some(CHILD_VALUE) {
        let mut barrier = [0_u8; 1];
        std::io::stdin()
            .read_exact(&mut barrier)
            .expect("identity barrier");
        let digest = xtask::util::running_executable_sha256().expect("running executable hash");
        println!("{DIGEST_PREFIX}{digest}");
        return;
    }

    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent");
    let scratch = ScratchDir(unique_scratch_dir(repo));
    let source = std::env::current_exe().expect("current integration test executable");
    let copied = scratch.0.join("executable_identity");
    std::fs::copy(&source, &copied).expect("copy integration test executable");
    let expected = xtask::util::sha256_file(&copied).expect("hash copied executable");

    let mut child = Command::new(&copied)
        .args([
            "--exact",
            "executable_identity_survives_unlink_and_replacement",
            "--nocapture",
        ])
        .env(CHILD_ENV, CHILD_VALUE)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .current_dir(repo)
        .spawn()
        .expect("spawn copied integration test executable");

    std::fs::remove_file(&copied).expect("unlink copied executable");
    std::fs::write(
        &copied,
        b"replacement file must not supply the running identity",
    )
    .expect("replace executable pathname");
    let mut stdin = child.stdin.take().expect("child stdin pipe");
    stdin.write_all(&[1]).expect("write identity barrier");
    drop(stdin);

    let output = child
        .wait_with_output()
        .expect("wait for copied integration test executable");
    assert!(
        output.status.success(),
        "copied integration test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let digest = stdout
        .lines()
        .find_map(|line| line.strip_prefix(DIGEST_PREFIX))
        .expect("child executable digest");
    assert_eq!(digest, expected);
}
