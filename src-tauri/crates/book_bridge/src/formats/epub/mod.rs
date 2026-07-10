pub mod nav;
pub mod spine;
mod html;

use std::collections::HashMap;

use axum::{
    extract::{Path as AxumPath, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    body::Body,
};

use crate::bridge::{BridgeState, get_or_start_session, wait_reader};
use crate::util;

use spine::parse_spine;
use nav::parse_nav;

pub async fn handle_spine(
    AxumPath(file_id): AxumPath<i64>,
    State(st): State<BridgeState>,
) -> Response {
    let reader = match get_or_start_session(file_id, &st).await {
        Ok(r) => r,
        Err((StatusCode::ACCEPTED, _)) => {
            match wait_reader(file_id, &st.mgr).await {
                Ok(r) => r,
                Err(e) => return e.into_response(),
            }
        }
        Err(e) => return e.into_response(),
    };

    match parse_spine(&reader).await {
        Ok(entries) => (StatusCode::OK, axum::Json(entries)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn handle_nav(
    AxumPath(file_id): AxumPath<i64>,
    State(st): State<BridgeState>,
) -> Response {
    let reader = match get_or_start_session(file_id, &st).await {
        Ok(r) => r,
        Err((StatusCode::ACCEPTED, _)) => {
            match wait_reader(file_id, &st.mgr).await {
                Ok(r) => r,
                Err(e) => return e.into_response(),
            }
        }
        Err(e) => return e.into_response(),
    };

    let result = async {
        let spine = parse_spine(&reader).await?;
        let spine_map: HashMap<String, usize> = spine.iter()
            .map(|e| (e.path.clone(), e.index))
            .collect();
        let nav = parse_nav(&reader, &spine_map).await?;
        Ok::<_, String>(nav)
    }.await;

    match result {
        Ok(entries) => (StatusCode::OK, axum::Json(entries)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn handle_chapter(
    AxumPath((file_id, chapter_index)): AxumPath<(i64, usize)>,
    State(st): State<BridgeState>,
) -> Response {
    let reader = match get_or_start_session(file_id, &st).await {
        Ok(r) => r,
        Err((StatusCode::ACCEPTED, _)) => {
            match wait_reader(file_id, &st.mgr).await {
                Ok(r) => r,
                Err(e) => return e.into_response(),
            }
        }
        Err(e) => return e.into_response(),
    };

    let spine = match parse_spine(&reader).await {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let entry = match spine.get(chapter_index) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, format!("chapter {chapter_index} not found")).into_response(),
    };

    let bytes = match reader.read_entry(&entry.path).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };

    let base_dir = entry.path.rsplit_once('/').map(|(d, _)| format!("{}/", d)).unwrap_or_default();
    let base_url = format!("http://127.0.0.1:{}/book/{}/res/{}", st.cfg.port, file_id, base_dir);
    let html = String::from_utf8_lossy(&bytes);
    let body = html::rewrite_relative_urls(&html, &base_url);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/xhtml+xml; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

pub async fn handle_resource(
    AxumPath((file_id, path)): AxumPath<(i64, String)>,
    State(st): State<BridgeState>,
) -> Response {
    let reader = match get_or_start_session(file_id, &st).await {
        Ok(r) => r,
        Err((StatusCode::ACCEPTED, _)) => {
            match wait_reader(file_id, &st.mgr).await {
                Ok(r) => r,
                Err(e) => return e.into_response(),
            }
        }
        Err(e) => return e.into_response(),
    };

    let ct = util::content_type(&path);
    let opf_dir = reader.opf_dir.clone();
    let paths: Vec<String> = if opf_dir.is_empty() {
        vec![path]
    } else {
        vec![path.clone(), format!("{}{}", opf_dir, path)]
    };

    for p in &paths {
        match reader.read_entry(p).await {
            Ok(bytes) => return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, ct)
                .body(Body::from(bytes))
                .unwrap(),
            _ => continue,
        }
    }

    (StatusCode::NOT_FOUND, "resource not found").into_response()
}
