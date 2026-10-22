//! gm-backend ↔ gm-console 前端契约一致性测试
//!
//! 2026-10-22 新增。起因是一次契约漂移审核：后端用 `#[derive(Serialize)]` 输出
//! snake_case 字段名，而前端（`tools/gm-console/frontend/src/pages/*.tsx`）按
//! camelCase 读取，两者之间没有 serde rename 规则 —— 共 13 处不一致，接线后
//! Players / Items / Support / Reports 会整块空白，Canvas 发送必然 400。
//!
//! 本测试把**前端实际读取的字段名**固化为断言，防止后端再漂回去。
//! 任何一处字段改名而这里没同步，测试立刻失败。
//!
//! 覆盖：players / items / support / reports / canvas / summary / servers / metrics

use actix_web::{test, web, App};
use gm_backend::{issue_jwt, register_routes, AppState, GmConfig, LoginRequest};
use serde_json::Value;

// ============================================================================
// helpers
// ============================================================================

fn test_state() -> AppState {
    let config = GmConfig::for_test("127.0.0.1:0", "127.0.0.1:0", "http://127.0.0.1:50055").unwrap();
    AppState::new(config)
}

/// 断言 JSON 对象里**没有**某个 snake_case 键（防止后端同时输出两套命名）
fn assert_no_snake_key(obj: &Value, keys: &[&str], what: &str) {
    if let Some(map) = obj.as_object() {
        for k in keys {
            assert!(
                !map.contains_key(*k),
                "{what} 不应再输出 snake_case 键 `{k}`（实际键: {:?}）",
                map.keys().collect::<Vec<_>>()
            );
        }
    }
}

fn obj_of(v: &Value) -> &serde_json::Map<String, Value> {
    v.as_object().unwrap_or_else(|| panic!("期望 JSON 对象，实际: {v}"))
}

// ============================================================================
// CT1: GET /gm/players —— 前端 Players.tsx 读取的每个字段
// ============================================================================

#[actix_web::test]
async fn ct_players_matches_frontend_interface() {
    let state = test_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/gm/players?page=1&pageSize=10")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body: Value = test::read_body_json(resp).await;

    let players = body["players"].as_array().expect("players 数组");
    assert!(!players.is_empty(), "mock 玩家不应为空");
    let p = &players[0];

    // Players.tsx 读取的字段
    assert!(p["name"].is_string(), "players[].name 应为字符串（前端读 player.name）：{p}");
    assert!(p["level"].is_number(), "players[].level 应为数字：{p}");
    assert!(p["vipLevel"].is_number(), "players[].vipLevel 应为数字（前端读 player.vipLevel）：{p}");
    assert!(p["totalSpend"].is_number(), "players[].totalSpend 应为数字：{p}");
    assert!(p["lastLogin"].is_string(), "players[].lastLogin 应为字符串：{p}");
    assert!(p["status"].is_string(), "players[].status 应为字符串：{p}");
    assert!(p.get("guild").is_some(), "players[] 应有 guild 键（可为 null）：{p}");
    assert!(p.get("region").is_some(), "players[] 应有 region 键（可为 null）：{p}");

    // 旧的 snake_case 必须消失
    assert_no_snake_key(
        p,
        &["display_name", "total_spent", "last_login", "vip_level"],
        "players[]",
    );

    // 前端的两张统计卡：stats.overall / stats.filtered
    let stats = obj_of(&body["stats"]);
    for key in ["overall", "filtered"] {
        let s = obj_of(stats.get(key).expect("stats 子键"));
        for field in ["total", "online", "offline", "banned", "averageLevel", "highValue"] {
            assert!(
                s.contains_key(field),
                "stats.{key} 应含 `{field}`（前端 Players.tsx 直接读它）"
            );
        }
    }
    assert!(
        !stats["overall"].as_object().unwrap().contains_key("average_level"),
        "stats.overall 不应再输出 average_level"
    );

    // 分页字段
    assert_eq!(body["page"], 1);
    assert_eq!(body["pageSize"], 10);
}

// ============================================================================
// CT2: 搜索 / 排序参数不再被静默忽略
// ============================================================================

#[actix_web::test]
async fn ct_players_honours_search_and_sort_params() {
    let state = test_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    // sortBy=level&sortOrder=asc → 首个玩家的 level 应为全表最小
    let req = test::TestRequest::get()
        .uri("/gm/players?page=1&pageSize=5&sortBy=level&sortOrder=asc")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let players = body["players"].as_array().unwrap();
    let first = players[0]["level"].as_u64().unwrap();
    for p in players.iter() {
        let lv = p["level"].as_u64().unwrap();
        assert!(lv >= first, "升序排序后后续玩家 level 不应小于首个");
    }

    // sortOrder=desc → 首个应为全表最大
    let req = test::TestRequest::get()
        .uri("/gm/players?page=1&pageSize=5&sortBy=level&sortOrder=desc")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let players = body["players"].as_array().unwrap();
    let first = players[0]["level"].as_u64().unwrap();
    for p in players.iter() {
        let lv = p["level"].as_u64().unwrap();
        assert!(lv <= first, "降序排序后后续玩家 level 不应大于首个");
    }

    // search 命中公会（前端搜索框写「Search by ID, name or guild」）
    let req = test::TestRequest::get()
        .uri("/gm/players?search=Valkyrie&pageSize=50")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let players = body["players"].as_array().unwrap();
    assert!(!players.is_empty(), "公会搜索应命中数据");
    for p in players {
        assert_eq!(
            p["guild"], "Valkyrie",
            "search=Valkyrie 的结果应全部属于 Valkyrie 公会"
        );
    }
}

