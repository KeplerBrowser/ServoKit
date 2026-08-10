use std::time::{Duration, Instant};

use servokit::events::{HostEvent, LoadStatusKind, ServokitEvent};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
pub struct SmokeOptions {
    pub timeout: Duration,
}

impl Default for SmokeOptions {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SmokeOutcome {
    Passed,
    Failed(String),
}

impl SmokeOutcome {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Passed => 0,
            Self::Failed(_) => 1,
        }
    }
}

#[derive(Debug)]
pub struct SmokeState {
    started_at: Instant,
    timeout: Duration,
    target_url: String,
    surface_attached: Option<(u32, u32)>,
    surface_resized: Option<(u32, u32)>,
    last_url: Option<String>,
    last_load: Option<LoadStatusKind>,
    load_completed: bool,
    failures: Vec<String>,
}

impl SmokeState {
    pub fn new(target_url: String, options: SmokeOptions) -> Self {
        println!(
            "smoke mode=enabled platform={}/{} url={} timeout_ms={}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            target_url,
            options.timeout.as_millis()
        );
        Self {
            started_at: Instant::now(),
            timeout: options.timeout,
            target_url,
            surface_attached: None,
            surface_resized: None,
            last_url: None,
            last_load: None,
            load_completed: false,
            failures: Vec::new(),
        }
    }

    pub fn observe(&mut self, event: &ServokitEvent) {
        match &event.event {
            HostEvent::SurfaceAttached { size } => {
                self.surface_attached = Some((size.width, size.height));
            }
            HostEvent::SurfaceResized { size } => {
                self.surface_resized = Some((size.width, size.height));
            }
            HostEvent::UrlChanged { url } => {
                self.last_url = Some(url.clone());
            }
            HostEvent::LoadStatusChanged { status } => {
                self.last_load = Some(*status);
                if *status == LoadStatusKind::Complete {
                    self.load_completed = true;
                }
            }
            HostEvent::Error { url, code, message } => {
                self.failures.push(format!(
                    "error url={} code={} message={}",
                    optional_string(url),
                    code,
                    display_value(message)
                ));
            }
            HostEvent::Crashed { url, reason, .. } => {
                self.failures.push(format!(
                    "crash url={} reason={}",
                    optional_string(url),
                    display_value(reason)
                ));
            }
            _ => {}
        }
    }

    pub fn record_failure(&mut self, reason: impl Into<String>) {
        self.failures.push(reason.into());
    }

    pub fn outcome(&self) -> Option<SmokeOutcome> {
        if let Some(failure) = self.failures.first() {
            return Some(SmokeOutcome::Failed(failure.clone()));
        }
        if self.load_completed {
            return Some(SmokeOutcome::Passed);
        }
        if self.started_at.elapsed() >= self.timeout {
            return Some(SmokeOutcome::Failed(format!(
                "timed out after {}ms waiting for load completion",
                self.timeout.as_millis()
            )));
        }
        None
    }

    pub fn print_result(&self, outcome: &SmokeOutcome) {
        let (result, reason) = match outcome {
            SmokeOutcome::Passed => ("pass", "none".to_owned()),
            SmokeOutcome::Failed(reason) => ("fail", display_value(reason)),
        };
        let load_status = self
            .last_load
            .as_ref()
            .map(|status| status.as_str())
            .unwrap_or("none");
        println!(
            "smoke result={} platform={}/{} target_url={} last_url={} load_status={} surface_attached={} surface_resized={} elapsed_ms={} errors={} reason={}",
            result,
            std::env::consts::OS,
            std::env::consts::ARCH,
            display_value(&self.target_url),
            optional_string(&self.last_url),
            load_status,
            optional_size(self.surface_attached),
            optional_size(self.surface_resized),
            self.started_at.elapsed().as_millis(),
            self.failures.len(),
            reason,
        );
        for (index, failure) in self.failures.iter().enumerate() {
            println!("smoke error[{}]={}", index + 1, display_value(failure));
        }
    }
}

fn optional_string(value: &Option<String>) -> String {
    value
        .as_ref()
        .map(|value| display_value(value))
        .unwrap_or_else(|| "none".to_owned())
}

fn optional_size(size: Option<(u32, u32)>) -> String {
    size.map(|(width, height)| format!("{}x{}", width, height))
        .unwrap_or_else(|| "none".to_owned())
}

fn display_value(value: &str) -> String {
    if value.is_empty() {
        "<empty>".to_owned()
    } else {
        value.replace('\n', "\\n")
    }
}
