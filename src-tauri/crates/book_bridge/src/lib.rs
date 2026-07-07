use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    extract::{Path as AxumPath, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
    body::Body,
};

use omega_drive_gateway::{
    provider::file_repository::FileRepository,
    provider::stream::StreamRegistry,
    provider::storage::PartMetadata,
    core::engine_context::EngineContext,
};

const PART_TIMEOUT: Duration = Duration::from_secs(60);
const WAIT_POLL_MS: u64 = 500;
const WAIT_TIMEOUT: Duration = Duration::from_secs(5 * 60);

// ── Config ──────────────────────────────────────────────

#[derive(Clone)]
pub struct BookBridgeConfig {
    pub base_dir: PathBuf,
    pub file_repo: Arc<dyn FileRepository>,
    pub stream_registry: Arc<StreamRegistry>,
    pub engine: EngineContext,
    pub port: u16,
}

// ── Session manager ─────────────────────────────────────

struct BookSession {
    temp_path: PathBuf,
    opf_dir: String,
    ready: bool,
    error: Option<String>,
}

pub struct BookManager {
    sessions: Arc<tokio::sync::Mutex<HashMap<i64, BookSession>>>,
}

impl BookManager {
    pub fn new() -> Self {
        Self { sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())) }
    }
}

// ── Bridge state (axum) ─────────────────────────────────

#[derive(Clone)]
struct BridgeState {
    cfg: Arc<BookBridgeConfig>,
    mgr: Arc<BookManager>,
}

// ── Public entry point ──────────────────────────────────

pub async fn start_book_bridge(config: BookBridgeConfig, manager: Arc<BookManager>) -> Result<u16, String> {
    let port = config.port;
    let state = BridgeState {
        cfg: Arc::new(config),
        mgr: manager,
    };

    let app = Router::new()
        .route("/book/:file_id/spine", get(handle_spine))
        .route("/book/:file_id/nav", get(handle_nav))
        .route("/book/:file_id/chapter/:chapter_index", get(handle_chapter))
        .route("/book/:file_id/res/*path", get(handle_resource))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| format!("book bridge bind: {e}"))?;
    let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);

    tracing::info!("book bridge ready http://127.0.0.1:{}", actual_port);
    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
    Ok(actual_port)
}

// ── Session helpers ─────────────────────────────────────

async fn get_or_start_session(
    file_id: i64,
    state: &BridgeState,
) -> Result<(PathBuf, bool), (StatusCode, String)> {
    // fast-path: ready session
    {
        let sessions = state.mgr.sessions.lock().await;
        if let Some(s) = sessions.get(&file_id) {
            if let Some(ref e) = s.error {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.clone()));
            }
            if s.ready {
                return Ok((s.temp_path.clone(), true));
            }
        }
    }

    let temp_dir = state.cfg.base_dir.join("cache").join("books");
    tokio::fs::create_dir_all(&temp_dir).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("cache dir: {e}")))?;
    let temp_path = temp_dir.join(format!("{}.epub", file_id));

    // double-check after dir creation
    {
        let mut sessions = state.mgr.sessions.lock().await;
        if let Some(s) = sessions.get(&file_id) {
            if let Some(ref e) = s.error {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.clone()));
            }
            if s.ready {
                return Ok((s.temp_path.clone(), true));
            }
        }
        sessions.insert(file_id, BookSession {
            temp_path: temp_path.clone(),
            opf_dir: String::new(),
            ready: false,
            error: None,
        });
    }

    let tp = temp_path.clone();
    let cfg = Arc::clone(&state.cfg);
    let mgr = Arc::clone(&state.mgr);

    tokio::spawn(async move {
        let result = download_and_assemble(file_id, &tp, &cfg).await;
        let opf_dir = if result.is_ok() {
            let tp2 = tp.clone();
            tokio::task::spawn_blocking(move || parse_opf_dir(&tp2)).await
                .ok().and_then(|r| r.ok()).unwrap_or_default()
        } else {
            String::new()
        };
        let mut sessions = mgr.sessions.lock().await;
        if let Some(s) = sessions.get_mut(&file_id) {
            if result.is_ok() {
                s.opf_dir = opf_dir;
                s.ready = true;
            } else if let Err(e) = result {
                s.error = Some(e);
            }
        }
    });

    Ok((temp_path, false))
}

