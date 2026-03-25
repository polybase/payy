use std::time::Duration;

pub struct StepResult {
    name: &'static str,
    state: StepState,
    extra_output: Vec<String>,
    duration: Duration,
}

impl StepResult {
    pub fn is_failure(&self) -> bool {
        matches!(self.state, StepState::Failed { .. })
    }

    pub fn success(name: &'static str, detail: String, duration: Duration) -> Self {
        StepResult {
            name,
            state: StepState::Success { detail },
            extra_output: Vec::new(),
            duration,
        }
    }

    pub fn skipped(name: &'static str, detail: String, duration: Duration) -> Self {
        StepResult {
            name,
            state: StepState::Skipped { detail },
            extra_output: Vec::new(),
            duration,
        }
    }

    pub fn failed(name: &'static str, error: String, duration: Duration) -> Self {
        StepResult {
            name,
            state: StepState::Failed { error },
            extra_output: Vec::new(),
            duration,
        }
    }

    pub fn with_extra_output(mut self, output: Vec<String>) -> Self {
        self.extra_output = output;
        self
    }

    pub fn fixed(
        name: &'static str,
        detail: String,
        files: Vec<String>,
        duration: Duration,
    ) -> Self {
        StepResult {
            name,
            state: StepState::Fixed { detail, files },
            extra_output: Vec::new(),
            duration,
        }
    }
}

enum StepState {
    Success { detail: String },
    Fixed { detail: String, files: Vec<String> },
    Skipped { detail: String },
    Failed { error: String },
}

pub fn print_step(result: &StepResult) {
    match &result.state {
        StepState::Success { detail } => {
            println!(
                "[OK] {} - {} {}",
                result.name,
                detail,
                format_duration(result.duration)
            );
        }
        StepState::Fixed { detail, files } => {
            println!(
                "[FIXED] {} - {} {}",
                result.name,
                detail,
                format_duration(result.duration)
            );
            if !files.is_empty() {
                for file in files {
                    println!("    {file}");
                }
            }
        }
        StepState::Skipped { detail } => {
            println!(
                "[SKIP] {} - {} {}",
                result.name,
                detail,
                format_duration(result.duration)
            );
        }
        StepState::Failed { error } => {
            println!(
                "[FAIL] {} - {} {}",
                result.name,
                error,
                format_duration(result.duration)
            );

            if !result.extra_output.is_empty() {
                for line in &result.extra_output {
                    println!("{line}");
                }
            }
        }
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds < 60.0 {
        format!("({seconds:.2}s)")
    } else {
        let minutes = duration.as_secs() / 60;
        let remaining_seconds = duration.as_secs() % 60;
        format!("({minutes}m {remaining_seconds}s)")
    }
}
