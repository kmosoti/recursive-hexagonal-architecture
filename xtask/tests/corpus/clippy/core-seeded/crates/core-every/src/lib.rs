//! Fixture: one use of every entry of the core deny list, so each entry is
//! shown to resolve and fire under the pinned Clippy.

use std::io::Read as _;

/// `std::time::SystemTime::now`, `std::time::Instant::now`.
#[must_use]
pub fn clock() -> (std::time::SystemTime, std::time::Instant) {
    (std::time::SystemTime::now(), std::time::Instant::now())
}

/// `std::env::var`, `std::env::vars`.
#[must_use]
pub fn environment() -> (Option<String>, usize) {
    (std::env::var("HOME").ok(), std::env::vars().count())
}

/// `std::process::Command::new`.
#[must_use]
pub fn process() -> std::process::Command {
    std::process::Command::new("true")
}

/// `std::thread::spawn`.
pub fn thread() {
    let _ = std::thread::spawn(|| ()).join();
}

/// `std::fs::read`, `std::fs::read_to_string`, `std::fs::write`, and the type `std::fs::File`.
///
/// # Errors
/// Any I/O error.
pub fn storage() -> std::io::Result<String> {
    let bytes = std::fs::read("a")?;
    let text = std::fs::read_to_string("b")?;
    std::fs::write("c", [bytes.as_slice(), text.as_bytes()].concat())?;
    let mut file = std::fs::File::open("d")?;
    let mut out = String::new();
    file.read_to_string(&mut out)?;
    Ok(out)
}

/// The type `std::net::TcpStream`.
///
/// # Errors
/// Any I/O error.
pub fn network() -> std::io::Result<std::net::TcpStream> {
    std::net::TcpStream::connect("127.0.0.1:1")
}

/// `std::io::stdin`.
#[must_use]
pub fn input() -> std::io::Stdin {
    std::io::stdin()
}

/// The macros `std::println`, `std::eprintln`, `std::dbg`.
pub fn output() {
    println!("out");
    eprintln!("err");
    let _ = dbg!(1);
}