async fn wait_ready(file_id: i64, mgr: &BookManager) -> Result<PathBuf, (StatusCode, String)> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > WAIT_TIMEOUT {
            return Err((StatusCode::GATEWAY_TIMEOUT, "download timeout".into()));
        }
        tokio::time::sleep(Duration::from_millis(WAIT_POLL_MS)).await;
        let sessions = mgr.sessions.lock().await;
        if let Some(s) = sessions.get(&file_id) {
            if let Some(ref e) = s.error {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.clone()));
            }
            if s.ready {
                return Ok(s.temp_path.clone());
            }
        }
    }
}

// ── Part download & assembly ────────────────────────────

async fn download_and_assemble(file_id: i64, temp_path: &Path, cfg: &BookBridgeConfig) -> Result<(), String> {
    let parts = cfg.file_repo.get_parts_for_file(file_id)
        .await
        .map_err(|e| format!("db: {e}"))?;

    let chunks: Vec<PartMetadata> = parts
        .into_iter()
        .filter(|p| p.part_type == "chunk")
        .fold(BTreeMap::<u32, PartMetadata>::new(), |mut map, p| {
            match map.get(&p.part_index) {
                Some(ex) if ex.platform == "discord" && p.platform != "discord" => {
                    map.insert(p.part_index, p);
                }
                None => { map.insert(p.part_index, p); }
                _ => {}
            }
            map
        })
        .into_values()
        .collect();

    if chunks.is_empty() {
        return Err("no parts found".into());
    }

    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::File::create(temp_path).await
        .map_err(|e| format!("create temp: {e}"))?;

    for p in &chunks {
        let gateway = cfg.stream_registry.get(&p.platform)
            .ok_or_else(|| format!("gateway {} not found", p.platform))?;

        let raw = tokio::time::timeout(
            PART_TIMEOUT,
            gateway.download_part_bytes(p),
        ).await
            .map_err(|_| format!("timeout part {}", p.part_index))?
            .map_err(|e| format!("download part {}: {e}", p.part_index))?;

        file.write_all(&raw).await
            .map_err(|e| format!("write part {}: {e}", p.part_index))?;
    }

    file.flush().await.map_err(|e| format!("flush: {e}"))?;
    drop(file);
    Ok(())
}

// ── EPUB parsing ────────────────────────────────────────

#[derive(serde::Serialize)]
struct SpineEntry {
    index: usize,
    title: String,
    path: String,
}

