use std::sync::{Arc, Mutex, MutexGuard};

use reqwest::{
    Method, StatusCode, Url,
    header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, TRANSFER_ENCODING},
    redirect::Policy,
};

use super::RequestSpec;

#[derive(Clone, Debug, Default)]
pub(super) struct RedirectTracker {
    followed_redirects: Arc<Mutex<Vec<RedirectStep>>>,
}

#[derive(Clone, Debug)]
struct RedirectStep {
    next_url: Url,
    status: StatusCode,
}

impl RedirectTracker {
    pub(super) fn clear(&self) {
        self.followed_redirects().clear();
    }

    pub(super) fn effective_request_spec(&self, spec: &RequestSpec) -> RequestSpec {
        let followed_redirects = self.followed_redirects();
        apply_followed_redirects(spec, followed_redirects.as_slice())
    }

    fn followed_redirects(&self) -> MutexGuard<'_, Vec<RedirectStep>> {
        match self.followed_redirects.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn record_follow(&self, status: StatusCode, next_url: &Url) {
        self.followed_redirects().push(RedirectStep {
            next_url: next_url.clone(),
            status,
        });
    }
}

pub(super) fn origin_locked_redirect_policy(
    max_redirects: usize,
    original_url: &Url,
    redirect_tracker: RedirectTracker,
) -> Policy {
    let original_origin = RequestOrigin::from_url(original_url);

    Policy::custom(move |attempt| {
        if attempt.previous().len() >= max_redirects {
            return attempt.error("too many redirects");
        }

        if original_origin.matches(attempt.url()) {
            redirect_tracker.record_follow(attempt.status(), attempt.url());
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

pub(super) fn retry_spec_for_challenge(spec: &RequestSpec, challenged_url: Url) -> RequestSpec {
    if RequestOrigin::from_url(&spec.url).matches(&challenged_url) {
        return RequestSpec {
            url: challenged_url,
            ..spec.clone()
        };
    }

    // Cross-origin challenges must not replay origin-scoped request metadata.
    RequestSpec {
        body: None,
        headers: HeaderMap::new(),
        method: cross_origin_retry_method(&spec.method),
        url: challenged_url,
    }
}

fn cross_origin_retry_method(method: &Method) -> Method {
    if *method == Method::HEAD {
        Method::HEAD
    } else {
        Method::GET
    }
}

fn apply_followed_redirects(
    spec: &RequestSpec,
    followed_redirects: &[RedirectStep],
) -> RequestSpec {
    let mut effective_spec = spec.clone();

    for redirect in followed_redirects {
        effective_spec.url = redirect.next_url.clone();

        match redirect.status {
            StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND => {
                if effective_spec.method == Method::POST {
                    rewrite_request_as_get(&mut effective_spec);
                }
            }
            StatusCode::SEE_OTHER => {
                if effective_spec.method != Method::HEAD {
                    effective_spec.method = Method::GET;
                }

                drop_request_payload(&mut effective_spec);
            }
            StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT => {}
            _ => {}
        }
    }

    effective_spec
}

fn rewrite_request_as_get(spec: &mut RequestSpec) {
    spec.method = Method::GET;
    drop_request_payload(spec);
}

fn drop_request_payload(spec: &mut RequestSpec) {
    spec.body = None;

    for header in &[
        CONTENT_TYPE,
        CONTENT_LENGTH,
        CONTENT_ENCODING,
        TRANSFER_ENCODING,
    ] {
        spec.headers.remove(header);
    }
}

#[derive(Clone, Debug)]
struct RequestOrigin {
    host: Option<String>,
    port: Option<u16>,
    scheme: String,
}

impl RequestOrigin {
    fn from_url(url: &Url) -> Self {
        Self {
            host: url.host_str().map(ToString::to_string),
            port: url.port_or_known_default(),
            scheme: url.scheme().to_string(),
        }
    }

    fn matches(&self, url: &Url) -> bool {
        self.scheme == url.scheme()
            && self.port == url.port_or_known_default()
            && self
                .host
                .as_deref()
                .zip(url.host_str())
                .is_some_and(|(expected, actual)| expected.eq_ignore_ascii_case(actual))
    }
}
