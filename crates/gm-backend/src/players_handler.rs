//! players_handler — 玩家列表 + 统计 (per [游戏C]_src gm_platform/modules/player 移植)
//! 2026-09-01 actix-web 重写, 内存 mock 数据 (生产应接 player-service gRPC)
//!
//! ## 2026-10-22 契约对齐 (per GM 控制台契约漂移审核)
//! - 响应结构体统一 `camelCase`（前端 `Players.tsx` 按 camelCase 读取）。
//!   **请求** DTO 仍用 snake_case（前端 `Players.tsx` 发 snake_case 查询参数）。
//! - `display_name` 对外序列化为 `name`（前端读 `player.name`）。
//! - 补齐前端表格用到的 `vipLevel` / `guild` / `region`。
//! - 响应补 `stats.overall` / `stats.filtered`（前端两张统计卡此前因缺该字段整块不渲染）。
//! - 补 `sortBy` / `sortOrder` 查询参数（此前被静默忽略，四个排序下拉对真实后端无效）。
//! - 搜索支持公会（前端搜索框写「Search by ID, name or guild」，后端此前只匹配 name/id）。
//! - mock 玩家改为**确定性生成**：此前每次请求重新随机，分页时行会漂移，且
//!   `/gm/summary` 的统计与 `/gm/players` 的列表对不上。

use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::AppState;

