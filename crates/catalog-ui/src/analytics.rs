#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentStatus {
    Allowed,
    Declined,
    Unset,
}

impl ConsentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Declined => "declined",
            Self::Unset => "unset",
        }
    }

    pub fn from_status_str(s: &str) -> Self {
        match s {
            "allowed" => Self::Allowed,
            "declined" => Self::Declined,
            _ => Self::Unset,
        }
    }
}

impl std::str::FromStr for ConsentStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_status_str(s))
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "CatalogAnalytics"], js_name = init)]
    fn init_analytics_js() -> bool;

    #[wasm_bindgen(js_namespace = ["window", "CatalogAnalytics"], js_name = getConsent)]
    fn get_consent_js() -> String;

    #[wasm_bindgen(js_namespace = ["window", "CatalogAnalytics"], js_name = setConsent)]
    fn set_consent_js(status: &str);

    #[wasm_bindgen(js_namespace = ["window", "CatalogAnalytics"], js_name = trackEvent)]
    fn track_event_js(name: &str, params: &JsValue);

    #[wasm_bindgen(js_namespace = ["window", "CatalogAnalytics"], js_name = isAnalyticsActive)]
    fn is_analytics_active_js() -> bool;
}

#[cfg(target_arch = "wasm32")]
fn to_js_value<T: serde::Serialize>(val: &T) -> JsValue {
    if let Ok(json_str) = serde_json::to_string(val) {
        if let Ok(js_val) = js_sys::JSON::parse(&json_str) {
            return js_val;
        }
    }
    JsValue::NULL
}

pub fn init_analytics() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        init_analytics_js()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn get_consent() -> ConsentStatus {
    #[cfg(target_arch = "wasm32")]
    {
        ConsentStatus::from_status_str(&get_consent_js())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ConsentStatus::Unset
    }
}

pub fn set_consent(status: ConsentStatus) {
    #[cfg(target_arch = "wasm32")]
    {
        set_consent_js(status.as_str());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = status;
    }
}

pub fn is_analytics_active() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        is_analytics_active_js()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn track_object_open(kind: &str, id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let params = serde_json::json!({
            "object_kind": kind,
            "object_id": id,
        });
        track_event_js("catalog_object_open", &to_js_value(&params));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (kind, id);
    }
}

pub fn track_filter_apply(category: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let params = serde_json::json!({
            "category": category,
        });
        track_event_js("catalog_filter_apply", &to_js_value(&params));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = category;
    }
}

pub fn track_search_empty() {
    #[cfg(target_arch = "wasm32")]
    {
        let params = serde_json::json!({
            "result_count": 0,
        });
        track_event_js("catalog_search_empty", &to_js_value(&params));
    }
}

pub fn track_maps_click(kind: &str, id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let params = serde_json::json!({
            "object_kind": kind,
            "object_id": id,
        });
        track_event_js("catalog_maps_click", &to_js_value(&params));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (kind, id);
    }
}

pub fn track_gpx_download(id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let params = serde_json::json!({
            "object_kind": "route",
            "object_id": id,
        });
        track_event_js("catalog_gpx_download", &to_js_value(&params));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
    }
}
