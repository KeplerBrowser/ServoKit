use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use servo::{
    ClipboardDelegate, EventLoopWaker, RenderingContext, SoftwareRenderingContext, StringRequest,
    WebView,
};
use servokit::{
    runtime::{ensure_default_rustls_crypto_provider, Runtime},
    surface::{
        HostSurface, MemoryClipboard, NativeSurface, SurfaceDelegate, SurfaceError, SurfaceFrame,
        SurfaceHost, SurfaceHostOptions, SurfaceSize, SurfaceTarget, SurfaceViewport,
    },
    webview::WebViewHandle,
    HostEvent,
};
use servokit_embedder::{PopupRequestPolicy, ServoRuntime, ServoWebViewInit};
use sha2::{Digest, Sha256};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Window, WindowId},
};

const COOKIE_NAME: &str = "servokit_profile";
const COOKIE_VALUE: &str = "opaque-test-session";
const CHILD_TIMEOUT: Duration = Duration::from_secs(60);
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(30);
const SURFACE_ID: &str = "profile-branch-proof-window";
const FIXTURE_HTML: &str = r#"<!doctype html>
<meta charset="utf-8">
<title>ServoKit profile branch proof</title>
<body>ServoKit profile branch proof</body>
<script>
(async () => {
  const params = new URLSearchParams(location.search);
  const phase = params.get("phase") || "";
  const branch = params.get("branch") || "";
  let error = "";

  let lsSeed = "";
  let lsBranch = "";
  try {
    if (phase === "seed") {
      localStorage.setItem("seed", "R1");
    } else if (phase === "branch") {
      localStorage.setItem("branch", branch);
    }
    lsSeed = localStorage.getItem("seed") || "";
    lsBranch = localStorage.getItem("branch") || "";
  } catch (cause) {
    error = String(cause && (cause.stack || cause.message || cause));
  }

  const report = new URLSearchParams({
    phase, branch, ls_seed: lsSeed, ls_branch: lsBranch, error
  });
  await fetch("/report?" + report.toString(), {cache: "no-store"});
  document.body.dataset.reported = "true";
})();
</script>
"#;

type ProofResult<T> = Result<T, Box<dyn Error>>;
type VisibleRuntime = Runtime<SurfaceHost<WinitSurface>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("profile branch proof failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> ProofResult<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("prove") => {
            reject_extra_args(args)?;
            prove()
        }
        Some("__visible") => run_visible_child(parse_child_args(args)?),
        Some("__headless") => run_headless_child(parse_child_args(args)?),
        Some(other) => Err(format!("unknown mode {other:?}; expected `prove`").into()),
        None => Err("missing mode; run this example with `-- prove`".into()),
    }
}

fn reject_extra_args(mut args: impl Iterator<Item = String>) -> ProofResult<()> {
    if let Some(argument) = args.next() {
        return Err(format!("unexpected argument {argument:?}").into());
    }
    Ok(())
}

#[derive(Debug)]
struct ChildArgs {
    profile: PathBuf,
    url: String,
    release: PathBuf,
}

fn parse_child_args(mut args: impl Iterator<Item = String>) -> ProofResult<ChildArgs> {
    let profile = PathBuf::from(args.next().ok_or("missing child profile directory")?);
    let url = args.next().ok_or("missing child fixture URL")?;
    let release = PathBuf::from(args.next().ok_or("missing child release path")?);
    reject_extra_args(args)?;
    Ok(ChildArgs {
        profile,
        url,
        release,
    })
}

fn prove() -> ProofResult<()> {
    let root = unique_proof_root();
    fs::create_dir_all(&root)?;
    let result = prove_in(&root);
    match result {
        Ok(()) => {
            fs::remove_dir_all(&root)?;
            Ok(())
        }
        Err(error) => Err(format!(
            "{error}; disposable proof artifacts were preserved at {}",
            root.display()
        )
        .into()),
    }
}

