#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum PackStatus {
    NotDownloaded,
    Downloading {
        percent: u8,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    Downloaded {
        total_bytes: u64,
    },
    NeedsDownload {
        total_bytes: u64,
    },
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = getPackStatus, catch)]
    async fn get_pack_status_js(route_id: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = downloadPack, catch)]
    async fn download_pack_js(route_id: &str, on_progress: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = cancelDownload)]
    fn cancel_download_js(route_id: &str);

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = removePack, catch)]
    async fn remove_pack_js(route_id: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = initServiceWorker)]
    fn init_service_worker_js(on_update: &JsValue);

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = applyUpdate)]
    fn apply_update_js();

    #[wasm_bindgen(js_namespace = ["window", "CatalogPWA"], js_name = isUpdateAvailable)]
    fn is_update_available_js() -> bool;
}

pub async fn get_pack_status(route_id: &str) -> PackStatus {
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(val) = get_pack_status_js(route_id).await {
            if val.is_object() {
                let status_str = js_sys::Reflect::get(&val, &JsValue::from_str("status"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let downloaded_bytes =
                    js_sys::Reflect::get(&val, &JsValue::from_str("downloadedBytes"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as u64;
                let total_bytes = js_sys::Reflect::get(&val, &JsValue::from_str("totalBytes"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                let percent = js_sys::Reflect::get(&val, &JsValue::from_str("percent"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u8;

                return match status_str.as_str() {
                    "downloading" => PackStatus::Downloading {
                        percent,
                        downloaded_bytes,
                        total_bytes,
                    },
                    "downloaded" => PackStatus::Downloaded { total_bytes },
                    "needs_download" => PackStatus::NeedsDownload { total_bytes },
                    _ => PackStatus::NotDownloaded,
                };
            }
        }
        PackStatus::NotDownloaded
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = route_id;
        PackStatus::NotDownloaded
    }
}

pub async fn download_pack(
    route_id: &str,
    on_progress: impl Fn(u8, u64, u64) + 'static,
) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let cb = Closure::wrap(Box::new(move |val: JsValue| {
            if val.is_object() {
                let percent = js_sys::Reflect::get(&val, &JsValue::from_str("percent"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u8;
                let bytes = js_sys::Reflect::get(&val, &JsValue::from_str("bytes"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                let total = js_sys::Reflect::get(&val, &JsValue::from_str("totalBytes"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                on_progress(percent, bytes, total);
            }
        }) as Box<dyn Fn(JsValue)>);

        let res = download_pack_js(route_id, cb.as_ref()).await;
        // Keep closure alive during call
        drop(cb);

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_msg = e.as_string().unwrap_or_else(|| {
                    js_sys::Reflect::get(&e, &JsValue::from_str("message"))
                        .ok()
                        .and_then(|m| m.as_string())
                        .unwrap_or_else(|| "Download failed".to_string())
                });
                Err(err_msg)
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = route_id;
        let _ = on_progress;
        Ok(())
    }
}

pub fn cancel_download(route_id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        cancel_download_js(route_id);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = route_id;
    }
}

pub async fn remove_pack(route_id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = remove_pack_js(route_id).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = route_id;
    }
}

pub fn init_service_worker(on_update: impl Fn() + 'static) {
    #[cfg(target_arch = "wasm32")]
    {
        let cb = Closure::wrap(Box::new(on_update) as Box<dyn Fn()>);
        init_service_worker_js(cb.as_ref());
        cb.forget();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = on_update;
    }
}

pub fn apply_update() {
    #[cfg(target_arch = "wasm32")]
    {
        apply_update_js();
    }
}

pub fn is_update_available() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        is_update_available_js()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}
