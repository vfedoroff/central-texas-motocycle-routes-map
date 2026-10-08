#[cfg(target_arch = "wasm32")]
use catalog_model::ObjectKey;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = Object)]
    pub type JsMapAdapter;

    #[wasm_bindgen(js_name = createMap)]
    pub fn create_map_js(element_id: &str, config: &JsValue) -> JsMapAdapter;

    #[wasm_bindgen(method, js_name = setOverview)]
    pub fn set_overview_js(this: &JsMapAdapter, key: &JsValue, feature: &JsValue);

    #[wasm_bindgen(method, js_name = removeOverview)]
    pub fn remove_overview_js(this: &JsMapAdapter, key: &JsValue);

    #[wasm_bindgen(method, js_name = setPlaces)]
    pub fn set_places_js(this: &JsMapAdapter, points: &JsValue);

    #[wasm_bindgen(method, js_name = setSelection)]
    pub fn set_selection_js(this: &JsMapAdapter, key: &JsValue, feature: &JsValue);

    #[wasm_bindgen(method, js_name = clearSelection)]
    pub fn clear_selection_js(this: &JsMapAdapter);

    #[wasm_bindgen(method, js_name = fitBounds)]
    pub fn fit_bounds_js(this: &JsMapAdapter, bounds: &JsValue);

    #[wasm_bindgen(method, js_name = reframeSelection)]
    pub fn reframe_selection_js(this: &JsMapAdapter);

    #[wasm_bindgen(method, js_name = reframeOverview)]
    pub fn reframe_overview_js(this: &JsMapAdapter);

    #[wasm_bindgen(method, js_name = locateUser)]
    pub fn locate_user_js(this: &JsMapAdapter, on_success: &JsValue, on_error: &JsValue);

    #[wasm_bindgen(method, js_name = setUserLocation)]
    pub fn set_user_location_js(this: &JsMapAdapter, lat: f64, lon: f64, accuracy: f64);

    #[wasm_bindgen(method, js_name = clearUserLocation)]
    pub fn clear_user_location_js(this: &JsMapAdapter);

    #[wasm_bindgen(method, js_name = resize)]
    pub fn resize_js(this: &JsMapAdapter);

    #[wasm_bindgen(method, js_name = destroy)]
    pub fn destroy_js(this: &JsMapAdapter);
}

#[cfg(target_arch = "wasm32")]
pub struct LeafletMap {
    adapter: Option<JsMapAdapter>,
}

#[cfg(target_arch = "wasm32")]
impl LeafletMap {
    pub fn new(element_id: &str, config: &JsValue) -> Result<Self, JsValue> {
        let adapter = create_map_js(element_id, config);
        Ok(Self {
            adapter: Some(adapter),
        })
    }

    pub fn set_overview(&self, key: &ObjectKey, feature: &JsValue) {
        if let Some(ref adapter) = self.adapter {
            if let Ok(json_str) = serde_json::to_string(key) {
                if let Ok(js_key) = js_sys::JSON::parse(&json_str) {
                    adapter.set_overview_js(&js_key, feature);
                }
            }
        }
    }

    pub fn remove_overview(&self, key: &ObjectKey) {
        if let Some(ref adapter) = self.adapter {
            if let Ok(json_str) = serde_json::to_string(key) {
                if let Ok(js_key) = js_sys::JSON::parse(&json_str) {
                    adapter.remove_overview_js(&js_key);
                }
            }
        }
    }

    pub fn set_places(&self, points: &JsValue) {
        if let Some(ref adapter) = self.adapter {
            adapter.set_places_js(points);
        }
    }

    pub fn set_selection(&self, key: &ObjectKey, feature: &JsValue) {
        if let Some(ref adapter) = self.adapter {
            if let Ok(json_str) = serde_json::to_string(key) {
                if let Ok(js_key) = js_sys::JSON::parse(&json_str) {
                    adapter.set_selection_js(&js_key, feature);
                }
            }
        }
    }

    pub fn clear_selection(&self) {
        if let Some(ref adapter) = self.adapter {
            adapter.clear_selection_js();
        }
    }

    pub fn fit_bounds(&self, bounds: [[f64; 2]; 2]) {
        if let Some(ref adapter) = self.adapter {
            if let Ok(json_str) = serde_json::to_string(&bounds) {
                if let Ok(js_bounds) = js_sys::JSON::parse(&json_str) {
                    adapter.fit_bounds_js(&js_bounds);
                }
            }
        }
    }

    pub fn reframe_selection(&self) {
        if let Some(ref adapter) = self.adapter {
            adapter.reframe_selection_js();
        }
    }

    pub fn reframe_overview(&self) {
        if let Some(ref adapter) = self.adapter {
            adapter.reframe_overview_js();
        }
    }

    pub fn locate_user(&self, on_success: &JsValue, on_error: &JsValue) {
        if let Some(ref adapter) = self.adapter {
            adapter.locate_user_js(on_success, on_error);
        }
    }

    pub fn set_user_location(&self, lat: f64, lon: f64, accuracy: f64) {
        if let Some(ref adapter) = self.adapter {
            adapter.set_user_location_js(lat, lon, accuracy);
        }
    }

    pub fn clear_user_location(&self) {
        if let Some(ref adapter) = self.adapter {
            adapter.clear_user_location_js();
        }
    }

    pub fn resize(&self) {
        if let Some(ref adapter) = self.adapter {
            adapter.resize_js();
        }
    }

    pub fn destroy(&mut self) {
        if let Some(adapter) = self.adapter.take() {
            adapter.destroy_js();
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for LeafletMap {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for LeafletMap {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for LeafletMap {}

#[cfg(not(target_arch = "wasm32"))]
pub struct LeafletMap;

#[cfg(not(target_arch = "wasm32"))]
impl LeafletMap {
    pub fn dummy() -> Self {
        Self
    }
}
