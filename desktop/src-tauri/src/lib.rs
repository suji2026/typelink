use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use axum::{
    extract::{State, Query, ws::{WebSocket, WebSocketUpgrade, Message}},
    response::{IntoResponse, Html},
    routing::get,
    Router,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use if_addrs::get_if_addrs;
use std::net::IpAddr;
use qrcode::QrCode;
use base64::{Engine as _, engine::general_purpose};
use arboard::Clipboard;
use enigo::{Enigo, KeyboardControllable, Key};

const INDEX_HTML: &str = include_str!("../../public/index.html");
const QR_HTML: &str = include_str!("../../public/qr.html");

#[derive(Clone)]
struct AppState {
    client_count: Arc<AtomicUsize>,
    status_menu_item: MenuItem<tauri::Wry>,
    app_handle: tauri::AppHandle,
}

#[derive(Serialize, Clone)]
struct IpInfo {
    ip: String,
    name: String,
    label: String,
}

fn guess_interface_type(name: &str, ip: &str) -> String {
    let name_lower = name.to_lowercase();
    let ip_parts: Vec<&str> = ip.split('.').collect();
    
    if ip_parts.len() == 4 {
        if ip_parts[0] == "198" && (ip_parts[1] == "18" || ip_parts[1] == "19") {
            return "代理/VPN 网卡 (Clash/TUN)".to_string();
        }
        if ip_parts[0] == "169" && ip_parts[1] == "254" {
            return "本地链路 (无网络)".to_string();
        }
    }

    if name_lower.contains("wi-fi") || name_lower.contains("wifi") || name_lower == "en0" {
        "无线网卡 (Wi-Fi)".to_string()
    } else if name_lower.contains("ethernet") || name_lower.contains("以太网") || name_lower.contains("en") {
        "有线网卡 (Ethernet)".to_string()
    } else if name_lower.contains("local area connection*") || name_lower.contains("本地连接*") {
        "移动热点 (Mobile Hotspot)".to_string()
    } else if name_lower.contains("virtual") || name_lower.contains("vethernet") || name_lower.contains("wsl") {
        "虚拟网卡 (Virtual)".to_string()
    } else if name_lower.contains("vpn") || name_lower.contains("tun") || name_lower.contains("tap") || name_lower.contains("clash") {
        "代理/VPN 网卡".to_string()
    } else {
        format!("网卡 {}", name)
    }
}

fn ip_score(ip_str: &str, name: &str) -> i32 {
    let name_lower = name.to_lowercase();
    let parts: Vec<&str> = ip_str.split('.').collect();
    if parts.len() != 4 {
        return 0;
    }
    let p0: u8 = parts[0].parse().unwrap_or(0);
    let p1: u8 = parts[1].parse().unwrap_or(0);

    // 首先排除虚拟网卡和 VPN
    if name_lower.contains("virtual") || name_lower.contains("vethernet") || name_lower.contains("wsl") {
        return 20;
    }
    if name_lower.contains("vpn") || name_lower.contains("tun") || name_lower.contains("tap") || name_lower.contains("clash") {
        return 10;
    }

    match (p0, p1) {
        (192, 168) => {
            // 如果是 192.168.137.x，说明是 Windows 移动热点，排在常规物理局域网后面一些
            if p1 == 168 && parts[2] == "137" {
                75
            } else {
                100
            }
        }
        (10, _) => 90,
        (172, x) if (16..=31).contains(&x) => 80,
        (198, 18) | (198, 19) => 5, // Clash TUN
        (169, 254) => 1, // 链路本地
        (127, _) => 0,
        _ => 50,
    }
}

fn get_all_ips_sorted() -> Vec<IpInfo> {
    let mut ip_infos = Vec::new();
    if let Ok(ifaces) = get_if_addrs() {
        for iface in ifaces {
            if !iface.is_loopback() {
                if let IpAddr::V4(ipv4) = iface.addr.ip() {
                    let ip = ipv4.to_string();
                    let name = iface.name.clone();
                    let label = guess_interface_type(&name, &ip);
                    ip_infos.push(IpInfo { ip, name, label });
                }
            }
        }
    }
    
    // 按打分降序排列
    ip_infos.sort_by(|a, b| {
        let score_a = ip_score(&a.ip, &a.name);
        let score_b = ip_score(&b.ip, &b.name);
        score_b.cmp(&score_a)
    });

    if ip_infos.is_empty() {
        ip_infos.push(IpInfo {
            ip: "127.0.0.1".to_string(),
            name: "lo".to_string(),
            label: "本地回环".to_string(),
        });
    }
    ip_infos
}

fn generate_qr_svg(url: &str) -> Result<String, String> {
    let code = QrCode::new(url.as_bytes()).map_err(|e| e.to_string())?;
    let svg_xml = code.render::<qrcode::render::svg::Color>().build();
    let base64_data = general_purpose::STANDARD.encode(svg_xml);
    Ok(format!("data:image/svg+xml;base64,{}", base64_data))
}

fn type_text_via_clipboard(text: &str) {
    if let Ok(mut clipboard) = Clipboard::new() {
        if clipboard.set_text(text).is_ok() {
            // A short delay to allow macOS clipboard (pasteboard) to settle and sync
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            let mut enigo = Enigo::new();
            #[cfg(target_os = "macos")]
            {
                enigo.key_down(Key::Meta);
                std::thread::sleep(std::time::Duration::from_millis(50));
                enigo.key_click(Key::Layout('v'));
                std::thread::sleep(std::time::Duration::from_millis(50));
                enigo.key_up(Key::Meta);
            }
            #[cfg(not(target_os = "macos"))]
            {
                enigo.key_down(Key::Control);
                std::thread::sleep(std::time::Duration::from_millis(50));
                enigo.key_click(Key::Layout('v'));
                std::thread::sleep(std::time::Duration::from_millis(50));
                enigo.key_up(Key::Control);
            }
        }
    }
}

fn create_tray_icon(r: u8, g: u8, b: u8) -> tauri::image::Image<'static> {
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) * 4;
            let mut is_pixel_set = false;
            
            // T 的横杠 (x: 1-7, y: 3-4)
            if y >= 3 && y <= 4 && x >= 1 && x <= 7 {
                is_pixel_set = true;
            }
            // T 的竖杠 (x: 3-4, y: 3-12)
            if x >= 3 && x <= 4 && y >= 3 && y <= 12 {
                is_pixel_set = true;
            }
            // L 的竖杠 (x: 9-10, y: 3-12)
            if x >= 9 && x <= 10 && y >= 3 && y <= 12 {
                is_pixel_set = true;
            }
            // L 的横杠 (x: 9-14, y: 11-12)
            if y >= 11 && y <= 12 && x >= 9 && x <= 14 {
                is_pixel_set = true;
            }
            
            if is_pixel_set {
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 255;
            }
        }
    }
    tauri::image::Image::new_owned(rgba, size as u32, size as u32)
}