fn prove_in(root: &Path) -> ProofResult<()> {
    let (fixture, reports) = FixtureServer::start()?;
    let seed = root.join("seed");
    let revision = root.join("R1");
    let local = root.join("local-R1");
    let cloud = root.join("cloud-R1");
    for directory in [&seed, &local, &cloud] {
        fs::create_dir_all(directory)?;
    }

    println!("proof=servokit-cold-profile-branch");
    println!("platform={} arch={}", env::consts::OS, env::consts::ARCH);
    println!("servokit_revision={}", git_revision()?);
    println!(
        "servo_dependency=0.3.0 source=registry+https://github.com/rust-lang/crates.io-index checksum=586b1f633dabd1ceb0b1d92965f526dbb8c418fc277fccb79bd506170900f8ff lockfile=examples/desktop-winit/Cargo.lock"
    );

    let mut seed_children = vec![spawn_runner(
        "seed-visible",
        "__visible",
        &seed,
        &fixture.url("seed", "seed"),
        &root.join("release-seed"),
    )?];
    let seed_reports = wait_for_reports(
        &reports,
        &[ReportKey::new("seed", "seed")],
        &mut seed_children,
    )?;
    release_and_wait(&mut seed_children)?;
    assert_seed_report(&seed_reports[0])?;

    copy_tree(&seed, &revision)?;
    let revision_digest = digest_tree(&revision)?;
    copy_tree(&revision, &local)?;
    copy_tree(&revision, &cloud)?;
    let revision_canonical = fs::canonicalize(&revision)?;
    let local_canonical = fs::canonicalize(&local)?;
    let cloud_canonical = fs::canonicalize(&cloud)?;
    if revision_canonical == local_canonical
        || revision_canonical == cloud_canonical
        || local_canonical == cloud_canonical
    {
        return Err("revision and branch config directories must be distinct".into());
    }
    println!("revision_path={}", revision_canonical.display());
    println!("local_branch_path={}", local_canonical.display());
    println!("cloud_branch_path={}", cloud_canonical.display());
    println!("revision_digest_before={revision_digest}");

    let mut branch_children = vec![
        spawn_runner(
            "local-visible",
            "__visible",
            &local,
            &fixture.url("branch", "local"),
            &root.join("release-local-branch"),
        )?,
        spawn_runner(
            "cloud-headless",
            "__headless",
            &cloud,
            &fixture.url("branch", "cloud"),
            &root.join("release-cloud-branch"),
        )?,
    ];
    let branch_reports = wait_for_reports(
        &reports,
        &[
            ReportKey::new("branch", "local"),
            ReportKey::new("branch", "cloud"),
        ],
        &mut branch_children,
    )?;
    assert_branch_report(find_report(&branch_reports, "branch", "local")?, "local")?;
    assert_branch_report(find_report(&branch_reports, "branch", "cloud")?, "cloud")?;
    println!("concurrent_authenticated=true");
    release_and_wait(&mut branch_children)?;
    assert_revision_unchanged(&revision, &revision_digest)?;

    let mut verify_children = vec![
        spawn_runner(
            "local-reopen-headless",
            "__headless",
            &local,
            &fixture.url("verify", "local"),
            &root.join("release-local-verify"),
        )?,
        spawn_runner(
            "cloud-reopen-headless",
            "__headless",
            &cloud,
            &fixture.url("verify", "cloud"),
            &root.join("release-cloud-verify"),
        )?,
    ];
    let verify_reports = wait_for_reports(
        &reports,
        &[
            ReportKey::new("verify", "local"),
            ReportKey::new("verify", "cloud"),
        ],
        &mut verify_children,
    )?;
    release_and_wait(&mut verify_children)?;
    assert_branch_report(find_report(&verify_reports, "verify", "local")?, "local")?;
    assert_branch_report(find_report(&verify_reports, "verify", "cloud")?, "cloud")?;
    assert_revision_unchanged(&revision, &revision_digest)?;

    println!("revision_digest_after={revision_digest}");
    println!("fresh_process_reopen=true");
    println!("result=PASS");
    drop(fixture);
    Ok(())
}

fn unique_proof_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!(
        "servokit-profile-branch-proof-{}-{nanos}",
        std::process::id()
    ))
}

fn git_revision() -> ProofResult<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !output.status.success() {
        return Err("failed to resolve the ServoKit Git revision".into());
    }
    let revision = String::from_utf8(output.stdout)?.trim().to_owned();
    if revision.is_empty() {
        return Err("ServoKit Git revision was empty".into());
    }
    Ok(revision)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportKey {
    phase: String,
    branch: String,
}

impl ReportKey {
    fn new(phase: &str, branch: &str) -> Self {
        Self {
            phase: phase.to_owned(),
            branch: branch.to_owned(),
        }
    }
}

#[derive(Debug)]
struct Report {
    phase: String,
    branch: String,
    authenticated: bool,
    ls_seed: String,
    ls_branch: String,
    error: String,
}

impl Report {
    fn key(&self) -> ReportKey {
        ReportKey::new(&self.phase, &self.branch)
    }
}

