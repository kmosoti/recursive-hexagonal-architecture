//! Clippy corpus source (ADR-0002): one use of every entry of the core
//! template, so each entry is shown to resolve and fire under the pinned
//! Clippy. Experiment 4 fails if any entry does not appear in the output,
//! which is how a misspelled or renamed path is caught.

use std::io::{IsTerminal as _, Read as _};
use std::net::ToSocketAddrs as _;
use std::path::Path;

/// `SystemTime::now`, `SystemTime::elapsed`, `Instant::now`, `Instant::elapsed`.
#[must_use]
pub fn clocks() -> (std::time::Duration, std::time::Duration) {
    let wall = std::time::SystemTime::now();
    let mono = std::time::Instant::now();
    (
        wall.elapsed().unwrap_or_default(),
        mono.elapsed(),
    )
}

/// `env::var`, `var_os`, `vars`, `vars_os`, `args`, `args_os`.
#[must_use]
pub fn environment() -> usize {
    let named = std::env::var("HOME").is_ok();
    let named_os = std::env::var_os("HOME").is_some();
    let all = std::env::vars().count() + std::env::vars_os().count();
    let argv = std::env::args().count() + std::env::args_os().count();
    all + argv + usize::from(named) + usize::from(named_os)
}

/// `env::set_var`, `env::remove_var`: both are `unsafe` in edition 2024, so
/// the fixture names them without calling them.
#[must_use]
pub fn environment_writes() -> usize {
    let set = std::env::set_var::<&str, &str>;
    let remove = std::env::remove_var::<&str>;
    core::mem::size_of_val(&set) + core::mem::size_of_val(&remove)
}

/// `env::current_dir`, `set_current_dir`, `current_exe`, `temp_dir`,
/// `home_dir`, `path::absolute`, `thread::available_parallelism`.
#[must_use]
pub fn process_environment() -> usize {
    let here = std::env::current_dir().unwrap_or_default();
    let back = std::env::set_current_dir(&here).is_ok();
    let exe = std::env::current_exe().unwrap_or_default();
    let tmp = std::env::temp_dir();
    let home = std::env::home_dir().unwrap_or_default();
    let absolute = std::path::absolute("a").unwrap_or_default();
    let cpus = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    cpus + usize::from(back)
        + exe.as_os_str().len()
        + tmp.as_os_str().len()
        + home.as_os_str().len()
        + absolute.as_os_str().len()
}

/// `io::IsTerminal::is_terminal`, `io::stdin`, `io::stdout`, `io::stderr`,
/// `io::pipe`, and the types `io::Stdin`-reaching functions above.
///
/// # Errors
/// Any I/O error.
pub fn streams() -> std::io::Result<String> {
    let interactive = std::io::stdin().is_terminal();
    let _ = std::io::stdout();
    let _ = std::io::stderr();
    let (mut reader, _writer) = std::io::pipe()?;
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    text.push(if interactive { 'i' } else { 'b' });
    Ok(text)
}

/// The std output macros: `print`, `println`, `eprint`, `eprintln`, `dbg`.
pub fn output() {
    print!("out");
    println!("out");
    eprint!("err");
    eprintln!("err");
    let _ = dbg!(1);
}

/// `fs::read`, `read_to_string`, `write`, `copy`, `rename`, `canonicalize`.
///
/// # Errors
/// Any I/O error.
pub fn storage_bytes() -> std::io::Result<usize> {
    let bytes = std::fs::read("a")?;
    let text = std::fs::read_to_string("b")?;
    std::fs::write("c", [bytes.as_slice(), text.as_bytes()].concat())?;
    let copied = std::fs::copy("a", "d")?;
    std::fs::rename("d", "e")?;
    let real = std::fs::canonicalize("e")?;
    Ok(usize::try_from(copied).unwrap_or(0) + real.as_os_str().len())
}

/// `fs::metadata`, `symlink_metadata`, `exists`, `read_link`, `set_permissions`.
///
/// # Errors
/// Any I/O error.
pub fn storage_metadata() -> std::io::Result<u64> {
    let meta = std::fs::metadata("a")?;
    let link_meta = std::fs::symlink_metadata("a")?;
    let there = std::fs::exists("a")?;
    let target = std::fs::read_link("a")?;
    std::fs::set_permissions("a", meta.permissions())?;
    Ok(meta.len() + link_meta.len() + u64::from(there) + target.as_os_str().len() as u64)
}

