use base64::Engine;
use http::{HeaderMap, HeaderName, HeaderValue};
use rand::prelude::*;

const CSP_HEADER_NAME: &str = "content-security-policy";

/// Generates a cryptographically secure nonce for CSP.
/// The nonce is a base64-encoded random string of 32 bytes.
pub fn generate_nonce() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random::<u8>()).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn add_source_token(value: &str, token: &str) -> String {
    let mut sources: Vec<&str> = value
        .split_whitespace()
        .filter(|source| *source != "'none'")
        .collect();

    if !sources.contains(&token) {
        sources.push(token);
    }

    sources.join(" ")
}

fn source_list_allows_unsafe_inline(value: &str) -> bool {
    value
        .split_whitespace()
        .any(|source| source.eq_ignore_ascii_case("'unsafe-inline'"))
}

fn source_list_has_nonce_or_hash(value: &str) -> bool {
    value.split_whitespace().any(|source| {
        let source = source.to_ascii_lowercase();
        source.starts_with("'nonce-")
            || source.starts_with("'sha256-")
            || source.starts_with("'sha384-")
            || source.starts_with("'sha512-")
    })
}

fn unsafe_inline_must_be_preserved(value: &str) -> bool {
    source_list_allows_unsafe_inline(value) && !source_list_has_nonce_or_hash(value)
}

fn sandbox_directive_blocks_scripts(directive: &str) -> bool {
    let mut parts = directive.split_whitespace();
    if !parts
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("sandbox"))
    {
        return false;
    }

    !parts.any(|token| token.eq_ignore_ascii_case("allow-scripts"))
}