fn assert_seed_report(report: &Report) -> ProofResult<()> {
    assert_report_ok(report)?;
    if report.ls_seed != "R1" || !report.ls_branch.is_empty() {
        return Err(format!("seed state did not match R1: {report:?}").into());
    }
    println!("seed_state=R1 authentication=true");
    Ok(())
}

fn assert_branch_report(report: &Report, expected_branch: &str) -> ProofResult<()> {
    assert_report_ok(report)?;
    if report.ls_seed != "R1" || report.ls_branch != expected_branch {
        return Err(format!(
            "{} state did not preserve R1 and diverge independently: {report:?}",
            report.key().branch
        )
        .into());
    }
    println!(
        "phase={} branch={} authentication=true local_storage_seed=R1 local_storage_branch={}",
        report.phase, report.branch, report.ls_branch
    );
    Ok(())
}

fn assert_report_ok(report: &Report) -> ProofResult<()> {
    if !report.error.is_empty() {
        return Err(format!(
            "fixture script failed for {:?}: {}",
            report.key(),
            report.error
        )
        .into());
    }
    if !report.authenticated {
        return Err(format!(
            "fixture did not observe authentication for {:?}",
            report.key()
        )
        .into());
    }
    Ok(())
}

fn find_report<'a>(reports: &'a [Report], phase: &str, branch: &str) -> ProofResult<&'a Report> {
    reports
        .iter()
        .find(|report| report.phase == phase && report.branch == branch)
        .ok_or_else(|| format!("missing {phase}/{branch} report").into())
}

fn assert_revision_unchanged(revision: &Path, expected_digest: &str) -> ProofResult<()> {
    let actual = digest_tree(revision)?;
    if actual != expected_digest {
        return Err(format!(
            "immutable revision changed: expected {expected_digest}, got {actual}"
        )
        .into());
    }
    Ok(())
}

struct ChildProcess {
    label: String,
    release: PathBuf,
    child: Option<Child>,
}

impl ChildProcess {
    fn ensure_running(&mut self) -> ProofResult<()> {
        let child = self.child.as_mut().ok_or("child was already reaped")?;
        if let Some(status) = child.try_wait()? {
            self.child = None;
            return Err(format!("{} exited before release with {status}", self.label).into());
        }
        Ok(())
    }

