use crate::events::{screenshot_session, AppUserEvent};
use crate::rpc::handle_api_request;
use crate::ui_assets::{resolve_ui_asset, INDEX_HTML};
use std::borrow::Cow;
use tao::event_loop::EventLoopProxy;
use wry::{http::Response, WebViewBuilder};

pub fn build_webview_with_api(
    window: &tao::window::Window,
    url: &str,
    proxy: EventLoopProxy<AppUserEvent>,
) -> wry::Result<wry::WebView> {
    WebViewBuilder::new()
        .with_transparent(true)
        .with_custom_protocol("omni".into(), move |_webview_id, request| {
            let path = request.uri().path();
            if path == "/api" {
                let resp_json = handle_api_request(request.body(), &proxy);
                let bytes = serde_json::to_vec(&resp_json).unwrap_or_default();
                Response::builder()
                    .header("Content-Type", "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Cow::Owned(bytes))
                    .unwrap()
            } else if path == "/screenshot.png"
                || (path.starts_with("/screenshot_") && path.ends_with(".png"))
            {
                let slot_idx = if path.starts_with("/screenshot_") {
                    path.trim_start_matches("/screenshot_")
                        .trim_end_matches(".png")
                        .parse::<usize>()
                        .unwrap_or(0)
                } else {
                    0
                };
                let bytes = {
                    let sess = screenshot_session().lock().unwrap();
                    sess.slots
                        .get(slot_idx)
                        .map(|s| s.png_bytes.clone())
                        .unwrap_or_default()
                };
                Response::builder()
                    .header("Content-Type", "image/png")
                    .header("Cache-Control", "no-store, no-cache, must-revalidate")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Cow::Owned(bytes))
                    .unwrap()
            } else if let Some((content_type, asset_bytes)) = resolve_ui_asset(path) {
                Response::builder()
                    .header("Content-Type", content_type)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Cow::Borrowed(asset_bytes))
                    .unwrap()
            } else {
                Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(Cow::Borrowed(INDEX_HTML.as_bytes()))
                    .unwrap()
            }
        })
        .with_url(url)
        .build(window)
}

