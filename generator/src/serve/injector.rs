//! HTML injection for live reload script.
//!
//! This module provides functionality to inject the live reload WebSocket
//! client script into HTML responses.

/// The JavaScript code for the live reload WebSocket client.
pub const LIVE_RELOAD_SCRIPT: &str = r#"
<script>
(function() {
    let ws = null;
    let reconnectAttempts = 0;
    const maxReconnectAttempts = 10;
    const reconnectDelay = 1000;

    function connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = protocol + '//' + window.location.host + '/__ws__';
        
        ws = new WebSocket(wsUrl);
        
        ws.onopen = function() {
            console.log('[yew-ssg] Live reload connected');
            reconnectAttempts = 0;
        };
        
        ws.onmessage = function(event) {
            try {
                const data = JSON.parse(event.data);
                if (data.type === 'reload') {
                    console.log('[yew-ssg] Reloading due to:', data.change_type, data.files);
                    window.location.reload();
                } else if (data.type === 'error') {
                    console.error('[yew-ssg] Build error:', data.message);
                    showErrorOverlay(data.message);
                }
            } catch (e) {
                console.error('[yew-ssg] Failed to parse message:', e);
            }
        };
        
        ws.onclose = function() {
            console.log('[yew-ssg] WebSocket closed');
            if (reconnectAttempts < maxReconnectAttempts) {
                reconnectAttempts++;
                setTimeout(connect, reconnectDelay);
            }
        };
        
        ws.onerror = function(err) {
            console.error('[yew-ssg] WebSocket error:', err);
        };
    }

    function showErrorOverlay(message) {
        // Remove existing overlay if present
        const existing = document.getElementById('__yew_ssg_error__');
        if (existing) existing.remove();
        
        const overlay = document.createElement('div');
        overlay.id = '__yew_ssg_error__';
        overlay.style.cssText = 'position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.85);color:#fff;z-index:99999;display:flex;flex-direction:column;align-items:center;justify-content:center;font-family:monospace;padding:20px;';
        
        const title = document.createElement('h1');
        title.textContent = 'Build Error';
        title.style.cssText = 'color:#ff6b6b;margin-bottom:20px;';
        
        const pre = document.createElement('pre');
        pre.textContent = message;
        pre.style.cssText = 'max-width:80%;overflow:auto;background:#1a1a1a;padding:20px;border-radius:8px;border:1px solid #ff6b6b;';
        
        const closeBtn = document.createElement('button');
        closeBtn.textContent = 'Close';
        closeBtn.style.cssText = 'margin-top:20px;padding:10px 20px;background:#ff6b6b;color:#fff;border:none;border-radius:4px;cursor:pointer;font-size:14px;';
        closeBtn.onclick = function() { overlay.remove(); };
        
        overlay.appendChild(title);
        overlay.appendChild(pre);
        overlay.appendChild(closeBtn);
        document.body.appendChild(overlay);
    }

    connect();
})();
</script>
"#;

/// Inject the live reload script into HTML content.
///
/// The script is injected immediately before the closing `</body>` tag.
/// If no `</body>` tag is found, the script is appended to the end.
///
/// # Arguments
///
/// * `html` - The original HTML content
///
/// # Returns
///
/// The HTML with the live reload script injected.
pub fn inject_live_reload_script(html: &str) -> String {
    // Find the closing body tag (case-insensitive)
    let body_close = html.rfind("</body>").or_else(|| html.rfind("</BODY>"));

    match body_close {
        Some(pos) => {
            // Insert before </body>
            let mut result = String::with_capacity(html.len() + LIVE_RELOAD_SCRIPT.len());
            result.push_str(&html[..pos]);
            result.push_str(LIVE_RELOAD_SCRIPT);
            result.push_str(&html[pos..]);
            result
        }
        None => {
            // No </body> tag found, append to end
            let mut result = String::with_capacity(html.len() + LIVE_RELOAD_SCRIPT.len());
            result.push_str(html);
            result.push_str(LIVE_RELOAD_SCRIPT);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_before_body_close() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <h1>Hello World</h1>
</body>
</html>"#;

        let result = inject_live_reload_script(html);

        assert!(result.contains("<script>"));
        assert!(result.contains("WebSocket"));
        // Script should be before </body>
        let script_pos = result.find("<script>").unwrap();
        let body_pos = result.find("</body>").unwrap();
        assert!(script_pos < body_pos);
    }

    #[test]
    fn test_inject_without_body_tag() {
        let html = "<html><head></head><body><p>Test</p></html>";
        let result = inject_live_reload_script(html);

        assert!(result.contains("<script>"));
        // Script should be at the end
        assert!(result.ends_with("</script>\n"));
    }

    #[test]
    fn test_inject_empty_html() {
        let html = "";
        let result = inject_live_reload_script(html);

        assert!(result.contains("<script>"));
    }

    #[test]
    fn test_inject_preserves_content() {
        let html = r#"<html><body><p>Important content</p></body></html>"#;
        let result = inject_live_reload_script(html);

        assert!(result.contains("Important content"));
    }

    #[test]
    fn test_inject_case_insensitive_body() {
        let html = "<html><body><p>Test</p></BODY></html>";
        let result = inject_live_reload_script(html);

        assert!(result.contains("<script>"));
        let script_pos = result.find("<script>").unwrap();
        let body_pos = result.find("</BODY>").unwrap();
        assert!(script_pos < body_pos);
    }

    #[test]
    fn test_script_contains_websocket() {
        assert!(LIVE_RELOAD_SCRIPT.contains("WebSocket"));
        assert!(LIVE_RELOAD_SCRIPT.contains("__ws__"));
        assert!(LIVE_RELOAD_SCRIPT.contains("reload"));
    }

    #[test]
    fn test_script_contains_error_overlay() {
        assert!(LIVE_RELOAD_SCRIPT.contains("showErrorOverlay"));
        assert!(LIVE_RELOAD_SCRIPT.contains("__yew_ssg_error__"));
    }
}