// ============================================================================
// CT3: GET /gm/items/grants —— 前端 Items.tsx 读取的字段
// ============================================================================

#[actix_web::test]
async fn ct_grants_match_frontend_interface() {
    let state = test_state();
    state.ensure_default_admin().await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    let token = issue_jwt("test-secret", "admin", vec!["GM_WRITE".to_string()], 3600).unwrap();

    // 先发一条，才能断言列表字段
    let req = test::TestRequest::post()
        .uri("/gm/items/grant")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "player_id": "player-100200",
            "item_id": "item_gem_600",
            "amount": 3,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "道具发放应成功");

    let req = test::TestRequest::get().uri("/gm/items/grants").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let grants = body["grants"].as_array().expect("grants 数组");
    assert!(!grants.is_empty());
    let g = &grants[0];
    assert_eq!(g["playerId"], "player-100200", "前端读 grant.playerId");
    assert_eq!(g["itemId"], "item_gem_600", "前端读 grant.itemId");
    assert!(g["createdAt"].is_string(), "前端读 grant.createdAt");
    assert_no_snake_key(g, &["player_id", "item_id", "created_at"], "grants[]");
}

// ============================================================================
// CT4: GET /gm/support/tickets —— 前端 Support.tsx 读取的字段
// ============================================================================

#[actix_web::test]
async fn ct_tickets_match_frontend_interface() {
    let state = test_state();
    state.ensure_default_admin().await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    let token = issue_jwt("test-secret", "admin", vec!["GM_WRITE".to_string()], 3600).unwrap();

    let req = test::TestRequest::post()
        .uri("/gm/support")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "player_id": "player-100200",
            "message": "contract test ticket",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "建单应成功");

    let req = test::TestRequest::get().uri("/gm/support/tickets").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let tickets = body["tickets"].as_array().expect("tickets 数组");
    assert!(!tickets.is_empty());
    let t = &tickets[0];
    assert_eq!(t["playerId"], "player-100200", "前端读 ticket.playerId");
    assert!(t["createdAt"].is_string(), "前端读 ticket.createdAt");
    assert!(t["updatedAt"].is_string(), "前端读 ticket.updatedAt");
    assert!(t["status"].is_string(), "前端读 ticket.status");
    assert_no_snake_key(t, &["player_id", "created_at", "updated_at"], "tickets[]");
}

// ============================================================================
// CT5: GET /gm/reports —— 前端 Reports.tsx 的鸭子类型回退链
// ============================================================================

#[actix_web::test]
async fn ct_reports_match_frontend_interface() {
    let state = test_state();
    // 报表 mock 在 main.rs 启动时 seed，测试里需手动 seed
    gm_backend::reports_handler::seed_reports(&state);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/gm/reports").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let reports = body["reports"].as_array().expect("reports 数组");
    assert!(!reports.is_empty());
    let r = &reports[0];

    // Reports.tsx: type → category → reportType
    assert!(r["type"].is_string(), "应有 `type` 键（前端类型回退链第一顺位）");
    // Reports.tsx: generatedAt → createdAt → updatedAt
    assert!(r["generatedAt"].is_string(), "应有 `generatedAt` 键");
    // Reports.tsx: summary → metrics 前 3 项
    assert!(r["summary"].is_string(), "应有 `summary` 键");
    assert!(r["metrics"].is_object(), "应有 `metrics` 对象（前端据此渲染聚合卡片）");
    assert!(
        !r["metrics"].as_object().unwrap().is_empty(),
        "metrics 不应为空对象，否则聚合卡片无内容"
    );
    assert!(r["period"].is_string(), "应有 `period` 键");
    assert_no_snake_key(r, &["generated_at", "kind"], "reports[]");
}

// ============================================================================
// CT6: Canvas —— 接受前端实际发送的嵌套 payload + 锚点为字符串数组
// ============================================================================

