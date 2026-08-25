//! smartpacker-server:基于 `smartpacker` 装箱库的 HTTP 服务(应用需求驱动,
//! 不再保证与参考实现 api.py 行为兼容)。
//!
//! 路由:
//! - `GET /` — 服务横幅
//! - `POST /getAllData` — 返回内嵌示例数据(含 `Success: true`);`GET` 被拒绝
//! - `POST /calPacking` — 对提交的 box/item/binding 执行装箱并返回结果;`GET` 被拒绝
//!
//! 物品 JSON 支持可选字段 `allowed_float_ratio`(0..=1,缺省 0.25):该货物允许
//! 底面悬空的面积占比;算法按件校验底部支撑(支撑比 ≥ 1−allowed,或底面四角落实)。
//!
//! 所有路由均启用 permissive CORS。监听地址由 `SMARTPACKER_ADDR` 覆盖,默认
//! `0.0.0.0:5050`。

use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};
use tower_http::cors::CorsLayer;

use serde_json::{json, Value};

/// 内嵌的示例数据(`widadvance.json`,与 api.py 启动时读取的文件一致)。
const WID_ADVANCE: &str = include_str!("../data/widadvance.json");

/// 组装应用路由。
pub fn app() -> Router {
    Router::new()
        .route("/", get(hello))
        .route("/getAllData", get(get_all_data_get).post(get_all_data_post))
        .route("/calPacking", get(cal_packing_get).post(cal_packing))
        .layer(CorsLayer::permissive())
}

/// `GET /`:服务横幅。
async fn hello() -> Html<&'static str> {
    Html(
        "<html><body><h1>welcome to 3D packing prob API_1.1</h1>\
         <p>POST /calPacking with { box, item, binding } to compute a packing plan.</p>\
         </body></html>",
    )
}

/// `GET /getAllData`:拒绝读取。
async fn get_all_data_get() -> Json<Value> {
    Json(json!({ "Success": false, "Reason": "can't use GET" }))
}

/// `POST /getAllData`:返回内嵌示例数据并标记成功。
async fn get_all_data_post() -> Json<Value> {
    let mut data: Value = serde_json::from_str(WID_ADVANCE).expect("widadvance.json is valid JSON");
    data["Success"] = Value::Bool(true);
    Json(data)
}

/// `GET /calPacking`:拒绝读取。
async fn cal_packing_get() -> Json<Value> {
    Json(json!({ "Success": false, "Reason": "method not POST" }))
}

/// `POST /calPacking`:执行装箱。
///
/// 错误映射顺序(保留历史兼容):
/// 1. 请求体非法 JSON → `input data err`
/// 2. 缺 box/item/binding 任一 → `box or item not in input data`
/// 3. 构造箱/物品失败 → `input data err`
/// 4. 装箱或出参构造panic → `cal packing err`
async fn cal_packing(body: String) -> Json<Value> {
    let mut res = json!({ "Success": false });

    let input: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            res["Reason"] = Value::String("input data err".into());
            return Json(res);
        }
    };

    let has_keys =
        input.get("box").is_some() && input.get("item").is_some() && input.get("binding").is_some();
    if !has_keys {
        res["Reason"] = Value::String("box or item not in input data".into());
        return Json(res);
    }

    let (mut packer, binding) = match build_input(&input) {
        Ok(x) => x,
        Err(()) => {
            res["Reason"] = Value::String("input data err".into());
            return Json(res);
        }
    };

    let packed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        packer.pack(&PackOptions {
            bigger_first: true,
            distribute_items: false,
            fix_point: true,
            number_of_decimals: 0,
            binding,
            ..PackOptions::default()
        });
        let bin = &packer.bins[0];
        let fit_item: Vec<Value> = bin.items.iter().map(make_dict_item).collect();
        let unfit_item: Vec<Value> = bin.unfitted_items.iter().map(make_dict_item).collect();
        json!({
            "box": json!([make_dict_box(bin)]),
            "fitItem": fit_item,
            "unfitItem": unfit_item,
        })
    }));

    match packed {
        Ok(data) => {
            res["Success"] = Value::Bool(true);
            res["data"] = data;
        }
        Err(_) => {
            res["Reason"] = Value::String("cal packing err".into());
        }
    }
    Json(res)
}

/// Python `int(x)`(向零截断)。
fn int_of(x: f64) -> i64 {
    x as i64
}

/// 颜色编号 → 色板(对齐 api.md 文档);越界回退灰色 `#808080`
/// (参考 api.py 用 seeded `randColor`,与文档契约不符,以文档为准)。
fn color_name(c: i64) -> &'static str {
    match c {
        1 => "red",
        2 => "yellow",
        3 => "blue",
        4 => "green",
        5 => "purple",
        6 => "brown",
        7 => "orange",
        _ => "#808080",
    }
}

