//! canvas_handler — Canvas 画布指令 (per [游戏C]_src gm_platform/modules/canvas 移植)
//! 2026-09-01 actix-web 重写, mock 转发 (生产应接 match-service / game-server)
//!
//! ## 2026-10-22 契约对齐 (per GM 控制台契约漂移审核)
//! 此前后端接收**扁平** DTO `{layer, anchor, width, height, image_base64, clear}`，
//! 而前端 `Canvas.tsx` 发的是**嵌套**体
//! `{clientId, instruction:{layer, width, height, rgbaBase64, opacity, scale, zIndex, clear, position:{x,y,anchor}, durationMs?}}`，
//! 两者形状不兼容 → `anchor` 缺失导致 serde 提取失败，Canvas 发送在真实环境下必然 400。
//! 现按前端实际发送的形状接收。

use actix_web::{web, HttpResponse};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

pub const ALLOWED_ANCHORS: &[&str] = &[
    "top_left", "top_center", "top_right",
    "center_left", "center", "center_right",
    "bottom_left", "bottom_center", "bottom_right",
];

/// RGBA payload 上限：10 MiB（Base64 文本长度）
const MAX_BASE64_LEN: usize = 10 * 1024 * 1024;

fn default_opacity() -> f32 { 1.0 }
fn default_scale() -> f32 { 1.0 }
fn default_z_index() -> i64 { 10_000 }

/// 前端 `Canvas.tsx` 的请求体（camelCase）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasCommandRequest {
    pub client_id: String,
    pub instruction: CanvasInstruction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasInstruction {
    pub layer: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub rgba_base64: String,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_z_index")]
    pub z_index: i64,
    #[serde(default)]
    pub clear: bool,
    pub position: CanvasPosition,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasPosition {
    pub x: f32,
    pub y: f32,
    pub anchor: String,
}

/// 锚点清单：前端 `Canvas.tsx` 期望 `anchors: string[]`（它自己按 `_` 分词生成 label）
pub async fn list_anchors(_state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(json!({"anchors": ALLOWED_ANCHORS}))
}

pub async fn send_canvas_command(
    _state: web::Data<AppState>,
    body: web::Json<CanvasCommandRequest>,
) -> HttpResponse {
    // actix-web 4 的 `web::Json<T>` 解引用到 T
    let req: &CanvasCommandRequest = &body;

    if req.client_id.trim().is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "missing_client_id",
            "message": "clientId is required",
        }));
    }
    if !ALLOWED_ANCHORS.contains(&req.instruction.position.anchor.as_str()) {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_anchor",
            "message": format!("anchor must be one of {:?}", ALLOWED_ANCHORS),
        }));
    }
    if req.instruction.width == 0 || req.instruction.height == 0 {
        return HttpResponse::BadRequest().json(json!({"error": "invalid_dimensions"}));
    }
    // 校验 base64 长度 (简单 sanity check)
    let b64 = req.instruction.rgba_base64.trim();
    if !b64.is_empty() {
        if b64.len() > MAX_BASE64_LEN {
            return HttpResponse::BadRequest().json(json!({
                "error": "image_too_large",
                "max_bytes": MAX_BASE64_LEN,
            }));
        }
        // 验证可解析
        let _ = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|_| actix_web::error::ErrorBadRequest("invalid_base64"));
    }

    // mock 转发成功
    HttpResponse::Ok().json(json!({
        "status": "queued",
        "command": {
            "clientId": req.client_id,
            "layer": req.instruction.layer,
            "anchor": req.instruction.position.anchor,
            "width": req.instruction.width,
            "height": req.instruction.height,
            "clear": req.instruction.clear,
            "opacity": req.instruction.opacity,
            "scale": req.instruction.scale,
            "zIndex": req.instruction.z_index,
            "position": {
                "x": req.instruction.position.x,
                "y": req.instruction.position.y,
            },
            "durationMs": req.instruction.duration_ms,
        }
    }))
}