    fn release_and_wait(&mut self) -> ProofResult<ExitStatus> {
        fs::write(&self.release, b"release\n")?;
        let deadline = Instant::now() + PROCESS_EXIT_TIMEOUT;
        loop {
            let child = self.child.as_mut().ok_or("child was already reaped")?;
            if let Some(status) = child.try_wait()? {
                self.child = None;
                if !status.success() {
                    return Err(format!("{} exited with {status}", self.label).into());
                }
                println!("child={} clean_exit=true", self.label);
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(format!("timed out waiting for {} to exit", self.label).into());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

fn spawn_runner(
    label: &str,
    mode: &str,
    profile: &Path,
    url: &str,
    release: &Path,
) -> ProofResult<ChildProcess> {
    let executable = env::current_exe()?;
    let child = Command::new(executable)
        .arg(mode)
        .arg(profile)
        .arg(url)
        .arg(release)
        .spawn()?;
    Ok(ChildProcess {
        label: label.to_owned(),
        release: release.to_owned(),
        child: Some(child),
    })
}

fn wait_for_reports(
    receiver: &Receiver<Report>,
    expected: &[ReportKey],
    children: &mut [ChildProcess],
) -> ProofResult<Vec<Report>> {
    let deadline = Instant::now() + CHILD_TIMEOUT;
    let mut reports = Vec::new();
    while reports.len() < expected.len() {
        for child in children.iter_mut() {
            child.ensure_running()?;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("timed out waiting for reports {expected:?}").into());
        }
        match receiver.recv_timeout(remaining.min(Duration::from_millis(100))) {
            Ok(report) => {
                let key = report.key();
                if !expected.contains(&key) {
                    return Err(format!("received unexpected fixture report {key:?}").into());
                }
                if reports
                    .iter()
                    .any(|existing: &Report| existing.key() == key)
                {
                    return Err(format!("received duplicate fixture report {key:?}").into());
                }
                reports.push(report);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err("fixture server stopped before sending reports".into())
            }
        }
    }
    for child in children.iter_mut() {
        child.ensure_running()?;
    }
    Ok(reports)
}

fn release_and_wait(children: &mut [ChildProcess]) -> ProofResult<()> {
    for child in children {
        child.release_and_wait()?;
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> ProofResult<()> {
    if destination.exists() {
        if destination.read_dir()?.next().is_some() {
            return Err(format!("copy destination is not empty: {}", destination.display()).into());
        }
    } else {
        fs::create_dir_all(destination)?;
    }
    copy_tree_contents(source, destination)
}

fn copy_tree_contents(source: &Path, destination: &Path) -> ProofResult<()> {
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir(&target)?;
            copy_tree_contents(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(format!(
                "profile contains unsupported non-file entry: {}",
                entry.path().display()
            )
            .into());
        }
    }
    Ok(())
}

fn digest_tree(root: &Path) -> ProofResult<String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for relative in files {
        let bytes = fs::read(root.join(&relative))?;
        hasher.update(b"path\0");
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(b"\0content\0");
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> ProofResult<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_files(root, &entry.path(), files)?;
        } else if file_type.is_file() {
            files.push(entry.path().strip_prefix(root)?.to_owned());
        } else {
            return Err(format!(
                "profile contains unsupported non-file entry: {}",
                entry.path().display()
            )
            .into());
        }
    }
    Ok(())
}

struct FixtureServer {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FixtureServer {
    fn start() -> ProofResult<(Self, Receiver<Report>)> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Err(error) = handle_fixture_request(stream, &sender) {
                            eprintln!("fixture request failed: {error}");
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => {
                        eprintln!("fixture server failed: {error}");
                        break;
                    }
                }
            }
        });
        Ok((
            Self {
                address,
                stop,
                thread: Some(thread),
            },
            receiver,
        ))
    }

    fn url(&self, phase: &str, branch: &str) -> String {
        format!(
            "http://{}/profile?phase={phase}&branch={branch}",
            self.address
        )
    }
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle_fixture_request(mut stream: TcpStream, sender: &mpsc::Sender<Report>) -> ProofResult<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    while request.len() < 32 * 1024 {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let request = String::from_utf8(request)?;
    let mut lines = request.split("\r\n");
    let request_line = lines.next().ok_or("missing HTTP request line")?;
    let mut request_parts = request_line.split_whitespace();
    if request_parts.next() != Some("GET") {
        return write_response(&mut stream, "405 Method Not Allowed", &[], b"");
    }
    let target = request_parts.next().ok_or("missing HTTP target")?;
    let cookie = lines.find_map(|line| {
        line.split_once(':').and_then(|(name, value)| {
            name.eq_ignore_ascii_case("cookie")
                .then(|| value.trim().to_owned())
        })
    });
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    match path {
        "/profile" => {
            let fields = parse_query(query)?;
            let mut headers = vec![("Content-Type", "text/html; charset=utf-8")];
            if fields.get("phase").map(String::as_str) == Some("seed") {
                headers.push((
                    "Set-Cookie",
                    "servokit_profile=opaque-test-session; HttpOnly; Path=/; SameSite=Lax",
                ));
            }
            write_response(&mut stream, "200 OK", &headers, FIXTURE_HTML.as_bytes())
        }
        "/report" => {
            let fields = parse_query(query)?;
            let report = Report {
                phase: required_field(&fields, "phase")?,
                branch: required_field(&fields, "branch")?,
                authenticated: cookie.as_deref().is_some_and(cookie_has_test_session),
                ls_seed: required_field(&fields, "ls_seed")?,
                ls_branch: required_field(&fields, "ls_branch")?,
                error: required_field(&fields, "error")?,
            };
            sender.send(report)?;
            write_response(&mut stream, "204 No Content", &[], b"")
        }
        "/favicon.ico" => write_response(&mut stream, "204 No Content", &[], b""),
        _ => write_response(&mut stream, "404 Not Found", &[], b"not found"),
    }
}

fn cookie_has_test_session(cookie: &str) -> bool {
    cookie.split(';').any(|pair| {
        pair.trim()
            .split_once('=')
            .is_some_and(|(name, value)| name == COOKIE_NAME && value == COOKIE_VALUE)
    })
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> ProofResult<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    )?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

fn parse_query(query: &str) -> ProofResult<BTreeMap<String, String>> {
    let mut fields = BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        fields.insert(percent_decode(name)?, percent_decode(value)?);
    }
    Ok(fields)
}

fn required_field(fields: &BTreeMap<String, String>, name: &str) -> ProofResult<String> {
    fields
        .get(name)
        .cloned()
        .ok_or_else(|| format!("missing report field {name}").into())
}

