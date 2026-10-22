//! reports_handler — 报表 (per [游戏C]_src gm_platform/modules/analytics 移植)
//! 2026-09-01 actix-web 重写

use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportEntry {
    pub id: String,
    pub title: String,
    /// 对外字段名为 `type`（前端 `Reports.tsx` 按 `type → category → reportType` 回退取值）
    #[serde(rename = "type")]
    pub kind: String,
    pub period: String,
    pub value: f64,
    pub generated_at: String,
    /// 摘要文案：前端 Highlights 列优先取 `summary`，缺失时退回 metrics 前 3 项
    pub summary: String,
    /// 指标集合：前端用它渲染 Highlights 回退值与聚合卡片
    pub metrics: BTreeMap<String, f64>,
}

/// 启动时填充 mock 报表 (5 类 × 14 天)
pub fn seed_reports(state: &AppState) {
    let mut reports = state.reports.lock().unwrap();
    let kinds = ["revenue", "dau", "conversion", "churn"];
    for kind in kinds {
        for d in 0..14 {
            let date = Utc::now() - chrono::Duration::days(d);
            let value: f64 = match kind {
                "revenue" => 10000.0 + (d as f64 * 200.0) + rand::thread_rng().gen_range(-500.0..500.0),
                "dau" => 5000.0 + rand::thread_rng().gen_range(-300.0..300.0),
                "conversion" => 0.05 + rand::thread_rng().gen_range(-0.01..0.01),
                "churn" => 0.02 + rand::thread_rng().gen_range(-0.005..0.005),
                _ => 0.0,
            };
            let value = (value * 100.0).round() / 100.0;
            let mut metrics = BTreeMap::new();
            metrics.insert("value".to_string(), value);
            reports.push(ReportEntry {
                id: format!("{}-{}", kind, date.format("%Y%m%d")),
                title: format!("{} report", kind),
                kind: kind.to_string(),
                period: date.format("%Y-%m-%d").to_string(),
                value,
                generated_at: date.to_rfc3339(),
                summary: format!("{} · {}", kind, value),
                metrics,
            });
        }
    }
}

pub async fn list_reports(state: web::Data<AppState>) -> HttpResponse {
    let reports = state.reports.lock().unwrap();
    HttpResponse::Ok().json(json!({"reports": reports.clone()}))
}
