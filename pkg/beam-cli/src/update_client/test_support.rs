use std::sync::{Mutex, OnceLock};

pub(crate) struct TestReleasesUrlGuard {
    previous: Option<String>,
}

pub(crate) fn override_releases_url_for_tests(
    releases_url: impl Into<String>,
) -> TestReleasesUrlGuard {
    let mut guard = releases_url_override_cell()
        .lock()
        .expect("lock test releases url override");
    let previous = guard.replace(releases_url.into());

    TestReleasesUrlGuard { previous }
}

pub(crate) fn releases_url_override() -> Option<String> {
    releases_url_override_cell()
        .lock()
        .expect("lock test releases url override")
        .clone()
}

impl Drop for TestReleasesUrlGuard {
    fn drop(&mut self) {
        *releases_url_override_cell()
            .lock()
            .expect("lock test releases url override") = self.previous.take();
    }
}

fn releases_url_override_cell() -> &'static Mutex<Option<String>> {
    static RELEASES_URL_OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

    RELEASES_URL_OVERRIDE.get_or_init(|| Mutex::new(None))
}