/// 从 JSON 值读取长度为 3 的浮点数组。
fn array3(v: &Value) -> Result<[f64; 3], ()> {
    let a = v.as_array().ok_or(())?;
    if a.len() != 3 {
        return Err(());
    }
    let mut out = [0.0; 3];
    for (i, e) in a.iter().enumerate() {
        out[i] = e.as_f64().ok_or(())?;
    }
    Ok(out)
}

/// 读取整数(允许 JSON 数字或字符串数字)。
fn as_i64(v: &Value) -> Option<i64> {
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// `makeDictBox`:箱子出参(中心坐标、整型 WHD/weight、重力分布)。
fn make_dict_box(bin: &Bin) -> Value {
    json!({
        "partNumber": bin.partno,
        "position": [bin.width / 2.0, bin.height / 2.0, bin.depth / 2.0],
        "WHD": [int_of(bin.width), int_of(bin.height), int_of(bin.depth)],
        "weight": int_of(bin.max_weight),
        "gravity": bin.gravity,
    })
}

/// `makeDictItem`:物品出参(按 rotation_type 置换 WHD,位置为旋转后 WHD 的一半偏移)。
fn make_dict_item(item: &Item) -> Value {
    let whd = item.dimension();
    let p = item.position;
    let type_name = match item.type_of {
        ItemType::Cube => "cube",
        ItemType::Cylinder => "cylinder",
    };
    json!({
        "partNumber": item.partno,
        "name": item.name,
        "type": type_name,
        "color": item.color,
        "position": [
            int_of(p[0]) + int_of(whd[0]) / 2,
            int_of(p[1]) + int_of(whd[1]) / 2,
            int_of(p[2]) + int_of(whd[2]) / 2,
        ],
        "rotationType": item.rotation_type,
        "WHD": [int_of(whd[0]), int_of(whd[1]), int_of(whd[2])],
        "weight": int_of(item.weight),
        "step": item.step,
        "allowedFloatRatio": item.allowed_float_ratio,
    })
}

/// `getBoxAndItem`:从请求构造 Packer 与绑定组。
fn build_input(v: &Value) -> Result<(Packer, Vec<Vec<String>>), ()> {
    let box_info = v["box"][0].as_object().ok_or(())?;
    let name = box_info.get("name").and_then(Value::as_str).ok_or(())?;
    let whd = array3(box_info.get("WHD").ok_or(())?)?;
    let weight = box_info.get("weight").and_then(Value::as_f64).ok_or(())?;
    let coner = box_info.get("coner").and_then(Value::as_f64).ok_or(())?;
    let open_top = box_info
        .get("openTop")
        .and_then(Value::as_array)
        .ok_or(())?;
    let put_type = open_top
        .first()
        .and_then(as_i64)
        .and_then(|x| i32::try_from(x).ok())
        .ok_or(())?;

    let mut bin = Bin::new(name, whd, weight);
    bin.corner = coner;
    bin.put_type = put_type;
    let mut packer = Packer::new();
    packer.add_bin(bin);

    let item_list = v["item"].as_array().ok_or(())?;
    for it in item_list {
        let it_name = it.get("name").and_then(Value::as_str).ok_or(())?;
        let whd = array3(it.get("WHD").ok_or(())?)?;
        let count = as_i64(it.get("count").ok_or(())?).ok_or(())?;
        let updown = as_i64(it.get("updown").ok_or(())?).ok_or(())? != 0;
        let kind = as_i64(it.get("type").ok_or(())?).ok_or(())?;
        let level = if as_i64(it.get("level").ok_or(())?).ok_or(())? == 1 {
            1
        } else {
            2
        };
        let loadbear: i32 = as_i64(it.get("loadbear").ok_or(())?)
            .and_then(|x| i32::try_from(x).ok())
            .ok_or(())?;
        let weight = it.get("weight").and_then(Value::as_f64).ok_or(())?;
        let color = as_i64(it.get("color").ok_or(())?).ok_or(())?;
        // 可选:允许悬空比例,缺省 0.25;越界值收敛到 0..=1。
        let allowed_float_ratio = it
            .get("allowed_float_ratio")
            .and_then(Value::as_f64)
            .unwrap_or(0.25)
            .clamp(0.0, 1.0);

        let type_of = if kind == 2 {
            ItemType::Cylinder
        } else {
            ItemType::Cube
        };
        // Python `range(count)`:负数展开 0 件。
        for j in 0..count.max(0) {
            packer.add_item(Item::new(
                format!("{}-{}", it_name, j + 1),
                it_name.to_string(),
                type_of,
                whd,
                weight,
                level,
                loadbear,
                updown,
                color_name(color).to_string(),
                allowed_float_ratio,
            ));
        }
    }

    let mut binding: Vec<Vec<String>> = Vec::new();
    if let Some(groups) = v["binding"].as_array() {
        for g in groups {
            if let Some(names) = g.as_array() {
                let names: Vec<String> = names
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect();
                binding.push(names);
            }
        }
    }

    Ok((packer, binding))
}