#[actix_web::test]
async fn ct_canvas_accepts_frontend_payload() {
    let state = test_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    // 锚点清单：前端 Canvas.tsx 期望 anchors: string[]
    let req = test::TestRequest::get().uri("/gm/canvas/anchors").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let anchors = body["anchors"].as_array().expect("anchors 数组");
    assert!(!anchors.is_empty());
    for a in anchors {
        assert!(
            a.is_string(),
            "anchors 元素应为字符串（前端对它做 value.split('_')）"
        );
    }

    // 前端 Canvas.tsx 的真实请求体（见其 send() 里的 instruction 构造）
    let payload = serde_json::json!({
        "clientId": "client-7f21",
        "instruction": {
            "layer": "gm_overlay",
            "width": 640,
            "height": 360,
            "rgbaBase64": "",
            "opacity": 1.0,
            "scale": 1.0,
            "zIndex": 10000,
            "clear": true,
            "position": { "x": 0.5, "y": 0.5, "anchor": "center" },
            "durationMs": 3000
        }
    });
    let req = test::TestRequest::post()
        .uri("/gm/canvas/send")
        .set_json(payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "前端形状的 Canvas 载荷必须被接受（此前因 anchor 缺失必然 400）"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "queued");
    assert_eq!(body["command"]["width"], 640);
    assert_eq!(body["command"]["height"], 360);
    assert_eq!(body["command"]["clear"], true);
}

// ============================================================================
// CT7: /gm/summary —— 与 /gm/players 同源 + 广播历史非空 + open 口径
// ============================================================================

#[actix_web::test]
async fn ct_summary_is_consistent_with_players_and_lists_broadcasts() {
    let state = test_state();
    state.ensure_default_admin().await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    let token = issue_jwt("test-secret", "admin", vec!["GM_WRITE".to_string()], 3600).unwrap();

    // 播一条广播，随后 summary.recentBroadcasts 应非空
    let req = test::TestRequest::post()
        .uri("/gm/broadcast")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(serde_json::json!({ "message": "contract test broadcast" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let req = test::TestRequest::get().uri("/gm/summary").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    // 广播历史不再恒为 []
    let recent = body["activity"]["recentBroadcasts"].as_array().expect("recentBroadcasts");
    assert!(!recent.is_empty(), "summary.recentBroadcasts 不应为空数组");
    assert_eq!(recent[0]["message"], "contract test broadcast");
    assert!(
        recent[0]["createdAt"].is_string(),
        "SSE/广播条目的时间键应为 createdAt（前端读 entry.createdAt）"
    );
    assert_no_snake_key(&recent[0], &["created_at"], "recentBroadcasts[]");

    // playerStats 与 /gm/players 的 stats.overall 同源
    let req = test::TestRequest::get().uri("/gm/players?pageSize=1").to_request();
    let resp = test::call_service(&app, req).await;
    let players_body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["playerStats"]["total"], players_body["stats"]["overall"]["total"],
        "summary.playerStats 与 /gm/players 的 overall 必须一致（此前一边硬编码一边随机）"
    );
    assert_eq!(
        body["playerStats"]["averageLevel"], players_body["stats"]["overall"]["averageLevel"],
        "summary.playerStats.averageLevel 必须与 /gm/players 一致"
    );
}

// ============================================================================
// CT8: /gm/servers camelCase + /gm/metrics 文本格式
// ============================================================================

#[actix_web::test]
async fn ct_servers_and_metrics_match_frontend_expectations() {
    let state = test_state();
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/gm/servers").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let servers = body["servers"].as_array().expect("servers 数组");
    assert!(!servers.is_empty());
    let s = &servers[0];
    assert!(s["onlinePlayers"].is_number(), "前端读 server.onlinePlayers");
    assert!(s.get("lastUpdated").is_some(), "前端读 server.lastUpdated");
    assert_no_snake_key(s, &["online_players", "last_updated"], "servers[]");

    // ActiveUsersChart.tsx 用 /online_connections\s+(\d+)/ 抓数
    let req = test::TestRequest::get().uri("/gm/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    let text = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
    assert!(
        text.contains("online_connections "),
        "metrics 应含 online_connections，前端正则依赖它: {text}"
    );
    assert!(text.contains("running_servers "), "metrics 应含 running_servers");
    assert!(text.contains("total_servers "), "metrics 应含 total_servers");

    // 抓到的值应等于分片在线人数之和
    let mut sum = 0u64;
    for srv in servers {
        sum += srv["onlinePlayers"].as_u64().unwrap_or(0);
    }
    let scraped: u64 = text
        .lines()
        .find_map(|l| l.strip_prefix("online_connections "))
        .and_then(|v| v.trim().parse().ok())
        .expect("online_connections 数值");
    assert_eq!(scraped, sum, "online_connections 应等于分片 online_players 之和");
}

// ============================================================================
// CT9: /gm/login 契约（前端 Login.tsx 读取 token）
// ============================================================================

#[actix_web::test]
async fn ct_login_returns_token_field() {
    let state = test_state();
    state.ensure_default_admin().await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state))
            .configure(register_routes),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/gm/login")
        .set_json(LoginRequest {
            username: "admin".to_string(),
            password: "adminpass".to_string(),
        })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body: Value = test::read_body_json(resp).await;
    assert!(body["token"].is_string(), "前端读 res.data.token");
}
