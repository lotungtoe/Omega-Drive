/// Rewrite relative URLs in HTML to be absolute for the proxy.
pub fn rewrite_relative_urls(html: &str, base_url: &str) -> String {
    let mut out = String::with_capacity(html.len() + 256);
    let mut pos = 0;

    while pos < html.len() {
        let remaining = &html[pos..];
        let attr = if let Some(i) = remaining.find(" src=\"") {
            Some((i, " src=\"", '"'))
        } else if let Some(i) = remaining.find(" href=\"") {
            Some((i, " href=\"", '"'))
        } else if let Some(i) = remaining.find(" xlink:href=\"") {
            Some((i, " xlink:href=\"", '"'))
        } else {
            None
        };

        match attr {
            Some((i, prefix, _)) => {
                out.push_str(&remaining[..i + prefix.len()]);
                let val_start = i + prefix.len();
                let val_end = remaining[val_start..].find('"').map(|e| val_start + e).unwrap_or(remaining.len());
                let raw_url = &remaining[val_start..val_end];
                if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") && !raw_url.starts_with("data:") && !raw_url.starts_with('#') {
                    out.push_str(base_url);
                    out.push_str(raw_url);
                    out.push('"');
                } else {
                    out.push_str(raw_url);
                }
                pos += i + prefix.len() + (val_end - val_start) + 1;
            }
            None => {
                out.push_str(remaining);
                break;
            }
        }
    }
    out
}