fn parse_epub_spine(temp_path: &Path) -> Result<Vec<SpineEntry>, String> {
    use std::fs::File;
    use zip::ZipArchive;

    let file = File::open(temp_path).map_err(|e| format!("open epub: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("zip: {e}"))?;

    let container = read_entry_str(&mut archive, "META-INF/container.xml")?;
    let opf_path = extract_attr_value(&container, "full-path=\"")
        .ok_or_else(|| "no rootfile in container.xml".to_string())?;
    let opf_dir = opf_path.rsplit_once('/').map(|(d, _)| format!("{}/", d)).unwrap_or_default();

    let opf = read_entry_str(&mut archive, &opf_path)?;
    let opf = opf.replace("\r\n", " ").replace('\n', " ").replace('\r', " ");

    // manifest: id → href
    let mut id_href: HashMap<String, String> = HashMap::new();
    if let Some(body) = extract_tag_body(&opf, "manifest") {
        for item in body.split("<item ") {
            if item.is_empty() { continue; }
            let id = extract_attr_value(item, "id=\"");
            let href = extract_attr_value(item, "href=\"");
            if let (Some(id), Some(href)) = (id, href) {
                id_href.insert(id, format!("{}{}", opf_dir, href));
            }
        }
    }

    // spine: ordered idrefs
    let mut entries = Vec::new();
    if let Some(body) = extract_tag_body(&opf, "spine") {
        for itemref in body.split("<itemref ") {
            if itemref.is_empty() { continue; }
            let idref = extract_attr_value(itemref, "idref=\"");
            if let Some(idref) = idref {
                if let Some(path) = id_href.get(&idref) {
                    let title = extract_title_from_entry(&mut archive, path)
                        .unwrap_or_else(|| {
                            path.rsplit_once('/')
                                .map(|(_, n)| n.rsplit_once('.').map(|(n, _)| n).unwrap_or(n))
                                .unwrap_or(path)
                                .to_string()
                        });
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

#[derive(serde::Serialize)]
struct NavEntry {
    title: String,
    path: String,
    index: Option<usize>,
    children: Vec<NavEntry>,
}

fn parse_nav(temp_path: &Path, spine_map: &HashMap<String, usize>) -> Result<Vec<NavEntry>, String> {
    use std::fs::File;
    use std::io::Read;
    use zip::ZipArchive;

    let file = File::open(temp_path).map_err(|e| format!("{e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("{e}"))?;

    let nav_html = match read_entry_str(&mut archive, "nav.xhtml") {
        Ok(h) => h,
        Err(_) => {
            let mut e = archive.by_name("toc.ncx").map_err(|e| format!("no nav file: {e}"))?;
            let mut s = String::new();
            e.read_to_string(&mut s).map_err(|e| e.to_string())?;
            s
        }
    };

    Ok(parse_ol_items(&nav_html, spine_map))
}

fn parse_ol_items(html: &str, spine_map: &HashMap<String, usize>) -> Vec<NavEntry> {
    let mut entries = Vec::new();
    let mut pos = 0;
    loop {
        // find next <li>
        let li_start = match html[pos..].find("<li") {
            Some(s) => pos + s,
            None => break,
        };
        // find </li>
        let li_end = match find_closing_tag(&html[li_start..], "li") {
            Some(e) => li_start + e,
            None => break,
        };
        let li_body = &html[li_start..li_end];

        // extract <a> for title + href
        if let Some(a) = extract_tag_body(li_body, "a") {
            let href = extract_attr_value(a, "href=\"").unwrap_or_default();
            let clean = href.split('#').next().unwrap_or("").to_string();
            let path = percent_decode(&clean);
            let title = extract_text_content(a);
            let index = spine_map.get(&path).copied();

            // check for nested <ol>
            let children = if let Some(ol) = extract_tag_body(li_body, "ol") {
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

fn extract_text_content(s: &str) -> String {
    if let Some(start) = s.find('>') {
        let after = &s[start + 1..];
        if let Some(end) = after.find('<') {
            return after[..end].trim().to_string();
        }
    }
    String::new()
}

fn find_closing_tag(s: &str, tag: &str) -> Option<usize> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut depth = 0;
    let mut pos = 0;
    loop {
        if pos >= s.len() {
            return None;
        }
        if s[pos..].starts_with(&close) {
            if depth == 0 {
                return Some(pos + close.len());
            }
            depth -= 1;
            pos += close.len();
        } else if s[pos..].starts_with(&open)
            && (s[pos + open.len()..].starts_with(' ')
                || s[pos + open.len()..].starts_with('>')
                || s[pos + open.len()..].starts_with('\n')
                || s[pos + open.len()..].starts_with('/'))
        {
            depth += 1;
            pos += open.len();
        } else {
            pos += 1;
        }
    }
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            let lo = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            out.push(char::from((hi * 16 + lo) as u8));
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_opf_dir(temp_path: &Path) -> Result<String, String> {
    use std::fs::File;
    use zip::ZipArchive;
    let file = File::open(temp_path).map_err(|e| format!("{e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("{e}"))?;
    let container = read_entry_str(&mut archive, "META-INF/container.xml")?;
    let opf_path = extract_attr_value(&container, "full-path=\"")
        .ok_or_else(|| "no rootfile".to_string())?;
    Ok(opf_path.rsplit_once('/').map(|(d, _)| format!("{}/", d)).unwrap_or_default())
}

fn read_entry_str(archive: &mut zip::ZipArchive<std::fs::File>, path: &str) -> Result<String, String> {
    use std::io::Read;
    let mut entry = archive.by_name(path)
        .map_err(|e| format!("missing {path}: {e}"))?;
    let mut s = String::new();
    entry.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

fn extract_tag_body<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open_start = xml.find(&format!("<{}", tag))?;
    let content_start = xml[open_start..].find('>')? + open_start + 1;
    let close = format!("</{}>", tag);
    let content_end = xml[content_start..].find(&close)?;
    Some(&xml[content_start..content_start + content_end])
}

fn extract_attr_value<'a>(s: &'a str, prefix: &str) -> Option<String> {
    let start = s.find(prefix)? + prefix.len();
    let end = s[start..].find('"')?;
    Some(s[start..start + end].to_string())
}

fn extract_title_from_entry(archive: &mut zip::ZipArchive<std::fs::File>, path: &str) -> Option<String> {
    use std::io::Read;
    let mut entry = archive.by_name(path).ok()?;
    let mut buf = [0u8; 4096];
    let n = entry.read(&mut buf).ok()?;
    let s = std::str::from_utf8(&buf[..n]).ok()?;
    let title_s = s.find("<title>")? + "<title>".len();
    let title_e = s[title_s..].find("</title>")?;
    Some(s[title_s..title_s + title_e].trim().to_string())
}

fn content_type(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm") {
        "application/xhtml+xml"
    } else if lower.ends_with(".css") {
        "text/css"
    } else     if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".ttf") { "font/ttf" }
    else if lower.ends_with(".woff") { "font/woff" }
    else if lower.ends_with(".woff2") { "font/woff2" }
    else if lower.ends_with(".otf") { "font/otf" }
    else if lower.ends_with(".ncx") { "application/x-dtbncx+xml" }
    else { "application/octet-stream" }
}

fn read_zip_entry(temp_path: &Path, path: &str) -> Result<Vec<u8>, String> {
    use std::fs::File;
    use std::io::Read;
    use zip::ZipArchive;
    let file = File::open(temp_path).map_err(|e| format!("{e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("{e}"))?;
    let mut entry = archive.by_name(path).map_err(|e| format!("{e}"))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|e| format!("{e}"))?;
    Ok(buf)
}

// ── Routes ──────────────────────────────────────────────

async fn handle_spine(
    AxumPath(file_id): AxumPath<i64>,
    State(st): State<BridgeState>,
) -> Response {
    let (_, ready) = match get_or_start_session(file_id, &st).await {
        Ok(ok) => ok,
        Err(e) => return e.into_response(),
    };
    if !ready { let _ = wait_ready(file_id, &st.mgr).await; }

    let sessions = st.mgr.sessions.lock().await;
    let temp_path = match sessions.get(&file_id) {
        Some(s) => s.temp_path.clone(),
        None => return (StatusCode::NOT_FOUND, "session gone").into_response(),
    };
    drop(sessions);

    match tokio::task::spawn_blocking(move || parse_epub_spine(&temp_path)).await {
        Ok(Ok(entries)) => (StatusCode::OK, axum::Json(entries)).into_response(),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn handle_nav(
    AxumPath(file_id): AxumPath<i64>,
    State(st): State<BridgeState>,
) -> Response {
    let (_, ready) = match get_or_start_session(file_id, &st).await {
        Ok(ok) => ok,
        Err(e) => return e.into_response(),
    };
    if !ready { let _ = wait_ready(file_id, &st.mgr).await; }

    let temp_path = {
        let sessions = st.mgr.sessions.lock().await;
        match sessions.get(&file_id) {
            Some(s) => s.temp_path.clone(),
            None => return (StatusCode::NOT_FOUND, "session gone").into_response(),
        }
    };

    // Get spine entries + build spine_map
    let result = tokio::task::spawn_blocking(move || {
        let entries = parse_epub_spine(&temp_path)?;
        let spine_map: HashMap<String, usize> = entries.iter()
            .map(|e| (e.path.clone(), e.index))
            .collect();
        let nav = parse_nav(&temp_path, &spine_map)?;
        Ok::<_, String>(nav)
    }).await;

    match result {
        Ok(Ok(entries)) => (StatusCode::OK, axum::Json(entries)).into_response(),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn handle_chapter(
    AxumPath((file_id, chapter_index)): AxumPath<(i64, usize)>,
    State(st): State<BridgeState>,
) -> Response {
    let (_, ready) = match get_or_start_session(file_id, &st).await {
        Ok(ok) => ok,
        Err(e) => return e.into_response(),
    };
    let temp_path = if !ready {
        match wait_ready(file_id, &st.mgr).await {
            Ok(p) => p,
            Err(e) => return e.into_response(),
        }
    } else {
        let sessions = st.mgr.sessions.lock().await;
        match sessions.get(&file_id) {
            Some(s) => s.temp_path.clone(),
            None => return (StatusCode::NOT_FOUND, "session gone").into_response(),
        }
    };

    let cfg = Arc::clone(&st.cfg);
    let port = cfg.port;
    let tp = temp_path.clone();

    // parse spine in blocking thread to get chapter path
    let chapter_path = tokio::task::spawn_blocking(move || {
        let entries = parse_epub_spine(&tp)?;
        let entry = entries.get(chapter_index)
            .ok_or_else(|| format!("chapter {chapter_index} not found"))?;
        Ok::<_, String>((entry.path.clone(), tp))
    }).await;

    let (path, temp_path) = match chapter_path {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return (StatusCode::NOT_FOUND, e).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let path_for_base = path.clone();
    let data = tokio::task::spawn_blocking(move || read_zip_entry(&temp_path, &path)).await;

    let bytes = match data {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let base_dir = path_for_base.rsplit_once('/').map(|(d, _)| format!("{}/", d)).unwrap_or_default();
    let base_tag = format!(
        "<base href=\"http://127.0.0.1:{}/book/{}/res/{}\">",
        port, file_id, base_dir
    );

    let html = String::from_utf8_lossy(&bytes);
    let body = if let Some(pos) = html.find("</head>") {
        format!("{}{}{}", &html[..pos], base_tag, &html[pos..])
    } else {
        format!("<!DOCTYPE html><html><head>{}</head><body>{}</body></html>", base_tag, html)
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/xhtml+xml; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

async fn handle_resource(
    AxumPath((file_id, path)): AxumPath<(i64, String)>,
    State(st): State<BridgeState>,
) -> Response {
    let (_, ready) = match get_or_start_session(file_id, &st).await {
        Ok(ok) => ok,
        Err(e) => return e.into_response(),
    };
    if !ready { let _ = wait_ready(file_id, &st.mgr).await; }

    let (temp_path, opf_dir) = {
        let sessions = st.mgr.sessions.lock().await;
        match sessions.get(&file_id) {
            Some(s) => (s.temp_path.clone(), s.opf_dir.clone()),
            None => return (StatusCode::NOT_FOUND, "session gone").into_response(),
        }
    };

    let ct = content_type(&path);
    let paths: Vec<String> = if opf_dir.is_empty() {
        vec![path]
    } else {
        vec![path.clone(), format!("{}{}", opf_dir, path)]
    };

    for p in &paths {
        let tp = temp_path.clone();
        let pp = p.clone();
        match tokio::task::spawn_blocking(move || read_zip_entry(&tp, &pp)).await {
            Ok(Ok(bytes)) => return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, ct)
                .body(Body::from(bytes))
                .unwrap(),
            _ => continue,
        }
    }

    (StatusCode::NOT_FOUND, "resource not found").into_response()
}