fn percent_decode(value: &str) -> ProofResult<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let high = hex_digit(bytes[index + 1])?;
                let low = hex_digit(bytes[index + 2])?;
                decoded.push(high * 16 + low);
                index += 3;
            }
            b'%' => return Err("truncated percent-encoded query value".into()),
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    Ok(String::from_utf8(decoded)?)
}

fn hex_digit(value: u8) -> ProofResult<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(format!("invalid percent-encoded hex digit {value:?}").into()),
    }
}

#[derive(Clone)]
struct PollWaker(Arc<AtomicBool>);

impl EventLoopWaker for PollWaker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }

    fn wake(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct ProofClipboard;

impl ClipboardDelegate for ProofClipboard {
    fn get_text(&self, _webview: WebView, request: StringRequest) {
        request.success(String::new());
    }
}

fn run_headless_child(args: ChildArgs) -> ProofResult<()> {
    let _ = ensure_default_rustls_crypto_provider();
    let rendering_context: Rc<dyn RenderingContext> = Rc::new(
        SoftwareRenderingContext::new(PhysicalSize::new(960, 640))
            .map_err(|error| format!("failed to create software rendering context: {error:?}"))?,
    );
    rendering_context
        .make_current()
        .map_err(|error| format!("failed to activate software rendering context: {error:?}"))?;
    let runtime = ServoRuntime::with_config_directory(
        Box::new(PollWaker(Arc::new(AtomicBool::new(false)))),
        &args.profile,
    )?;
    let mut webview = runtime.create_webview(ServoWebViewInit {
        rendering_context,
        clipboard_delegate: Rc::new(ProofClipboard),
        initial_url: Some(args.url),
        density: 1.0,
        popup_policy: PopupRequestPolicy::DefaultDeny,
        managed_child_rendering_context_factory: None,
    })?;
    let deadline = Instant::now() + CHILD_TIMEOUT;
    while !args.release.exists() {
        let events = webview.perform_updates(true, || {}, || {})?;
        handle_headless_events(&mut webview, events)?;
        if Instant::now() >= deadline {
            return Err("headless child timed out before release".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
    drop(webview);
    runtime.shutdown()?;
    Ok(())
}

fn handle_headless_events(
    webview: &mut servokit_embedder::ServoWebView,
    events: Vec<HostEvent>,
) -> ProofResult<()> {
    for event in events {
        match event {
            HostEvent::NavigationRequested { navigation_id, .. } => {
                webview.resolve_navigation_request(&navigation_id, true)?;
            }
            HostEvent::Crashed { reason, .. } => {
                return Err(format!("headless Servo crashed: {reason}").into())
            }
            HostEvent::Error { code, message, .. } => {
                return Err(format!("headless Servo error {code}: {message}").into())
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum AppEvent {
    Wake,
}

struct WinitSurface {
    window: Rc<Window>,
}

impl SurfaceDelegate for WinitSurface {
    type Frame<'a> = SurfaceFrame<'a>;

    fn render_target(
        &mut self,
        _surface: &HostSurface,
        _viewport: SurfaceViewport,
    ) -> Result<SurfaceTarget<'_>, SurfaceError> {
        let display = self
            .window
            .display_handle()
            .map_err(|error| SurfaceError::new(error.to_string()))?;
        let window = self
            .window
            .window_handle()
            .map_err(|error| SurfaceError::new(error.to_string()))?;
        Ok(NativeSurface::new(display, window).into())
    }
}

struct VisibleProofApp {
    args: Option<ChildArgs>,
    options: SurfaceHostOptions,
    runtime: Option<VisibleRuntime>,
    window: Option<Rc<Window>>,
    webview: Option<WebViewHandle>,
    deadline: Instant,
    result: Option<Result<(), String>>,
}

impl VisibleProofApp {
    fn new(args: ChildArgs, options: SurfaceHostOptions) -> Self {
        Self {
            args: Some(args),
            options,
            runtime: None,
            window: None,
            webview: None,
            deadline: Instant::now() + CHILD_TIMEOUT,
            result: None,
        }
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) -> ProofResult<()> {
        if self.window.is_some() {
            return Ok(());
        }
        let args = self
            .args
            .as_ref()
            .ok_or("visible child arguments missing")?;
        let window = Rc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("ServoKit local profile branch proof")
                    .with_inner_size(LogicalSize::new(960.0, 640.0)),
            )?,
        );
        let host = SurfaceHost::new(
            WinitSurface {
                window: window.clone(),
            },
            self.options.clone(),
        );
        let mut runtime = Runtime::new(host);
        let session = runtime.create_session();
        let webview = runtime.create_webview(session)?;
        runtime.load_url(webview, &args.url)?;
        runtime.attach_surface_with_viewport(
            webview,
            HostSurface::new(SURFACE_ID),
            viewport(&window),
        )?;
        runtime.perform_updates(webview)?;
        self.runtime = Some(runtime);
        self.window = Some(window);
        self.webview = Some(webview);
        self.drain_events()?;
        Ok(())
    }

    fn pump(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        if self.result.is_some() {
            return;
        }
        if self.args.as_ref().is_some_and(|args| args.release.exists()) {
            self.finish(event_loop, Ok(()));
            return;
        }
        if Instant::now() >= self.deadline {
            self.finish(
                event_loop,
                Err("visible child timed out before release".to_owned()),
            );
            return;
        }
        let (Some(runtime), Some(webview)) = (self.runtime.as_mut(), self.webview) else {
            return;
        };
        if let Err(error) = runtime.perform_updates(webview) {
            self.finish(
                event_loop,
                Err(format!("visible Servo update failed: {error}")),
            );
            return;
        }
        if let Err(error) = self.drain_events() {
            self.finish(event_loop, Err(error.to_string()));
        }
    }

    fn drain_events(&mut self) -> ProofResult<()> {
        loop {
            let events = self
                .runtime
                .as_mut()
                .map(Runtime::drain_events)
                .unwrap_or_default();
            if events.is_empty() {
                return Ok(());
            }
            for event in events {
                match event.event {
                    HostEvent::NavigationRequested { navigation_id, .. }
                        if event.managed_child_webview_id.is_none() =>
                    {
                        self.runtime
                            .as_mut()
                            .ok_or("visible runtime missing")?
                            .resolve_navigation_request(event.webview, &navigation_id, true)?;
                    }
                    HostEvent::Crashed { reason, .. } => {
                        return Err(format!("visible Servo crashed: {reason}").into())
                    }
                    HostEvent::Error { code, message, .. } => {
                        return Err(format!("visible Servo error {code}: {message}").into())
                    }
                    _ => {}
                }
            }
        }
    }

    fn resize(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(runtime), Some(window), Some(webview)) =
            (self.runtime.as_mut(), self.window.as_ref(), self.webview)
        else {
            return;
        };
        if let Err(error) = runtime.update_surface_viewport(webview, viewport(window)) {
            self.finish(
                event_loop,
                Err(format!("visible viewport update failed: {error}")),
            );
        }
    }

    fn finish(&mut self, event_loop: &ActiveEventLoop, mut result: Result<(), String>) {
        if self.result.is_some() {
            return;
        }
        self.webview = None;
        if let Some(runtime) = self.runtime.take() {
            if let Err(error) = runtime.shutdown() {
                result = Err(format!("visible Servo shutdown failed: {error}"));
            }
        }
        self.result = Some(result);
        event_loop.exit();
    }
}

impl ApplicationHandler<AppEvent> for VisibleProofApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.start(event_loop) {
            self.finish(event_loop, Err(error.to_string()));
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: AppEvent) {
        self.pump(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => self.finish(
                event_loop,
                Err("visible proof window closed before release".to_owned()),
            ),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.resize(event_loop)
            }
            WindowEvent::RedrawRequested => self.pump(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.pump(event_loop);
    }
}

fn run_visible_child(args: ChildArgs) -> ProofResult<()> {
    let _ = ensure_default_rustls_crypto_provider();
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = Arc::new(Mutex::new(event_loop.create_proxy()));
    let waker = Arc::new(move || {
        if let Ok(proxy) = proxy.lock() {
            let _ = proxy.send_event(AppEvent::Wake);
        }
    });
    let options = SurfaceHostOptions::new(waker, Rc::new(MemoryClipboard::default()))
        .with_config_directory(&args.profile);
    let mut app = VisibleProofApp::new(args, options);
    event_loop.run_app(&mut app)?;
    match app.result.take() {
        Some(Ok(())) => Ok(()),
        Some(Err(error)) => Err(error.into()),
        None => Err("visible proof event loop exited without a result".into()),
    }
}

fn viewport(window: &Window) -> SurfaceViewport {
    let size = window.inner_size();
    SurfaceViewport::new(
        SurfaceSize::new(size.width, size.height),
        window.scale_factor() as f32,
    )
}
