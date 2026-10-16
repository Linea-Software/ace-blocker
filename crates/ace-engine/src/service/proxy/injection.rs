//! HTML-side resistance injection. Response headers are handled by the proxy;
//! this module updates in-document CSP meta policies using the same nonce.

use dom_query::Document;
use tracing::info;

use crate::service::proxy::csp::modify_csp_with_nonce;
use crate::settings::Settings;

// Remove injected script tags after execution so their payload is not left
// in the DOM. The microtask runs after the current script body finishes.
const SCRIPT_NODE_CLEANUP_PRELUDE: &str =
    "(()=>{const s=document.currentScript;queueMicrotask(()=>s?.remove());})();";

fn injected_script_tag(nonce: &str, kind: &str, script: &str) -> String {
    format!(
        r#"<script nonce="{}" data-ace-resistance="{}">{}
{}</script>"#,
        nonce, kind, SCRIPT_NODE_CLEANUP_PRELUDE, script
    )
}

fn modify_meta_csp_with_nonce(doc: &Document, nonce: &str) {
    let meta_tags = doc.select("meta[http-equiv]");

    for meta_tag in &meta_tags {
        let Some(http_equiv) = meta_tag.attr("http-equiv") else {
            continue;
        };
        if !http_equiv.eq_ignore_ascii_case("content-security-policy") {
            continue;
        }

        let Some(content) = meta_tag.attr("content") else {
            continue;
        };
        let modified = modify_csp_with_nonce(content.as_ref(), nonce);
        meta_tag.set_attr("content", &modified);
    }
}

/// Injects resistance scripts into HTML content with CSP nonce support.
///
/// Both response-header CSP and HTML `<meta http-equiv="Content-Security-Policy">`
/// policies must authorize the same nonce. The response layer rewrites headers;
/// this function rewrites meta policies before adding the scripts so a second
/// in-document policy cannot reject the injection.
///
/// # Arguments
/// * `host` - The hostname to get scripts for
/// * `html_content` - The original HTML content
/// * `is_pro` - Whether the user has pro status
/// * `scripts` - List of script contents to inject (from NetworkManager)
/// * `nonce` - CSP nonce to add to script tags
/// * `intensity` - The current resistance intensity level
///
/// # Returns
/// Modified HTML content with injected scripts
pub fn inject_scripts_with_nonce(
    host: &str,
    html_content: String,
    is_pro: bool,
    scripts: &[String],
    nonce: &str,
    intensity: &str,
) -> String {
    if !is_pro {
        return html_content;
    }

    if scripts.is_empty() {
        return html_content;
    }

    info!(
        "Injecting {} scripts for host: {} with intensity {} and CSP nonce",
        scripts.len(),
        host,
        intensity
    );

    let doc = Document::from(html_content);
    modify_meta_csp_with_nonce(&doc, nonce);

    let resistance_values = injected_script_tag(
        nonce,
        "setup",
        &format!(
            r#"
            window.__BOREDOM_SERVER_HOSTNAME__ = 'ace-engine.local';
            window.__BOREDOM_SERVER_PORT__ = {};
            window.__BOREDOM_INTENSITY__ = '{}';
            console.log('Boredom engine setup successfully')
        "#,
            Settings::get_api_port(),
            intensity
        ),
    );
    doc.select("head").append_html(resistance_values);

    for script in scripts {
        let wrapped_script = injected_script_tag(nonce, "script", script);
        doc.select("head").append_html(wrapped_script);
    }

    doc.html().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_scripts_with_nonce() {
        let html = r#"<!DOCTYPE html><html><head><title>Test</title></head><body></body></html>"#;
        let scripts = vec!["console.log('test');".to_string()];
        let nonce = "abc123";
        let intensity = "high";

        let result = inject_scripts_with_nonce(
            "example.com",
            html.to_string(),
            true,
            &scripts,
            nonce,
            intensity,
        );

        assert!(result.contains(r#"nonce="abc123""#));
        assert!(result.contains("console.log('test');"));
        assert!(result.contains("window.__BOREDOM_SERVER_HOSTNAME__"));
        assert!(result.contains("window.__BOREDOM_INTENSITY__ = 'high'"));
        assert_eq!(result.matches("data-ace-resistance=").count(), 2);
        assert_eq!(result.matches(SCRIPT_NODE_CLEANUP_PRELUDE).count(), 2);
        assert!(
            result.find(SCRIPT_NODE_CLEANUP_PRELUDE).unwrap()
                < result.find("window.__BOREDOM_SERVER_HOSTNAME__").unwrap()
        );
        assert!(
            result.rfind(SCRIPT_NODE_CLEANUP_PRELUDE).unwrap()
                < result.find("console.log('test');").unwrap()
        );
    }

    #[test]
    fn test_inject_scripts_rewrites_meta_csp_with_same_nonce() {
        let html = r#"<!DOCTYPE html><html><head>
            <meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; script-src-elem 'self'">
        </head><body></body></html>"#;
        let scripts = vec!["console.log('test');".to_string()];

        let result = inject_scripts_with_nonce(
            "example.com",
            html.to_string(),
            true,
            &scripts,
            "abc123",
            "high",
        );

        let result_doc = Document::from(result);
        let meta = result_doc.select("meta[http-equiv]").first();
        let content = meta.attr("content").unwrap();
        assert!(content.contains("script-src 'self' 'nonce-abc123'"));
        assert!(content.contains("script-src-elem 'self' 'nonce-abc123'"));
    }

    #[test]
    fn test_inject_scripts_rewrites_meta_csp_case_insensitively() {
        let html = r#"<!DOCTYPE html><html><head>
            <meta http-equiv="content-security-policy" content="default-src 'self'">
        </head><body></body></html>"#;
        let scripts = vec!["console.log('test');".to_string()];

        let result = inject_scripts_with_nonce(
            "example.com",
            html.to_string(),
            true,
            &scripts,
            "abc123",
            "high",
        );

        let result_doc = Document::from(result);
        let content = result_doc
            .select("meta[http-equiv]")
            .first()
            .attr("content")
            .unwrap();
        assert!(content.contains("script-src 'self' 'nonce-abc123'"));
    }

    #[test]
    fn test_inject_scripts_not_pro() {
        let html = r#"<!DOCTYPE html><html><head><title>Test</title></head><body></body></html>"#;
        let scripts = vec!["console.log('test');".to_string()];
        let nonce = "abc123";
        let intensity = "high";

        let result = inject_scripts_with_nonce(
            "example.com",
            html.to_string(),
            false,
            &scripts,
            nonce,
            intensity,
        );

        assert_eq!(result, html);
    }

    #[test]
    fn test_inject_scripts_empty_scripts() {
        let html = r#"<!DOCTYPE html><html><head><title>Test</title></head><body></body></html>"#;
        let scripts: Vec<String> = vec![];
        let nonce = "abc123";
        let intensity = "high";

        let result = inject_scripts_with_nonce(
            "example.com",
            html.to_string(),
            true,
            &scripts,
            nonce,
            intensity,
        );

        assert_eq!(result, html);
    }
}
