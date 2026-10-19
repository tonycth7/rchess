// src/http.rs — shared HTTP agent configuration (ureq + native-tls)
//
// Every Lichess request goes through one of these builders so TLS and timeout
// behaviour is configured in a single place. native-tls (system OpenSSL) is
// used deliberately: it sidesteps the ring/rustls linking issues seen when
// building the AUR package. ureq never selects native-tls by default, so it
// must be requested explicitly on the agent's TLS config.

use std::time::Duration;

use ureq::config::ConfigBuilder;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::typestate::AgentScope;

/// Base configuration shared by all agents: native-tls with the platform's
/// root store, and non-2xx responses returned instead of turned into errors so
/// callers can surface the response body.
fn base_config() -> ConfigBuilder<AgentScope> {
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .http_status_as_error(false)
}

/// Build an agent for one-shot requests.
///
/// `global` bounds the whole call end-to-end (DNS, connect, headers *and* body);
/// pass `None` to disable it.
pub fn agent(connect: Duration, global: Option<Duration>) -> ureq::Agent {
    base_config()
        .timeout_connect(Some(connect))
        .timeout_global(global)
        .build()
        .new_agent()
}

/// Build an agent for long-lived streaming endpoints.
///
/// A global timeout would abort the stream mid-response, so only the connect
/// and response-header timeouts are set.
pub fn streaming_agent(connect: Duration, recv_headers: Duration) -> ureq::Agent {
    base_config()
        .timeout_connect(Some(connect))
        .timeout_recv_response(Some(recv_headers))
        .build()
        .new_agent()
}

/// Read a response body as UTF-8 text, turning a non-success status into an
/// error that includes the status line and any body the server sent.
pub fn read_text(resp: &mut ureq::http::Response<ureq::Body>) -> Result<String, String> {
    let status = resp.status();
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("read: {e}"))?;
    if status.is_success() {
        Ok(body)
    } else {
        Err(format!("HTTP {status}: {body}"))
    }
}