/// Modifies one Content-Security-Policy value so Ace's injected inline scripts
/// are permitted without discarding the site's other CSP protections.
///
/// The injected nonce is added to both `script-src` and `script-src-elem` when
/// those directives exist. If `script-src` is absent but `default-src` exists,
/// a `script-src` directive is derived from `default-src` so adding the nonce
/// does not accidentally make the site's existing scripts more restrictive.
///
/// Policies that already use `'unsafe-inline'` are intentionally left without
/// an added nonce for that directive. Adding any nonce/hash to the source list
/// causes browsers to ignore `'unsafe-inline'`, which would break the site's
/// own inline scripts while trying to authorize Ace's injected script.
///
/// CSP `sandbox` is never relaxed here. The response inspector skips injection
/// when an enforcing sandbox does not already contain `allow-scripts`.
pub fn modify_csp_with_nonce(csp: &str, nonce: &str) -> String {
    let nonce_value = format!("'nonce-{}'", nonce);
    let mut directives: Vec<(String, String)> = Vec::new();
    let mut has_script_src = false;
    let mut default_src_value: Option<String> = None;

    for directive in csp.split(';') {
        let directive = directive.trim();
        if directive.is_empty() {
            continue;
        }

        if let Some((name, value)) = directive.split_once(char::is_whitespace) {
            let name = name.trim().to_lowercase();
            let value = value.trim().to_string();

            match name.as_str() {
                "script-src" => {
                    has_script_src = true;
                    if unsafe_inline_must_be_preserved(&value) {
                        directives.push((name, value));
                    } else {
                        directives.push((name, add_source_token(&value, &nonce_value)));
                    }
                }
                "script-src-elem" => {
                    if unsafe_inline_must_be_preserved(&value) {
                        directives.push((name, value));
                    } else {
                        directives.push((name, add_source_token(&value, &nonce_value)));
                    }
                }
                "default-src" => {
                    default_src_value = Some(value.clone());
                    directives.push((name, value));
                }
                _ => directives.push((name, value)),
            }
        } else {
            let name = directive.to_lowercase();
            directives.push((name, String::new()));
        }
    }

    if !has_script_src
        && let Some(default_src) = default_src_value
        && !unsafe_inline_must_be_preserved(&default_src)
    {
        directives.push((
            "script-src".to_string(),
            add_source_token(&default_src, &nonce_value),
        ));
    }

    directives
        .into_iter()
        .map(|(name, value)| {
            if value.is_empty() {
                name
            } else {
                format!("{} {}", name, value)
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Returns true when a CSP value contains an enforcing sandbox that does not
/// grant `allow-scripts`.
fn csp_value_blocks_script_injection(csp: &str) -> bool {
    csp.split(',').any(|policy| {
        policy
            .split(';')
            .map(str::trim)
            .any(sandbox_directive_blocks_scripts)
    })
}

/// Returns true when any enforcing CSP header sandboxes the document without
/// `allow-scripts`. Such a policy blocks script execution independently of a
/// nonce, so resistance injection must be skipped rather than weakening the
/// site's sandbox boundary.
pub fn csp_headers_block_script_injection(headers: &HeaderMap) -> bool {
    let header_name = HeaderName::from_static(CSP_HEADER_NAME);

    headers
        .get_all(&header_name)
        .iter()
        .any(|value| value.to_str().is_ok_and(csp_value_blocks_script_injection))
}

fn modify_csp_header_value_with_nonce(csp_header: &str, nonce: &str) -> String {
    csp_header
        .split(',')
        .map(|policy| modify_csp_with_nonce(policy.trim(), nonce))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Rewrites every enforcing CSP response header with the same injection nonce.
/// Browsers enforce repeated headers and comma-separated policies independently,
/// so every policy must explicitly authorize Ace's scripts.
pub fn modify_csp_headers_with_nonce(headers: &mut HeaderMap, nonce: &str) {
    let header_name = HeaderName::from_static(CSP_HEADER_NAME);
    let original_values: Vec<HeaderValue> = headers.get_all(&header_name).iter().cloned().collect();

    if original_values.is_empty() {
        return;
    }

    headers.remove(&header_name);

    for original_value in original_values {
        let replacement = original_value
            .to_str()
            .ok()
            .map(|csp| modify_csp_header_value_with_nonce(csp, nonce))
            .and_then(|csp| HeaderValue::from_str(&csp).ok())
            .unwrap_or(original_value);
        headers.append(header_name.clone(), replacement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_nonce_length() {
        let nonce = generate_nonce();
        // Base64 of 32 bytes = 44 characters (with padding)
        assert_eq!(nonce.len(), 44);
    }

    #[test]
    fn test_generate_nonce_uniqueness() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_modify_csp_preserves_unsafe_inline_without_adding_nonce() {
        let csp = "default-src 'self'; script-src 'self' 'unsafe-inline'; img-src 'self' data:";
        let modified = modify_csp_with_nonce(csp, "abc123");

        assert!(modified.contains("script-src 'self' 'unsafe-inline'"));
        assert!(!modified.contains("script-src 'self' 'unsafe-inline' 'nonce-abc123'"));
        assert!(modified.contains("img-src 'self' data:"));
    }

    #[test]
    fn test_modify_csp_adds_nonce_when_unsafe_inline_is_already_disabled_by_nonce() {
        let csp = "script-src 'self' 'unsafe-inline' 'nonce-site123'";
        let modified = modify_csp_with_nonce(csp, "ace456");

        assert!(modified.contains("'nonce-site123'"));
        assert!(modified.contains("'nonce-ace456'"));
    }

    #[test]
    fn test_modify_csp_adds_nonce_to_script_src_elem() {
        let csp =
            "default-src 'self'; script-src 'self'; script-src-elem 'self' https://cdn.example";
        let modified = modify_csp_with_nonce(csp, "abc123");

        assert!(modified.contains("script-src 'self' 'nonce-abc123'"));
        assert!(modified.contains("script-src-elem 'self' https://cdn.example 'nonce-abc123'"));
    }

    #[test]
    fn test_modify_csp_derives_script_src_from_default_src() {
        let csp = "default-src 'self' https://cdn.example; img-src 'self' data:";
        let modified = modify_csp_with_nonce(csp, "xyz789");

        assert!(modified.contains("script-src 'self' https://cdn.example 'nonce-xyz789'"));
    }

    #[test]
    fn test_modify_csp_does_not_add_script_restriction_when_no_default_exists() {
        let csp = "img-src 'self' data:; connect-src https://api.example";
        let modified = modify_csp_with_nonce(csp, "xyz789");

        assert!(!modified.contains("script-src"));
        assert_eq!(modified, csp);
    }

    #[test]
    fn test_modify_csp_preserves_other_directives() {
        let csp =
            "default-src 'self'; style-src 'self'; connect-src 'self' https://api.example.com";
        let modified = modify_csp_with_nonce(csp, "test");

        assert!(modified.contains("default-src 'self'"));
        assert!(modified.contains("style-src 'self'"));
        assert!(modified.contains("connect-src 'self' https://api.example.com"));
    }

    #[test]
    fn test_modify_csp_no_duplicate_nonce() {
        let csp = "script-src 'self' 'nonce-abc123'";
        let modified = modify_csp_with_nonce(csp, "abc123");

        assert_eq!(modified.matches("'nonce-abc123'").count(), 1);
    }

    #[test]
    fn test_modify_csp_replaces_none_when_nonce_is_added() {
        let modified = modify_csp_with_nonce("script-src 'none'", "abc123");
        assert_eq!(modified, "script-src 'nonce-abc123'");
    }

    #[test]
    fn test_modify_csp_does_not_relax_csp_sandbox() {
        let modified = modify_csp_with_nonce(
            "default-src 'self'; sandbox allow-forms allow-popups",
            "abc123",
        );

        assert!(modified.contains("sandbox allow-forms allow-popups"));
        assert!(!modified.contains("sandbox allow-forms allow-popups allow-scripts"));
    }

    #[test]
    fn test_blocking_sandbox_is_detected_before_injection() {
        let header_name = HeaderName::from_static(CSP_HEADER_NAME);
        let mut headers = HeaderMap::new();
        headers.insert(
            header_name.clone(),
            HeaderValue::from_static("default-src 'self'; sandbox allow-forms"),
        );
        assert!(csp_headers_block_script_injection(&headers));

        headers.insert(
            header_name,
            HeaderValue::from_static("default-src 'self'; sandbox allow-scripts allow-forms"),
        );
        assert!(!csp_headers_block_script_injection(&headers));
    }

    #[test]
    fn test_modify_all_comma_separated_csp_policies() {
        let header_name = HeaderName::from_static(CSP_HEADER_NAME);
        let mut headers = HeaderMap::new();
        headers.insert(
            header_name.clone(),
            HeaderValue::from_static(
                "script-src 'self', script-src-elem https://cdn.example; sandbox",
            ),
        );

        modify_csp_headers_with_nonce(&mut headers, "abc123");

        let value = headers.get(&header_name).unwrap().to_str().unwrap();
        let policies: Vec<&str> = value.split(',').map(str::trim).collect();
        assert_eq!(policies.len(), 2);
        assert!(policies[0].contains("script-src 'self' 'nonce-abc123'"));
        assert!(policies[1].contains("script-src-elem https://cdn.example 'nonce-abc123'"));
        assert!(policies[1].contains("sandbox"));
        assert!(!policies[1].contains("sandbox allow-scripts"));
    }

    #[test]
    fn test_modify_all_repeated_csp_headers() {
        let header_name = HeaderName::from_static(CSP_HEADER_NAME);
        let mut headers = HeaderMap::new();
        headers.append(
            header_name.clone(),
            HeaderValue::from_static("script-src 'self'"),
        );
        headers.append(
            header_name.clone(),
            HeaderValue::from_static("script-src-elem https://cdn.example; sandbox"),
        );

        modify_csp_headers_with_nonce(&mut headers, "abc123");

        let values: Vec<&str> = headers
            .get_all(&header_name)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect();
        assert_eq!(values.len(), 2);
        assert!(values[0].contains("script-src 'self' 'nonce-abc123'"));
        assert!(values[1].contains("script-src-elem https://cdn.example 'nonce-abc123'"));
        assert!(values[1].contains("sandbox"));
        assert!(!values[1].contains("sandbox allow-scripts"));
    }
}