/// `fs::create_dir`, `create_dir_all`, `remove_dir`, `remove_dir_all`,
/// `remove_file`, `hard_link`, `soft_link`, `read_dir`.
///
/// # Errors
/// Any I/O error.
pub fn storage_tree() -> std::io::Result<usize> {
    std::fs::create_dir("x")?;
    std::fs::create_dir_all("x/y")?;
    std::fs::hard_link("a", "x/link")?;
    #[allow(deprecated)]
    std::fs::soft_link("a", "x/soft")?;
    let entries = std::fs::read_dir("x")?.count();
    std::fs::remove_file("x/link")?;
    std::fs::remove_dir("x/y")?;
    std::fs::remove_dir_all("x")?;
    Ok(entries)
}

/// The storage types: `fs::File`, `OpenOptions`, `DirBuilder`, `ReadDir`.
///
/// # Errors
/// Any I/O error.
pub fn storage_types(listing: std::fs::ReadDir) -> std::io::Result<usize> {
    let mut file = std::fs::File::open("a")?;
    let opened = std::fs::OpenOptions::new().read(true).open("b")?;
    std::fs::DirBuilder::new().recursive(true).create("x")?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text.len() + listing.count() + usize::try_from(opened.metadata()?.len()).unwrap_or(0))
}

/// The `Path` methods that touch the filesystem.
///
/// # Errors
/// Any I/O error.
pub fn path_queries(path: &Path) -> std::io::Result<usize> {
    let flags = usize::from(path.exists())
        + usize::from(path.try_exists()?)
        + usize::from(path.is_file())
        + usize::from(path.is_dir())
        + usize::from(path.is_symlink());
    let meta = path.metadata()?;
    let link_meta = path.symlink_metadata()?;
    let real = path.canonicalize()?;
    let entries = path.read_dir()?.count();
    let target = path.read_link()?;
    Ok(flags
        + entries
        + real.as_os_str().len()
        + target.as_os_str().len()
        + usize::try_from(meta.len() + link_meta.len()).unwrap_or(0))
}

/// The network types and `ToSocketAddrs::to_socket_addrs`.
///
/// # Errors
/// Any I/O error.
pub fn network() -> std::io::Result<usize> {
    let resolved = "127.0.0.1:1".to_socket_addrs()?.count();
    let stream = std::net::TcpStream::connect("127.0.0.1:1")?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let socket = std::net::UdpSocket::bind("127.0.0.1:0")?;
    Ok(resolved
        + stream.peer_addr()?.port() as usize
        + listener.local_addr()?.port() as usize
        + socket.local_addr()?.port() as usize)
}

/// `process::Command`, `process::Child`, `exit`, `abort`, `id`.
///
/// # Errors
/// Any I/O error.
pub fn processes(mut running: std::process::Child) -> std::io::Result<u32> {
    let child = std::process::Command::new("true").spawn()?;
    running.kill()?;
    let exit: fn(i32) -> ! = std::process::exit;
    let abort: fn() -> ! = std::process::abort;
    Ok(std::process::id() + child.id() + exit as u32 + abort as u32)
}

/// `thread::spawn`, `Builder`, `scope`, `Scope::spawn`, `sleep`, `park`,
/// `park_timeout`, `yield_now`.
pub fn threads() {
    let _ = std::thread::spawn(|| ()).join();
    let _ = std::thread::Builder::new().name("t".to_owned());
    std::thread::scope(|scope| {
        scope.spawn(|| ());
    });
    std::thread::sleep(std::time::Duration::from_millis(1));
    std::thread::park();
    std::thread::park_timeout(std::time::Duration::from_millis(1));
    std::thread::yield_now();
}

/// `hash::RandomState`, the seeded randomness behind `HashMap`.
#[must_use]
pub fn randomness() -> std::hash::RandomState {
    std::hash::RandomState::new()
}
