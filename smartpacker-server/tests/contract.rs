//! 服务契约测试:对齐参考 `api.py` 的路由与响应结构(API 文档 `api.md`)。
//!
//! 用 tower `oneshot` 直接驱动 `app()` 路由器,无需真实端口。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use smartpacker_server::app;
use tower::ServiceExt;

/// 发送 POST 请求并解析 JSON 响应。
async fn post_json(app: &Router, path: &str, body: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!(
            "response must be valid JSON in test at {path}: {e}: {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, value)
}

/// 发送 GET 请求。
async fn get_json(app: &Router, path: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap();
    (status, value)
}

/// 发送 GET 请求返回原始字符串(用于欢迎页)。
async fn get_text(app: &Router, path: &str) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn get_root_serves_welcome() {
    let (status, text) = get_text(&app(), "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(text.contains("welcome"), "root must greet: {text}");
}

#[tokio::test]
async fn post_get_all_data_returns_embedded_samples() {
    let (_, v) = post_json(&app(), "/getAllData", "{}").await;
    assert_eq!(v["Success"], json!(true));
    // box 4 项、item 为 21 个分组、首个分组名称固定。
    assert_eq!(v["box"].as_array().unwrap().len(), 4);
    assert_eq!(v["item"].as_array().unwrap().len(), 21);
    assert_eq!(v["item"][0]["name"], "YM-7002-2 沖孔機(1)");
}

#[tokio::test]
async fn get_get_all_data_rejected() {
    let (_, v) = get_json(&app(), "/getAllData").await;
    assert_eq!(v["Success"], json!(false));
    assert_eq!(v["Reason"], "can't use GET");
}

#[tokio::test]
async fn cal_packing_missing_binding_rejected() {
    let body =
        r#"{"box":[{"name":"b","WHD":[10,10,10],"weight":9,"openTop":[1],"coner":0}],"item":[]}"#;
    let (_, v) = post_json(&app(), "/calPacking", body).await;
    assert_eq!(v["Success"], json!(false));
    assert_eq!(v["Reason"], "box or item not in input data");
}

#[tokio::test]
async fn cal_packing_invalid_json_rejected() {
    let (_, v) = post_json(&app(), "/calPacking", "not json at all").await;
    assert_eq!(v["Success"], json!(false));
    assert_eq!(v["Reason"], "input data err");
}

#[tokio::test]
async fn cal_packing_empty_arrays_input_err() {
    let (_, v) = post_json(
        &app(),
        "/calPacking",
        r#"{"box":[],"item":[],"binding":[]}"#,
    )
    .await;
    assert_eq!(v["Success"], json!(false));
    assert_eq!(v["Reason"], "input data err");
}

#[tokio::test]
async fn get_cal_packing_method_not_post() {
    let (_, v) = get_json(&app(), "/calPacking").await;
    assert_eq!(v["Success"], json!(false));
    assert_eq!(v["Reason"], "method not POST");
}

/// api.md 的 40呎超高货柜入参,原样保留(用于算法形状校验,非位置契约)。
#[cfg(test)]
const CAL_40FT_BODY: &str = r#"{
    "box": [
        {
            "name": "40呎超高貨櫃",
            "WHD": [1203,235,269],
            "weight": 26280,
            "openTop": [1,2],
            "coner": 15
        }
    ],
    "item": [
        {"name":"Dyson_DC34_Animal","WHD":[170,82,46],"count":5,"updown":1,"type":1,"level":0,"loadbear":100,"weight":85,"color":1},
        {"name":"Panasonic_NA-V160GBS","WHD":[85,60,60],"count":18,"updown":1,"type":1,"level":0,"loadbear":100,"weight":30,"color":2},
        {"name":"Superlux_RS921","WHD":[60,80,200],"count":15,"updown":1,"type":1,"level":0,"loadbear":10,"weight":30,"color":3},
        {"name":"Dell_R740","WHD":[70,100,30],"count":30,"updown":1,"type":1,"level":0,"loadbear":100,"weight":20,"color":4},
        {"name":"50_Gal_Oil_Drum","WHD":[80,80,120],"count":20,"updown":0,"type":2,"level":0,"loadbear":50,"weight":170,"color":5},
        {"name":"Moving_Box","WHD":[60,40,50],"count":25,"updown":1,"type":1,"level":0,"loadbear":40,"weight":30,"color":6},
        {"name":"Wood_Table","WHD":[152,152,75],"count":2,"updown":1,"type":1,"level":0,"loadbear":50,"weight":70,"color":7}
    ],
    "binding": [
        ["Wood_Table", "50_Gal_Oil_Drum"]
    ]
}"#;

#[tokio::test]
async fn cal_packing_40ft_contract() {
    let (status, v) = post_json(&app(), "/calPacking", CAL_40FT_BODY).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["Success"], json!(true), "should succeed: {v}");
    assert!(v.get("Reason").is_none());

    let data = &v["data"];
    let bins = data["box"].as_array().unwrap();
    assert_eq!(bins.len(), 1);

    // 箱子出参:名称、布局、中心、承重、重力分布。
    let b = &bins[0];
    assert_eq!(b["partNumber"], "40呎超高貨櫃");
    assert_eq!(b["WHD"], json!([1203, 235, 269]));
    assert_eq!(b["position"], json!([601.5, 117.5, 134.5]));
    assert_eq!(b["weight"], json!(26280));
    let gravity = b["gravity"].as_array().unwrap();
    assert_eq!(gravity.len(), 4, "gravity must be 4 floats");

    // 8 个角件:15x15x15、黑色、零重、rotationType 0,精确断言中心。
    let fit = data["fitItem"].as_array().unwrap();
    let corners: Vec<&Value> = fit.iter().filter(|it| it["name"] == "corner").collect();
    assert_eq!(corners.len(), 8, "corner>0 must emit 8 corner items");
    for c in &corners {
        assert_eq!(c["WHD"], json!([15, 15, 15]));
        assert_eq!(c["color"], "#000000");
        assert_eq!(c["weight"], json!(0));
        assert_eq!(c["rotationType"], json!(0));
        assert_eq!(c["type"], "cube");
    }
    let corner0 = corners
        .iter()
        .find(|c| c["partNumber"] == "corner0")
        .unwrap();
    let corner7 = corners
        .iter()
        .find(|c| c["partNumber"] == "corner7")
        .unwrap();
    assert_eq!(corner0["position"], json!([7, 7, 7]));
    assert_eq!(corner7["position"], json!([1195, 227, 261]));

    // 非角件物品结构约定。
    for it in fit.iter().filter(|it| it["name"] != "corner") {
        assert!(it["position"].is_array());
        assert!(it["WHD"].is_array());
        for e in it["position"].as_array().unwrap() {
            assert_eq!(
                e.as_i64(),
                e.as_f64().map(|x| x as i64),
                "positions are integers"
            );
        }
        let rot = it["rotationType"].as_i64().unwrap();
        assert!((0..=5).contains(&rot), "rotationType in 0..=5, got {rot}");
        let ty = it["type"].as_str().unwrap();
        assert!(
            ty == "cube" || ty == "cylinder",
            "type must be cube/cylinder"
        );
        // partNumber = "<name>-<n>"
        let pn = it["partNumber"].as_str().unwrap();
        let nm = it["name"].as_str().unwrap();
        assert!(
            pn.starts_with(&format!("{nm}-")),
            "{pn} must start with {nm}-"
        );
    }

    // unfitItem 结构同 fitItem。
    for it in data["unfitItem"].as_array().unwrap() {
        assert!(it["type"].is_string());
        assert!(it["position"].is_array());
    }

    // 普通物品精确位置不做断言(算法输出,非契约)。
}
