use catalog_model::{ObjectKey, ObjectKind};
use std::collections::HashSet;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = getSavedKeys, catch)]
    async fn get_saved_keys_js() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = saveObject, catch)]
    async fn save_object_js(kind: &str, id: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = removeSaved, catch)]
    async fn remove_saved_js(kind: &str, id: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = isSaved, catch)]
    async fn is_saved_js(kind: &str, id: &str) -> Result<JsValue, JsValue>;
}

pub fn parse_object_key_str(s: &str) -> Option<ObjectKey> {
    let mut parts = s.splitn(2, ':');
    let kind_str = parts.next()?;
    let id = parts.next()?.to_string();
    let kind = match kind_str {
        "route" => ObjectKind::Route,
        "place" => ObjectKind::Place,
        "road" => ObjectKind::Road,
        _ => return None,
    };
    Some(ObjectKey { kind, id })
}

pub fn format_object_key(key: &ObjectKey) -> String {
    let kind_str = match key.kind {
        ObjectKind::Route => "route",
        ObjectKind::Place => "place",
        ObjectKind::Road => "road",
    };
    format!("{}:{}", kind_str, key.id)
}

pub async fn load_saved_keys() -> HashSet<ObjectKey> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(val) = get_saved_keys_js().await {
            let array = js_sys::Array::from(&val);
            let mut keys = HashSet::new();
            for i in 0..array.length() {
                if let Some(s) = array.get(i).as_string() {
                    if let Some(k) = parse_object_key_str(&s) {
                        keys.insert(k);
                    }
                }
            }
            return keys;
        }
        HashSet::new()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        HashSet::new()
    }
}

pub async fn save_key(key: &ObjectKey) {
    #[cfg(target_arch = "wasm32")]
    {
        let kind_str = match key.kind {
            ObjectKind::Route => "route",
            ObjectKind::Place => "place",
            ObjectKind::Road => "road",
        };
        let _ = save_object_js(kind_str, &key.id).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
    }
}

pub async fn remove_key(key: &ObjectKey) {
    #[cfg(target_arch = "wasm32")]
    {
        let kind_str = match key.kind {
            ObjectKind::Route => "route",
            ObjectKind::Place => "place",
            ObjectKind::Road => "road",
        };
        let _ = remove_saved_js(kind_str, &key.id).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
    }
}

pub async fn check_is_saved(key: &ObjectKey) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let kind_str = match key.kind {
            ObjectKind::Route => "route",
            ObjectKind::Place => "place",
            ObjectKind::Road => "road",
        };
        if let Ok(val) = is_saved_js(kind_str, &key.id).await {
            return val.as_bool().unwrap_or(false);
        }
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        false
    }
}

pub async fn toggle_saved(key: &ObjectKey) -> bool {
    if check_is_saved(key).await {
        remove_key(key).await;
        false
    } else {
        save_key(key).await;
        true
    }
}
