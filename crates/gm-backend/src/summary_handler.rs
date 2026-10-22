//! summary_handler — Dashboard 聚合数据 (per [游戏C]_src gm_platform/server.js /gm/summary 移植)
//! 2026-09-01 actix-web 重写
//!
//! ## 2026-10-22 契约对齐 (per GM 控制台契约漂移审核)
//! - `playerStats` 此前**硬编码** 1000/487/500/13/32.5/47，与 `/gm/players` 实际生成的
//!   mock 玩家对不上（同一后端两屏数字互相打架）。现改为从同一份确定性 mock 计算。
//! - `activity.recentBroadcasts` 此前恒为 `[]`，Dashboard 的「Recent Broadcasts」永远空白。
//!   现从审计流反查。
//! - `support.open` 此前统计 `status != "resolved"`（含 pending），与标签「Open Tickets」
//!   及 Support 页的 open 口径不一致。现改为只数 `status == "open"`。

use actix_web::{web, HttpResponse};
use serde_json::json;

use crate::broadcast_handler::recent_broadcasts;
use crate::players_handler::mock_player_stats;
use crate::AppState;

/// GET /gm/summary — 一次拉取 Dashboard 全部数据 (per [游戏C]_src dashboard 风格)
pub async fn summary(state: web::Data<AppState>) -> HttpResponse {
    // 直接从 InMemory 聚合 (生产应调各 5 域 gRPC)
    let grants_count = state.grants.lock().unwrap().len() as u32;
    let broadcasts_count = state.audit_store.list_entries(1000).await.iter()
        .filter(|e| e.action == "broadcast").count() as u32;
    let recent = recent_broadcasts(&state, 10).await;
    let tickets = state.tickets.lock().unwrap();
    // 口径与 Support 页的 open 一致：只数 status == "open"
    let tickets_open = tickets.iter().filter(|t| t.status == "open").count() as u32;
    let tickets_total = tickets.len() as u32;
    drop(tickets);
    let mall_count = state.mall_items.lock().unwrap().len() as u32;
    let servers = state.servers.lock().unwrap();
    let total = servers.len() as u32;
    let running = servers.iter().filter(|s| s.status == "running").count() as u32;
    drop(servers);

    HttpResponse::Ok().json(json!({
        // 直接嵌入结构体：手拼 JSON 会把 f32 提升成 f64，averageLevel 的末位
        // 会与 /gm/players 的序列化结果对不上
        "playerStats": mock_player_stats(),
        "activity": {
            "totalBroadcasts": broadcasts_count,
            "recentBroadcasts": recent,
            "totalGrants": grants_count,
        },
        "support": {
            "open": tickets_open,
            "total": tickets_total,
        },
        "mall": {
            "totalItems": mall_count,
        },
        "servers": {
            "stats": { "total": total, "running": running },
            "list": state.servers.lock().unwrap().clone(),
        },
    }))
}