fn update_status_menu(app: &tauri::AppHandle, item: &MenuItem<tauri::Wry>, count: usize) {
    let text = if count > 0 {
        format!("状态: 已连接 ({})", count)
    } else {
        "状态: 等待连接".to_string()
    };
    let _ = item.set_text(text);

    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app_clone.tray_by_id("main_tray") {
            let icon = if count > 0 {
                create_tray_icon(22, 163, 74) // #16a34a Green
            } else {
                create_tray_icon(0x80, 0x80, 0x80) // #808080 Gray
            };
            let _ = tray.set_icon(Some(icon));
        }
    });
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn qr_handler() -> Html<&'static str> {
    Html(QR_HTML)
}

async fn index_or_ws_handler(
    ws_opt: Option<WebSocketUpgrade>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Some(ws) = ws_opt {
        ws.on_upgrade(|socket| handle_socket(socket, state))
    } else {
        Html(INDEX_HTML).into_response()
    }
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let count = state.client_count.fetch_add(1, Ordering::SeqCst) + 1;
    update_status_menu(&state.app_handle, &state.status_menu_item, count);

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(msg_type) = client_msg.get("type").and_then(|v| v.as_str()) {
                        if msg_type == "text" {
                            if let Some(payload) = client_msg.get("payload").and_then(|v| v.as_str()) {
                                let payload = payload.to_string();
                                let app_handle = state.app_handle.clone();
                                let _ = app_handle.run_on_main_thread(move || {
                                    type_text_via_clipboard(&payload);
                                });
                                let ack = serde_json::json!({
                                    "type": "ack",
                                    "payload": "ok"
                                });
                                let _ = socket.send(Message::Text(ack.to_string())).await;
                            }
                        } else if msg_type == "ping" {
                            let pong = serde_json::json!({
                                "type": "pong"
                            });
                            let _ = socket.send(Message::Text(pong.to_string())).await;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let count = state.client_count.fetch_sub(1, Ordering::SeqCst) - 1;
    update_status_menu(&state.app_handle, &state.status_menu_item, count);
}

#[derive(Serialize)]
struct QrResponse {
    url: String,
    #[serde(rename = "dataUrl")]
    data_url: String,
    connected: bool,
    #[serde(rename = "clientCount")]
    client_count: usize,
    #[serde(rename = "allIPs")]
    all_ips: Vec<IpInfo>,
}

async fn qr_api_handler(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<QrResponse> {
    let all_ips = get_all_ips_sorted();
    
    let selected_ip = params.get("ip")
        .filter(|ip| all_ips.iter().any(|info| &info.ip == *ip))
        .cloned()
        .unwrap_or_else(|| all_ips.first().map(|info| info.ip.clone()).unwrap_or_else(|| "127.0.0.1".to_string()));

    let url = format!("http://{}:9527", selected_ip);
    let data_url = generate_qr_svg(&url).unwrap_or_default();
    let client_count = state.client_count.load(Ordering::SeqCst);

    Json(QrResponse {
        url,
        data_url,
        connected: client_count > 0,
        client_count,
        all_ips,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let show_qr = MenuItem::with_id(app, "show_qr", "显示二维码", true, None::<&str>)?;
            let status = MenuItem::with_id(app, "status", "状态: 等待连接", false, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[
                &show_qr,
                &PredefinedMenuItem::separator(app)?,
                &status,
                &PredefinedMenuItem::separator(app)?,
                &quit,
            ])?;

            let _tray = TrayIconBuilder::with_id("main_tray")
                .icon(create_tray_icon(0x80, 0x80, 0x80))
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show_qr" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            // Start HTTP + WebSocket Server on port 9527
            let state = AppState {
                client_count: Arc::new(AtomicUsize::new(0)),
                status_menu_item: status,
                app_handle: app.handle().clone(),
            };

            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                let router = Router::new()
                    .route("/", get(index_or_ws_handler))
                    .route("/index.html", get(index_handler))
                    .route("/qr.html", get(qr_handler))
                    .route("/api/qr", get(qr_api_handler))
                    .with_state(state_clone);

                if let Ok(listener) = tokio::net::TcpListener::bind("0.0.0.0:9527").await {
                    let _ = axum::serve(listener, router).await;
                }
            });

            // Navigate window to local HTTP server
            let url = tauri::Url::parse("http://localhost:9527/qr.html").unwrap();
            let _ = window.navigate(url);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
// rebuild trigger