/// mock 玩家总数（与 /gm/summary 的统计口径一致）
pub const MOCK_PLAYER_COUNT: usize = 1000;
/// 确定性 seed：保证多次请求、跨接口看到同一批玩家
const MOCK_SEED: u64 = 0x5247_5300; // "RGS\0"

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
    /// 排序键：lastLogin | level | name | totalSpend（与前端下拉取值一致）
    #[serde(default)]
    pub sort_by: Option<String>,
    /// 排序方向：asc | desc
    #[serde(default)]
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEntry {
    pub id: String,
    /// 对外字段名为 `name`（前端读 `player.name`）
    #[serde(rename = "name")]
    pub display_name: String,
    pub status: String,
    pub level: u32,
    pub vip_level: u32,
    pub total_spend: f64,
    pub last_login: String,
    pub guild: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStatsResponse {
    pub total: u32,
    pub online: u32,
    pub offline: u32,
    pub banned: u32,
    /// 用 f64 而非 f32：`json!` 会把 f32 提升成 f64（Value 只存 f64），
    /// 而直接序列化保留 f32，两者末位会不一致（40.3 vs 40.29999923706055）。
    pub average_level: f64,
    pub high_value: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersStats {
    /// 全量统计（不受筛选影响）
    pub overall: PlayerStatsResponse,
    /// 筛选后统计
    pub filtered: PlayerStatsResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersResponse {
    pub players: Vec<PlayerEntry>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
    pub stats: PlayersStats,
}

/// 生成 mock 玩家数据 (per [游戏C]_src — 无外部数据源时使用)
///
/// 确定性：同一 seed 恒产出同一批数据。分页、统计、summary 必须看到同一批玩家，
/// 否则列表翻页时行会漂移，且 `/gm/summary` 与 `/gm/players` 的数字互相打架。
fn generate_mock_players(count: usize) -> Vec<PlayerEntry> {
    let mut rng = StdRng::seed_from_u64(MOCK_SEED);
    let statuses = ["online", "offline", "banned"];
    let guilds = ["蒼嵐", "Valkyrie", "Iron Pact"];
    let regions = ["ap-northeast-1", "ap-southeast-1"];
    (0..count)
        .map(|i| {
            let status = statuses[rng.gen_range(0..statuses.len())];
            let guild = if rng.gen_bool(0.6) {
                Some(guilds[rng.gen_range(0..guilds.len())].to_string())
            } else {
                None
            };
            let region = regions[rng.gen_range(0..regions.len())].to_string();
            PlayerEntry {
                id: format!("player-{:06}", 100000 + i),
                display_name: format!("Player_{:04}", i),
                status: status.to_string(),
                level: rng.gen_range(1..80),
                vip_level: rng.gen_range(0..8),
                total_spend: rng.gen_range(0.0..5000.0),
                last_login: (Utc::now() - chrono::Duration::seconds(rng.gen_range(0..86400 * 30)))
                    .to_rfc3339(),
                guild,
                region: Some(region),
            }
        })
        .collect()
}

/// 高价值玩家阈值：total_spend > 1000.0（与 /gm/summary 统计口径一致）
const HIGH_VALUE_THRESHOLD: f64 = 1000.0;

fn stats_of(list: &[PlayerEntry]) -> PlayerStatsResponse {
    let mut online = 0u32;
    let mut offline = 0u32;
    let mut banned = 0u32;
    let mut high_value = 0u32;
    let mut level_sum: u32 = 0;
    for p in list {
        match p.status.as_str() {
            "online" => online += 1,
            "offline" => offline += 1,
            "banned" => banned += 1,
            _ => {}
        }
        level_sum += p.level;
        if p.total_spend > HIGH_VALUE_THRESHOLD {
            high_value += 1;
        }
    }
    let total = list.len() as u32;
    PlayerStatsResponse {
        total,
        online,
        offline,
        banned,
        // 与 mock_player_stats_json() 保持同一精度，否则 /gm/summary 与
        // /gm/players 的 averageLevel 会出现末位差异
        average_level: if total > 0 {
            ((level_sum as f64 / total as f64) * 10.0).round() / 10.0
        } else {
            0.0
        },
        high_value,
    }
}

fn sort_players(list: &mut [PlayerEntry], sort_by: &str, order: &str) {
    let desc = !order.eq_ignore_ascii_case("asc");
    match sort_by {
        "level" => list.sort_by(|a, b| a.level.cmp(&b.level)),
        "name" => list.sort_by(|a, b| a.display_name.cmp(&b.display_name)),
        "totalSpend" => {
            list.sort_by(|a, b| {
                a.total_spend
                    .partial_cmp(&b.total_spend)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }
        // 默认 lastLogin
        _ => list.sort_by(|a, b| a.last_login.cmp(&b.last_login)),
    }
    if desc {
        list.reverse();
    }
}

pub async fn list_players(
    _state: web::Data<AppState>,
    q: web::Query<PlayersQuery>,
) -> HttpResponse {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 200);
    let search = q.search.as_deref().unwrap_or("").to_lowercase();
    let status_filter = q.status.as_deref().unwrap_or("");
    let sort_by = q.sort_by.as_deref().unwrap_or("lastLogin");
    let sort_order = q.sort_order.as_deref().unwrap_or("desc");

    let all = generate_mock_players(MOCK_PLAYER_COUNT);
    let overall = stats_of(&all);

    let mut filtered: Vec<_> = all
        .into_iter()
        .filter(|p| {
            (search.is_empty()
                || p.display_name.to_lowercase().contains(&search)
                || p.id.contains(&search)
                || p.guild
                    .as_deref()
                    .map(|g| g.to_lowercase().contains(&search))
                    .unwrap_or(false))
                && (status_filter.is_empty() || p.status == status_filter)
        })
        .collect();

    let filtered_stats = stats_of(&filtered);
    sort_players(&mut filtered, sort_by, sort_order);

    let total = filtered.len() as u32;
    let start = ((page - 1) * page_size) as usize;
    let players: Vec<_> = filtered.into_iter().skip(start).take(page_size as usize).collect();

    HttpResponse::Ok().json(PlayersResponse {
        players,
        total,
        page,
        page_size,
        stats: PlayersStats {
            overall,
            filtered: filtered_stats,
        },
    })
}

/// 全量玩家统计（供 /gm/summary 复用，避免两处口径漂移）
pub fn mock_player_stats() -> PlayerStatsResponse {
    stats_of(&generate_mock_players(MOCK_PLAYER_COUNT))
}
