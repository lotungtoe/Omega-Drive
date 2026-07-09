use std::collections::HashMap;

use crate::reader::ZipReader;
use crate::util;

use super::nav::{self, NavEntry};

#[derive(serde::Serialize)]
pub struct SpineEntry {
    pub index: usize,
    pub title: String,
    pub path: String,
}

/// Parse OPF spine + build title map từ nav.xhtml
pub async fn parse_spine(reader: &ZipReader) -> Result<Vec<SpineEntry>, String> {
    let container = reader.read_entry_str("META-INF/container.xml").await?;
    let opf_path = util::extract_attr_value(&container, "full-path=\"")
        .ok_or_else(|| "no rootfile".to_string())?;
    let opf_dir = opf_path.rsplit_once('/').map(|(d, _)| format!("{}/", d)).unwrap_or_default();

    let opf = reader.read_entry_str(&opf_path).await?;
    let opf = opf.replace("\r\n", " ").replace('\n', " ").replace('\r', " ");

    // Build title map from nav.xhtml (1 file instead of 3800)
    let mut nav_titles: HashMap<String, String> = HashMap::new();
    if let Ok(nav_html) = reader.read_entry_str("nav.xhtml").await {
        let entries = nav::parse_ol_items(&nav_html, &HashMap::new());
        flatten_nav_titles(&entries, &opf_dir, &mut nav_titles);
    }

    // manifest: id → href
    let mut id_href: HashMap<String, String> = HashMap::new();
    if let Some(body) = util::extract_tag_body(&opf, "manifest") {
        for item in body.split("<item ") {
            if item.is_empty() { continue; }
            let id = util::extract_attr_value(item, "id=\"");
            let href = util::extract_attr_value(item, "href=\"");
            if let (Some(id), Some(href)) = (id, href) {
                id_href.insert(id, format!("{}{}", opf_dir, href));
            }
        }
    }

    // spine: ordered idrefs
    let mut entries = Vec::new();
    if let Some(body) = util::extract_tag_body(&opf, "spine") {
        for itemref in body.split("<itemref ") {
            if itemref.is_empty() { continue; }
            let idref = util::extract_attr_value(itemref, "idref=\"");
            if let Some(idref) = idref {
                if let Some(path) = id_href.get(&idref) {
                    let title = match nav_titles.get(path.as_str()) {
                        Some(t) => t.clone(),
                        None => {
                            extract_title_from_reader(reader, path).await
                                .unwrap_or_else(|| {
                                    path.rsplit_once('/')
                                        .map(|(_, n)| n.rsplit_once('.').map(|(n, _)| n).unwrap_or(n))
                                        .unwrap_or(path)
                                        .to_string()
                                })
                        }
                    };
                    entries.push(SpineEntry {
                        index: entries.len(),
                        title,
                        path: path.clone(),
                    });
                }
            }
        }
    }

    Ok(entries)
}

fn flatten_nav_titles(entries: &[NavEntry], opf_dir: &str, out: &mut HashMap<String, String>) {
    for e in entries {
        if !e.path.is_empty() && !e.title.is_empty() {
            out.insert(format!("{}{}", opf_dir, e.path), e.title.clone());
        }
        if !e.children.is_empty() {
            flatten_nav_titles(&e.children, opf_dir, out);
        }
    }
}

/// Extract title từ chapter HTML (fallback khi nav map miss)
async fn extract_title_from_reader(reader: &ZipReader, path: &str) -> Option<String> {
    let data = reader.read_entry(path).await.ok()?;
    let s = std::str::from_utf8(&data).ok()?;
    let title_s = s.find("<title>")? + "<title>".len();
    let title_e = s[title_s..].find("</title>")?;
    let t = s[title_s..title_s + title_e].trim();
    if t == "Unknown" {
        let body = if let Some(b) = util::extract_tag_body(s, "body") { b } else { "" };
        if let Some(h2) = util::extract_first_tag_text(body, "h2") { return Some(h2); }
        if let Some(h1) = util::extract_first_tag_text(body, "h1") { return Some(h1); }
    }
    Some(t.to_string())
}
