use std::collections::HashMap;

use crate::util;

#[derive(Clone, serde::Serialize)]
pub struct NavEntry {
    pub title: String,
    pub path: String,
    pub index: Option<usize>,
    pub children: Vec<NavEntry>,
}

/// Parse nav.xhtml → tree NavEntry
pub async fn parse_nav(reader: &crate::reader::ZipReader, spine_map: &HashMap<String, usize>) -> Result<Vec<NavEntry>, String> {
    if let Some(cached) = reader.get_cached_nav() {
        return Ok(cached.clone());
    }

    let nav_html = match reader.lazy_nav_html().await {
        Ok(h) => h.to_owned(),
        Err(e) => {
            tracing::warn!("nav.xhtml entry not found: {e}");
            let s = reader.read_entry_str("toc.ncx").await
                .map_err(|e| format!("no nav file: {e}"))?;
            s
        }
    };

    tracing::info!(nav_html_len = nav_html.len(), preview = &nav_html[..nav_html.len().min(200)], "parse_nav: raw");

    let entries = parse_ol_items(&nav_html, spine_map);
    reader.set_cached_nav(entries.clone());
    Ok(entries)
}

pub fn find_li_tag(s: &str) -> Option<usize> {
    let pattern = "<li";
    let mut pos = 0;
    loop {
        let found = s[pos..].find(pattern)?;
        let after = pos + found + pattern.len();
        if after >= s.len() || matches!(s.as_bytes().get(after), Some(b'>' | b' ' | b'/' | b'\n')) {
            return Some(pos + found);
        }
        pos = after;
    }
}

pub fn parse_ol_items(html: &str, spine_map: &HashMap<String, usize>) -> Vec<NavEntry> {
    let mut entries = Vec::new();
    let mut pos = 0;
    let mut total_found = 0;
    loop {
        let li_start = match find_li_tag(&html[pos..]) {
            Some(s) => pos + s,
            None => {
                tracing::info!(total_found, prev_pos = pos, "parse_ol_items: no more <li>");
                break;
            }
        };
        total_found += 1;
        let li_end = match util::find_closing_tag(&html[li_start..], "li") {
            Some(e) => li_start + e,
            None => {
                tracing::warn!(li_start, html_len = html.len(), "parse_ol_items: find_closing_tag failed");
                break;
            }
        };
        let li_body = &html[li_start..li_end];

        let a_body = util::extract_tag_body(li_body, "a");
        if a_body.is_some() {
            let href = util::extract_attr_value(li_body, "href=\"").unwrap_or_default();
            let clean = href.split('#').next().unwrap_or("").to_string();
            let path = util::percent_decode(&clean);
            let title = a_body.unwrap_or("").trim().to_string();
            let index = spine_map.get(&path).copied();

            let children = if let Some(ol) = util::extract_tag_body(li_body, "ol") {
                parse_ol_items(ol, spine_map)
            } else {
                Vec::new()
            };

            entries.push(NavEntry { title, path, index, children });
        }

        pos = li_end;
    }
    entries
}
