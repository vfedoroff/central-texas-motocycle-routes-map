#[cfg(target_arch = "wasm32")]
use catalog_model::{CatalogIndex, ObjectKey, ObjectKind};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::collections::HashSet;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{UrlSearchParams, window};

#[cfg(target_arch = "wasm32")]
use crate::controls::Controls;
#[cfg(target_arch = "wasm32")]
use crate::details::DetailView;
#[cfg(target_arch = "wasm32")]
use crate::results::Results;
#[cfg(target_arch = "wasm32")]
use crate::state::{CatalogState, PAGE_SIZE};

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobileSnapState {
    Collapsed,
    Half,
    Expanded,
}

#[cfg(target_arch = "wasm32")]
fn parse_url_selection() -> Result<Option<ObjectKey>, String> {
    let win = window().ok_or("No window")?;
    let search = win.location().search().map_err(|_| "No search")?;
    if search.is_empty() {
        return Ok(None);
    }

    let params = UrlSearchParams::new_with_str(&search).map_err(|_| "Failed to parse params")?;
    let kind_val = params.get("kind");
    let id_val = params.get("id");

    match (kind_val, id_val) {
        (Some(k), Some(id)) => {
            let kind = match k.as_str() {
                "route" => ObjectKind::Route,
                "road" => ObjectKind::Road,
                "place" => ObjectKind::Place,
                _ => return Err(format!("Unknown kind '{}' in URL", k)),
            };
            if id.trim().is_empty() {
                return Err("Empty ID in URL".to_string());
            }
            Ok(Some(ObjectKey { kind, id }))
        }
        (Some(_), None) => Err("Missing 'id' parameter in URL".to_string()),
        (None, Some(_)) => Err("Missing 'kind' parameter in URL".to_string()),
        (None, None) => Ok(None),
    }
}

#[cfg(target_arch = "wasm32")]
fn update_url_selection(selection: Option<&ObjectKey>) {
    if let Some(win) = window() {
        if let Ok(history) = win.history() {
            let url = match selection {
                Some(key) => {
                    let k = format!("{:?}", key.kind).to_lowercase();
                    format!("/?kind={}&id={}", k, key.id)
                }
                None => "/".to_string(),
            };
            let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn pump_loader_requests(
    loader: &std::sync::Arc<std::sync::Mutex<crate::loader::LoaderState>>,
    map_cell: &std::sync::Arc<std::sync::Mutex<Option<crate::map::LeafletMap>>>,
    set_error: WriteSignal<Option<String>>,
) {
    let mut to_fetch = Vec::new();
    if let Ok(mut l) = loader.lock() {
        while let Some(cand) = l.next_request() {
            to_fetch.push((l.generation, cand));
        }
    }

    for (generation, cand) in to_fetch {
        let loader_clone = loader.clone();
        let map_clone = map_cell.clone();
        leptos::task::spawn_local(async move {
            let mut res = match gloo_net::http::Request::get(&cand.overview_url)
                .send()
                .await
            {
                Ok(resp) if resp.ok() => resp.json::<serde_json::Value>().await.ok(),
                _ => None,
            };

            if let Some(ref mut feat) = res {
                if let Some(props) = feat.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    if !props.contains_key("color") {
                        if let Some(ref col) = cand.color {
                            props.insert("color".into(), serde_json::Value::String(col.clone()));
                        }
                    }
                }
            }

            let (to_add, to_remove) = if let Ok(mut l) = loader_clone.lock() {
                let (add, rem) = l.complete_request(generation, cand.key.clone(), res);
                if l.has_error {
                    set_error.set(Some("Failed to load geometry for some routes".to_string()));
                }
                (add, rem)
            } else {
                (None, None)
            };

            if let Ok(lock) = map_clone.lock() {
                if let Some(map) = lock.as_ref() {
                    if let Some(rem_key) = to_remove {
                        map.remove_overview(&rem_key);
                    }
                    if let Some((add_key, feat)) = to_add {
                        if let Ok(json_str) = serde_json::to_string(&feat) {
                            if let Ok(js_feat) = js_sys::JSON::parse(&json_str) {
                                map.set_overview(&add_key, &js_feat);
                            }
                        }
                    }
                }
            }

            pump_loader_requests(&loader_clone, &map_clone, set_error);
        });
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn App() -> impl IntoView {
    // Core reactive state
    let (catalog, set_catalog) = signal(Option::<CatalogIndex>::None);
    let (loading, set_loading) = signal(true);
    let (load_error, set_load_error) = signal(Option::<String>::None);
    let (url_error, set_url_error) = signal(Option::<String>::None);

    let (state, set_state) = signal(CatalogState::new());
    let (mobile_snap, set_mobile_snap) = signal(MobileSnapState::Half);

    // Function to fetch index
    let fetch_catalog = move || {
        set_loading.set(true);
        set_load_error.set(None);

        leptos::task::spawn_local(async move {
            match gloo_net::http::Request::get("/catalog-index.json")
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.ok() {
                        match resp.json::<CatalogIndex>().await {
                            Ok(idx) => {
                                // Check initial URL selection
                                match parse_url_selection() {
                                    Ok(Some(sel_key)) => {
                                        let kind_str = match sel_key.kind {
                                            ObjectKind::Route => "route",
                                            ObjectKind::Road => "road",
                                            ObjectKind::Place => "place",
                                        };
                                        crate::analytics::track_object_open(kind_str, &sel_key.id);
                                        set_state.update(|s| {
                                            s.set_kind(sel_key.kind);
                                            s.select_key(Some(sel_key));
                                        });
                                    }
                                    Ok(None) => {}
                                    Err(err) => {
                                        set_url_error.set(Some(err));
                                    }
                                }
                                set_catalog.set(Some(idx));
                                set_loading.set(false);
                            }
                            Err(e) => {
                                set_load_error.set(Some(format!("JSON parsing error: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_load_error.set(Some(format!("HTTP status {}", resp.status())));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    set_load_error.set(Some(format!("Network error: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Initial load
    let (sw_update_available, set_sw_update_available) = signal(false);
    Effect::new(move |_| {
        fetch_catalog();

        leptos::task::spawn_local(async move {
            let keys = crate::saved::load_saved_keys().await;
            set_state.update(|s| s.set_saved_keys(keys));
        });

        crate::offline::init_service_worker(move || {
            set_sw_update_available.set(true);
        });

        // Listen for browser popstate
        if let Some(win) = window() {
            let callback = wasm_bindgen::closure::Closure::<dyn Fn()>::wrap(Box::new(move || {
                match parse_url_selection() {
                    Ok(Some(sel_key)) => {
                        let kind_str = match sel_key.kind {
                            ObjectKind::Route => "route",
                            ObjectKind::Road => "road",
                            ObjectKind::Place => "place",
                        };
                        crate::analytics::track_object_open(kind_str, &sel_key.id);
                        set_state.update(|s| {
                            s.set_kind(sel_key.kind);
                            s.select_key(Some(sel_key));
                        });
                    }
                    Ok(None) => {
                        set_state.update(|s| s.select_key(None));
                    }
                    Err(err) => {
                        set_url_error.set(Some(err));
                    }
                }
            }));
            let _ =
                win.add_event_listener_with_callback("popstate", callback.as_ref().unchecked_ref());
            callback.forget();
        }
    });

    // Computed filtered results
    let matching_keys = Memo::new(move |_| {
        if let Some(ref cat) = catalog.get() {
            state.get().current_results(cat)
        } else {
            Vec::new()
        }
    });

    let total_results = Signal::derive(move || matching_keys.get().len());

    let total_pages = Signal::derive(move || {
        let count = total_results.get();
        if count == 0 {
            1
        } else {
            count.div_ceil(PAGE_SIZE)
        }
    });

    let current_page_summaries = Signal::derive(move || {
        let cat_opt = catalog.get();
        if let Some(ref cat) = cat_opt {
            let keys = matching_keys.get();
            let page_keys = state.get().page_slice(&keys);
            // Map keys to summaries
            page_keys
                .iter()
                .filter_map(|key| cat.objects.iter().find(|s| &s.key == key).cloned())
                .collect()
        } else {
            Vec::new()
        }
    });

    // Consent and privacy settings signals
    let (consent_status, set_consent_status) = signal(crate::analytics::get_consent());
    let (show_privacy_dialog, set_show_privacy_dialog) = signal(false);

    let opener_element: std::rc::Rc<std::cell::RefCell<Option<web_sys::HtmlElement>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let opener_element_eff = opener_element.clone();
    let privacy_listener: std::rc::Rc<
        std::cell::RefCell<
            Option<wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>>,
        >,
    > = std::rc::Rc::new(std::cell::RefCell::new(None));
    let privacy_listener_eff = privacy_listener.clone();

    Effect::new(move |_| {
        let is_open = show_privacy_dialog.get();
        if is_open {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Some(active) = doc.active_element() {
                        *opener_element_eff.borrow_mut() =
                            active.dyn_into::<web_sys::HtmlElement>().ok();
                    }

                    // Focus close button after modal mounts
                    gloo_timers::callback::Timeout::new(20, move || {
                        if let Some(win) = web_sys::window() {
                            if let Some(doc) = win.document() {
                                if let Some(close_btn) =
                                    doc.query_selector(".btn-close-privacy").ok().flatten()
                                {
                                    if let Ok(el) = close_btn.dyn_into::<web_sys::HtmlElement>() {
                                        let _ = el.focus();
                                    }
                                }
                            }
                        }
                    })
                    .forget();

                    // Window keydown listener for Escape and Tab trap
                    let set_show = set_show_privacy_dialog;
                    let cb =
                        wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(
                            Box::new(move |ev: web_sys::KeyboardEvent| {
                                let key = ev.key();
                                if key == "Escape" {
                                    ev.prevent_default();
                                    ev.stop_propagation();
                                    set_show.set(false);
                                    return;
                                }

                                if key == "Tab" {
                                    if let Some(w) = web_sys::window() {
                                        if let Some(d) = w.document() {
                                            if let Some(dialog) =
                                                d.query_selector(".privacy-dialog").ok().flatten()
                                            {
                                                if let Ok(nodes) = dialog.query_selector_all(
                                            "button, a[href], input, select, textarea, [tabindex]:not([tabindex='-1'])",
                                        ) {
                                            let len = nodes.length();
                                            if len > 0 {
                                                let first_el = nodes
                                                    .get(0)
                                                    .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok());
                                                let last_el = nodes
                                                    .get(len - 1)
                                                    .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok());
                                                let active_el = d
                                                    .active_element()
                                                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());

                                                let is_inside = active_el
                                                    .as_ref()
                                                    .map(|a| dialog.contains(Some(a)))
                                                    .unwrap_or(false);

                                                if !is_inside {
                                                    ev.prevent_default();
                                                    if let Some(f) = first_el {
                                                        let _ = f.focus();
                                                    }
                                                } else if ev.shift_key() {
                                                    if active_el.as_ref() == first_el.as_ref() {
                                                        ev.prevent_default();
                                                        if let Some(l) = last_el {
                                                            let _ = l.focus();
                                                        }
                                                    }
                                                } else if active_el.as_ref() == last_el.as_ref() {
                                                    ev.prevent_default();
                                                    if let Some(f) = first_el {
                                                        let _ = f.focus();
                                                    }
                                                }
                                            }
                                        }
                                            }
                                        }
                                    }
                                }
                            }),
                        );

                    let _ = win
                        .add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                    *privacy_listener_eff.borrow_mut() = Some(cb);
                }
            }
        } else {
            // Clean up window keydown listener
            if let Some(cb) = privacy_listener_eff.borrow_mut().take() {
                if let Some(win) = web_sys::window() {
                    let _ = win.remove_event_listener_with_callback(
                        "keydown",
                        cb.as_ref().unchecked_ref(),
                    );
                }
            }
            // Restore focus to opener element
            if let Some(opener) = opener_element_eff.borrow_mut().take() {
                let _ = opener.focus();
            }
        }
    });

    // Track empty search results without exposing query string
    let last_empty_query = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let last_empty_query_clone = last_empty_query.clone();
    Effect::new(move |_| {
        let q = state.get().filter.query.trim().to_string();
        let count = total_results.get();
        if !q.is_empty() && count == 0 {
            if *last_empty_query_clone.borrow() != q {
                *last_empty_query_clone.borrow_mut() = q;
                crate::analytics::track_search_empty();
            }
        } else {
            last_empty_query_clone.borrow_mut().clear();
        }
    });

    // Control callbacks
    let on_query_change = Callback::new(move |q: String| {
        set_state.update(|s| s.set_query(q));
    });

    let on_category_toggle = Callback::new(move |cat: String| {
        crate::analytics::track_filter_apply(&cat);
        set_state.update(|s| {
            let mut cats = s.filter.categories.clone();
            if let Some(pos) = cats.iter().position(|c| c == &cat) {
                cats.remove(pos);
            } else {
                cats.push(cat);
            }
            s.set_categories(cats);
        });
    });

    let on_mileage_change = Callback::new(move |(min, max): (Option<f64>, Option<f64>)| {
        set_state.update(|s| {
            let _ = s.set_mileage(min, max);
        });
    });

    let on_apply_bounds = Callback::new(move |_| {
        set_state.update(|s| s.apply_pending_bounds());
    });

    let on_clear_filters = Callback::new(move |_| {
        set_state.update(|s| s.clear_filters());
    });

    let on_clear_bounds = Callback::new(move |_| {
        set_state.update(|s| s.clear_bounds());
    });

    // Leaflet map handle
    let map_cell = std::sync::Arc::new(std::sync::Mutex::new(None::<crate::map::LeafletMap>));
    let map_cell_cleanup = map_cell.clone();
    let map_cell_mount = map_cell.clone();
    let map_cell_back = map_cell.clone();
    let map_cell_detail = map_cell.clone();
    let map_cell_locate = map_cell.clone();

    let (location_notice, set_location_notice) = signal(Option::<String>::None);
    let is_near_me_sig = Signal::derive(move || state.get().sort_by_near_me);
    let has_user_location_sig = Signal::derive(move || state.get().user_location.is_some());
    let user_location_sig = Signal::derive(move || state.get().user_location);

    let on_user_location = Callback::new(move |(lat, lon): (f64, f64)| {
        set_state.update(|s| {
            s.set_user_location(Some([lon, lat]));
            s.set_sort_by_near_me(true);
        });
        set_location_notice.set(None);
    });

    let on_location_error = Callback::new(move |msg: String| {
        set_location_notice.set(Some(msg));
    });

    let on_locate_me = Callback::new(move |_| {
        if let Ok(lock) = map_cell_locate.lock() {
            if let Some(ref m) = *lock {
                m.locate_user(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::NULL);
            }
        }
    });

    let on_toggle_near_me = Callback::new(move |_| {
        set_state.update(|s| {
            let next = !s.sort_by_near_me;
            s.set_sort_by_near_me(next);
        });
    });

    let (detail_data, set_detail_data) = signal(None::<catalog_model::ObjectDetail>);
    let (detail_loading, set_detail_loading) = signal(false);
    let (detail_error, set_detail_error) = signal(None::<String>);
    let (detail_reload_trigger, set_detail_reload_trigger) = signal(0u64);

    let on_select = Callback::new(move |key: ObjectKey| {
        let kind_str = match key.kind {
            ObjectKind::Route => "route",
            ObjectKind::Road => "road",
            ObjectKind::Place => "place",
        };
        crate::analytics::track_object_open(kind_str, &key.id);
        set_state.update(|s| s.select_with_prior_context(key.clone()));
        update_url_selection(Some(&key));
        set_mobile_snap.set(MobileSnapState::Half);
    });

    let on_back_to_results = Callback::new(move |_| {
        let prior_vp = state.get().prior_viewport_bounds;
        set_state.update(|s| s.restore_back_to_results());
        update_url_selection(None);
        if let Ok(m_lock) = map_cell_back.lock() {
            if let Some(ref m) = *m_lock {
                m.clear_selection();
                if let Some(b) = prior_vp {
                    m.fit_bounds([[b[0], b[1]], [b[2], b[3]]]);
                } else {
                    m.reframe_overview();
                }
            }
        }
    });

    let on_back_to_route = Callback::new(move |_| {
        set_state.update(|s| s.restore_back_to_route());
        if let Some(ref r_key) = state.get().selected_key {
            update_url_selection(Some(r_key));
        } else {
            update_url_selection(None);
        }
    });

    let on_select_place_from_route = Callback::new(move |place_key: ObjectKey| {
        crate::analytics::track_object_open("place", &place_key.id);
        let cur_route_key = state.get().selected_key.clone();
        if let Some(r_key) = cur_route_key {
            set_state.update(|s| s.select_place_from_route(place_key.clone(), r_key));
        } else {
            set_state.update(|s| s.select_with_prior_context(place_key.clone()));
        }
        update_url_selection(Some(&place_key));
        set_mobile_snap.set(MobileSnapState::Half);
    });

    let on_retry_detail = Callback::new(move |_| {
        set_detail_reload_trigger.update(|n| *n = n.wrapping_add(1));
    });

    let return_route_key_sig = Signal::derive(move || state.get().return_route_key.clone());
    let detail_sig = Signal::derive(move || detail_data.get());
    let detail_loading_sig = Signal::derive(move || detail_loading.get());
    let detail_error_sig = Signal::derive(move || detail_error.get());

    let on_retry = Callback::new(move |_| {
        fetch_catalog();
    });

    let kind_sig = Signal::derive(move || state.get().filter.kind);
    let on_kind_change = Callback::new(move |new_kind| {
        set_state.update(|s| s.set_kind(new_kind));
    });

    let sort_sig = Signal::derive(move || state.get().sort);
    let on_sort_change = Callback::new(move |new_sort| {
        set_state.update(|s| s.set_sort(new_sort));
    });

    let query_sig = Signal::derive(move || state.get().filter.query.clone());
    let categories_sig = Signal::derive(move || state.get().filter.categories.clone());
    let min_dist_sig = Signal::derive(move || state.get().filter.min_distance_mi);
    let max_dist_sig = Signal::derive(move || state.get().filter.max_distance_mi);
    let nearby_routes_sig = Signal::derive(move || state.get().nearby_routes_visible);
    let on_nearby_routes_toggle = Callback::new(move |v| {
        set_state.update(|s| s.nearby_routes_visible = v);
    });
    let places_overlay_sig = Signal::derive(move || state.get().places_overlay_visible);
    let on_places_overlay_toggle = Callback::new(move |v| {
        set_state.update(|s| s.set_places_overlay_visible(v));
    });
    let pending_bounds_sig = Signal::derive(move || {
        state.get().pending_bounds.is_some()
            && state.get().pending_bounds != state.get().filter.applied_bounds
    });
    let applied_bounds_sig = Signal::derive(move || state.get().filter.applied_bounds.is_some());

    let saved_only_sig = Signal::derive(move || state.get().filter.saved_only);
    let on_saved_toggle = Callback::new(move |v| {
        set_state.update(|s| s.set_saved_only(v));
    });

    let saved_keys_sig = Signal::derive(move || state.get().saved_keys.clone());
    let on_toggle_save = Callback::new(move |key: ObjectKey| {
        let key_clone = key.clone();
        leptos::task::spawn_local(async move {
            let is_now_saved = crate::saved::toggle_saved(&key_clone).await;
            set_state.update(|s| {
                if is_now_saved {
                    s.saved_keys.insert(key_clone);
                } else {
                    s.saved_keys.remove(&key_clone);
                }
            });
        });
    });

    let detail_data_save = detail_data.clone();
    let on_toggle_save_selected = {
        let on_toggle_save = on_toggle_save;
        Callback::new(move |_| {
            if let Some(key) = state.get().selected_key {
                let is_already_saved = state.get().saved_keys.contains(&key);
                let has_valid_detail = detail_data_save.get().is_some();
                if has_valid_detail || is_already_saved {
                    on_toggle_save.run(key);
                }
            }
        })
    };

    let is_selected_saved_sig = Signal::derive(move || {
        if let Some(ref sel) = state.get().selected_key {
            state.get().saved_keys.contains(sel)
        } else {
            false
        }
    });

    let unavailable_keys = Signal::derive(move || {
        let st = state.get();
        if !st.filter.saved_only {
            return Vec::new();
        }
        let Some(ref cat) = catalog.get() else {
            return Vec::new();
        };
        let existing: HashSet<ObjectKey> = cat.objects.iter().map(|o| o.key.clone()).collect();
        st.saved_keys
            .iter()
            .filter(|k| k.kind == st.filter.kind && !existing.contains(k))
            .cloned()
            .collect()
    });

    let has_places = Memo::new(move |_| {
        catalog
            .get()
            .map(|c| c.objects.iter().any(|o| o.key.kind == ObjectKind::Place))
            .unwrap_or(false)
    });
    let catalog_places_count = Signal::derive(move || {
        catalog
            .get()
            .map(|c| {
                c.objects
                    .iter()
                    .filter(|o| o.key.kind == ObjectKind::Place)
                    .count()
            })
            .unwrap_or(0)
    });
    let on_browse_routes = Callback::new(move |()| {
        set_state.update(|s| {
            s.set_kind(ObjectKind::Route);
        });
    });
    let on_disable_saved_only = Callback::new(move |()| {
        set_state.update(|s| {
            s.set_saved_only(false);
        });
    });

    let page_sig = Signal::derive(move || state.get().page);
    let on_page_change = Callback::new(move |p| {
        set_state.update(|s| s.set_page(p));
    });

    let selected_key_sig = Signal::derive(move || state.get().selected_key.clone());
    let is_selected_sig = Memo::new(move |_| state.get().selected_key.is_some());

    // Focus restoration: when selected_key transitions from Some(key) to None, restore focus to that card
    let (last_selected_key, set_last_selected_key) = signal(Option::<ObjectKey>::None);
    Effect::new(move |_| {
        let cur = state.get().selected_key;
        if let Some(ref k) = cur {
            set_last_selected_key.set(Some(k.clone()));
        } else if let Some(last) = last_selected_key.get_untracked() {
            let key_id = last.id.clone();
            set_last_selected_key.set(None);
            for delay in [30, 80, 200, 350, 500] {
                let kid = key_id.clone();
                gloo_timers::callback::Timeout::new(delay, move || {
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        let active = doc.active_element();
                        let needs_focus = active.as_ref().map_or(true, |el| {
                            el.tag_name() == "BODY" || !el.is_connected()
                        });
                        if needs_focus {
                            let selector = format!("[data-id=\"{}\"]", kid);
                            if let Ok(nodes) = doc.query_selector_all(&selector) {
                                let mut focused = false;
                                for i in 0..nodes.length() {
                                    if let Some(node) = nodes.item(i) {
                                        if let Ok(el) = node.dyn_into::<web_sys::HtmlElement>() {
                                            if el.offset_parent().is_some() || el.client_width() > 0 {
                                                let _ = el.focus();
                                                focused = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                if !focused {
                                    let fallback = doc.query_selector(".sidebar:not([style*='display: none']) .result-card, .bottom-panel .result-card, .tab-btn.tab-active");
                                    if let Ok(Some(fb)) = fallback {
                                        if let Ok(el) = fb.dyn_into::<web_sys::HtmlElement>() {
                                            let _ = el.focus();
                                        }
                                    }
                                }
                            }
                        }
                    }
                })
                .forget();
            }
        }
    });

    let mobile_snap_class = move || match mobile_snap.get() {
        MobileSnapState::Collapsed => "bottom-panel snap-collapsed",
        MobileSnapState::Half => "bottom-panel snap-half",
        MobileSnapState::Expanded => "bottom-panel snap-expanded",
    };
    Effect::new(move |_| {
        let config = js_sys::Object::new();

        // onSelect callback
        let on_sel = on_select;
        let select_cb = wasm_bindgen::closure::Closure::<dyn Fn(wasm_bindgen::JsValue)>::wrap(
            Box::new(move |val| {
                if let Ok(json_str) = js_sys::JSON::stringify(&val) {
                    if let Some(s) = json_str.as_string() {
                        if let Ok(key) = serde_json::from_str::<ObjectKey>(&s) {
                            on_sel.run(key);
                        }
                    }
                }
            }),
        );
        let _ = js_sys::Reflect::set(
            &config,
            &wasm_bindgen::JsValue::from_str("onSelect"),
            select_cb.as_ref().unchecked_ref(),
        );
        select_cb.forget();

        // onBoundsChanged callback
        let bounds_cb = wasm_bindgen::closure::Closure::<
            dyn Fn(wasm_bindgen::JsValue, wasm_bindgen::JsValue),
        >::wrap(Box::new(move |val, initial| {
            if let Ok(json_str) = js_sys::JSON::stringify(&val) {
                if let Some(s) = json_str.as_string() {
                    if let Ok(b) = serde_json::from_str::<[[f64; 2]; 2]>(&s) {
                        let flat_bounds = [b[0][0], b[0][1], b[1][0], b[1][1]];
                        set_state.update(|st| {
                            st.viewport_bounds = Some(flat_bounds);
                            if initial.as_bool() != Some(true) {
                                st.set_pending_bounds(Some(flat_bounds));
                            }
                        });
                    }
                }
            }
        }));
        let _ = js_sys::Reflect::set(
            &config,
            &wasm_bindgen::JsValue::from_str("onBoundsChanged"),
            bounds_cb.as_ref().unchecked_ref(),
        );
        bounds_cb.forget();

        // onUserLocation callback
        let on_usr_loc = on_user_location;
        let loc_cb = wasm_bindgen::closure::Closure::<
            dyn Fn(wasm_bindgen::JsValue, wasm_bindgen::JsValue, wasm_bindgen::JsValue),
        >::wrap(Box::new(move |lat_val, lon_val, _acc| {
            if let (Some(lat), Some(lon)) = (lat_val.as_f64(), lon_val.as_f64()) {
                on_usr_loc.run((lat, lon));
            }
        }));
        let _ = js_sys::Reflect::set(
            &config,
            &wasm_bindgen::JsValue::from_str("onUserLocation"),
            loc_cb.as_ref().unchecked_ref(),
        );
        loc_cb.forget();

        // onLocationError callback
        let on_loc_err = on_location_error;
        let loc_err_cb = wasm_bindgen::closure::Closure::<dyn Fn(wasm_bindgen::JsValue)>::wrap(
            Box::new(move |err_val| {
                let msg = err_val
                    .as_string()
                    .unwrap_or_else(|| "Unable to retrieve location".to_string());
                on_loc_err.run(msg);
            }),
        );
        let _ = js_sys::Reflect::set(
            &config,
            &wasm_bindgen::JsValue::from_str("onLocationError"),
            loc_err_cb.as_ref().unchecked_ref(),
        );
        loc_err_cb.forget();

        match crate::map::LeafletMap::new("map-canvas", &config) {
            Ok(m) => {
                if let Ok(mut lock) = map_cell_mount.lock() {
                    *lock = Some(m);
                }
            }
            Err(e) => {
                web_sys::console::error_1(&e);
            }
        }
    });

    // Reframe selection when mobile panel snap state changes
    let map_cell_snap = map_cell.clone();
    Effect::new(move |_| {
        let _ = mobile_snap.get();
        let map_cell_inner = map_cell_snap.clone();
        gloo_timers::callback::Timeout::new(320, move || {
            if let Ok(lock) = map_cell_inner.lock() {
                if let Some(map) = lock.as_ref() {
                    map.reframe_selection();
                    map.reframe_overview();
                }
            }
        })
        .forget();
    });

    // Overview geometry loader & Places synchronization
    let loader_state =
        std::sync::Arc::new(std::sync::Mutex::new(crate::loader::LoaderState::new()));
    let (geometry_error, set_geometry_error) = signal(None::<String>);
    let (roads_limit_exceeded, set_roads_limit_exceeded) = signal(false);

    let on_retry_geometry = Callback::new(move |_| {
        set_geometry_error.set(None);
        set_state.update(|st| {
            st.filter_generation = st.filter_generation.wrapping_add(1);
        });
    });

    let map_cell_sync = map_cell.clone();
    let loader_state_sync = loader_state.clone();
    let nearby_keys = std::sync::Arc::new(std::sync::Mutex::new(Vec::<ObjectKey>::new()));
    let nearby_token = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    Effect::new(move |_| {
        let cat_opt = catalog.get();
        let cur_state = state.get();
        let generation = cur_state.filter_generation;

        if let (Some(cat), Ok(lock)) = (cat_opt, map_cell_sync.lock()) {
            if let Some(map) = lock.as_ref() {
                let token = nearby_token.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                if let Ok(mut keys) = nearby_keys.lock() {
                    for key in keys.drain(..) {
                        map.remove_overview(&key);
                    }
                }
                // 1. Synchronize Places
                if cur_state.places_overlay_visible || cur_state.filter.kind == ObjectKind::Place {
                    let place_keys = cur_state.visible_place_keys(&cat);
                    let mut points = Vec::new();
                    for key in place_keys {
                        if let Some(obj) = cat.objects.iter().find(|o| o.key == key) {
                            if let Some(coords) = obj.point {
                                points.push(serde_json::json!({
                                    "key": key,
                                    "coordinates": coords,
                                    "title": obj.title,
                                    "place_category": obj.place_category,
                                }));
                            }
                        }
                    }
                    if let Ok(json_str) = serde_json::to_string(&points) {
                        if let Ok(js_points) = js_sys::JSON::parse(&json_str) {
                            map.set_places(&js_points);
                        }
                    }
                } else {
                    if let Ok(json_str) = serde_json::to_string(&Vec::<serde_json::Value>::new()) {
                        if let Ok(js_points) = js_sys::JSON::parse(&json_str) {
                            map.set_places(&js_points);
                        }
                    }
                }

                // Nearby routes use full geometry, independent of route filters.
                if cur_state.filter.kind == ObjectKind::Place
                    && !cur_state
                        .selected_key
                        .as_ref()
                        .is_some_and(|key| matches!(key.kind, ObjectKind::Route | ObjectKind::Road))
                {
                    if let Ok(mut loader) = loader_state_sync.lock() {
                        for key in loader.set_target_candidates(generation, Vec::new()).0 {
                            map.remove_overview(&key);
                        }
                    }
                    set_roads_limit_exceeded.set(false);
                    if cur_state.nearby_routes_visible {
                        let keys = if let Some(key) = cur_state
                            .selected_key
                            .as_ref()
                            .filter(|k| k.kind == ObjectKind::Place)
                        {
                            vec![key.clone()]
                        } else {
                            crate::search::filter_summaries_with_saved(
                                &cat,
                                &cur_state.filter,
                                Some(&cur_state.saved_keys),
                            )
                        };
                        let points: Vec<_> = cat
                            .objects
                            .iter()
                            .filter(|o| keys.contains(&o.key))
                            .filter_map(|o| o.point)
                            .collect();
                        let routes: Vec<_> = cat
                            .objects
                            .iter()
                            .filter(|o| {
                                o.key.kind == ObjectKind::Route
                                    && o.geometry_url.is_some()
                                    && points.iter().any(|p| {
                                        let latitude_margin = 1609.344 / 111_000.0;
                                        let longitude_margin =
                                            latitude_margin / p[1].to_radians().cos();
                                        o.bounds[0] <= p[0] + longitude_margin
                                            && o.bounds[2] >= p[0] - longitude_margin
                                            && o.bounds[1] <= p[1] + latitude_margin
                                            && o.bounds[3] >= p[1] - latitude_margin
                                    })
                            })
                            .cloned()
                            .collect();
                        let map_clone = map_cell_sync.clone();
                        let keys_clone = nearby_keys.clone();
                        let active = nearby_token.clone();
                        leptos::task::spawn_local(async move {
                            for route in routes {
                                if active.load(std::sync::atomic::Ordering::SeqCst) != token {
                                    return;
                                }
                                let Some(url) = route.geometry_url else {
                                    continue;
                                };
                                let Ok(response) = gloo_net::http::Request::get(&url).send().await
                                else {
                                    continue;
                                };
                                let Ok(mut feature) = response.json::<serde_json::Value>().await
                                else {
                                    continue;
                                };
                                if active.load(std::sync::atomic::Ordering::SeqCst) != token {
                                    return;
                                }
                                let coordinates: Vec<[f64; 2]> = feature
                                    .pointer("/geometry/coordinates")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                                    .unwrap_or_default();
                                if !crate::search::route_near_places(&coordinates, &points) {
                                    continue;
                                }
                                if let Some(color) = route.color {
                                    feature["properties"]["color"] = color.into();
                                }
                                if let Ok(js) = js_sys::JSON::parse(&feature.to_string()) {
                                    if let Ok(lock) = map_clone.lock() {
                                        if let Some(map) = lock.as_ref() {
                                            map.set_overview(&route.key, &js);
                                        }
                                    }
                                    if let Ok(mut keys) = keys_clone.lock() {
                                        keys.push(route.key);
                                    }
                                }
                            }
                        });
                    }
                    return;
                }

                // 2. Synchronize Overview Lines
                if cur_state.selected_key.is_some() {
                    set_roads_limit_exceeded.set(false);
                    let to_remove = if let Ok(mut l) = loader_state_sync.lock() {
                        l.set_target_candidates(generation, Vec::new()).0
                    } else {
                        Vec::new()
                    };
                    for key in to_remove {
                        map.remove_overview(&key);
                    }
                } else {
                    let Some(vp_bounds) = cur_state.viewport_bounds else {
                        return;
                    };

                    let eligible_summaries = crate::search::filter_summaries_for_overview(
                        &cat,
                        &cur_state.filter,
                        &vp_bounds,
                    );

                    let candidates: Vec<crate::loader::OverviewCandidate> = eligible_summaries
                        .into_iter()
                        .filter_map(|summary| {
                            summary.overview_url.clone().map(|url| {
                                crate::loader::OverviewCandidate {
                                    key: summary.key.clone(),
                                    overview_url: url,
                                    bounds: summary.bounds,
                                    color: summary.color.clone(),
                                    center_distance_sq:
                                        crate::loader::distance_to_viewport_center_sq(
                                            &summary.bounds,
                                            &vp_bounds,
                                        ),
                                }
                            })
                        })
                        .collect();

                    let (top_candidates, exceeded) = crate::loader::sort_and_cap_candidates(
                        candidates,
                        cur_state.selected_key.as_ref(),
                        crate::loader::MAX_ATTACHED_LAYERS,
                    );
                    set_roads_limit_exceeded.set(exceeded);

                    let (to_remove, to_add) = if let Ok(mut l) = loader_state_sync.lock() {
                        l.set_target_candidates(generation, top_candidates)
                    } else {
                        (Vec::new(), Vec::new())
                    };

                    for key in to_remove {
                        map.remove_overview(&key);
                    }

                    for (key, feat) in to_add {
                        if let Ok(json_str) = serde_json::to_string(&feat) {
                            if let Ok(js_feat) = js_sys::JSON::parse(&json_str) {
                                map.set_overview(&key, &js_feat);
                            }
                        }
                    }

                    pump_loader_requests(&loader_state_sync, &map_cell_sync, set_geometry_error);
                }
            }
        }
    });

    on_cleanup(move || {
        if let Ok(mut lock) = map_cell_cleanup.lock() {
            if let Some(mut m) = lock.take() {
                m.destroy();
            }
        }
    });

    // Selection details & geometry loading effect
    let active_selection_token = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let map_cell_detail_eff = map_cell_detail.clone();
    let active_token_effect = active_selection_token.clone();
    let selected_key_memo = Memo::new(move |_| state.get().selected_key);

    Effect::new(move |_| {
        let sel_key = selected_key_memo.get();
        let _ = detail_reload_trigger.get();

        let cur_token = active_token_effect.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

        match sel_key {
            None => {
                set_detail_data.set(None);
                set_detail_loading.set(false);
                set_detail_error.set(None);
                if let Ok(m_lock) = map_cell_detail_eff.lock() {
                    if let Some(ref m) = *m_lock {
                        m.clear_selection();
                    }
                }
            }
            Some(key) => {
                set_detail_loading.set(true);
                set_detail_error.set(None);
                set_detail_data.set(None);

                let kind_plural = match key.kind {
                    ObjectKind::Route => "routes",
                    ObjectKind::Road => "roads",
                    ObjectKind::Place => "places",
                };
                let detail_url = format!("/data/{}/{}.json", kind_plural, key.id);
                let geom_url = format!("/data/geometry/{}/{}.geojson", kind_plural, key.id);

                let token_check = active_token_effect.clone();
                let map_cell_async = map_cell_detail_eff.clone();
                let key_clone = key.clone();

                leptos::task::spawn_local(async move {
                    let detail_res = gloo_net::http::Request::get(&detail_url).send().await;

                    if token_check.load(std::sync::atomic::Ordering::SeqCst) != cur_token {
                        return;
                    }

                    let detail_obj: catalog_model::ObjectDetail = match detail_res {
                        Ok(resp) if resp.ok() => {
                            match resp.json::<catalog_model::ObjectDetail>().await {
                                Ok(d) => d,
                                Err(_) => {
                                    if token_check.load(std::sync::atomic::Ordering::SeqCst)
                                        == cur_token
                                    {
                                        set_detail_loading.set(false);
                                        set_detail_error
                                            .set(Some("Failed to parse detail data".to_string()));
                                    }
                                    return;
                                }
                            }
                        }
                        Ok(resp) if resp.status() == 404 => {
                            if token_check.load(std::sync::atomic::Ordering::SeqCst) == cur_token {
                                set_detail_loading.set(false);
                                set_detail_error.set(Some("Object not found".to_string()));
                            }
                            return;
                        }
                        _ => {
                            if token_check.load(std::sync::atomic::Ordering::SeqCst) == cur_token {
                                set_detail_loading.set(false);
                                set_detail_error.set(Some("Failed to load details".to_string()));
                            }
                            return;
                        }
                    };

                    if token_check.load(std::sync::atomic::Ordering::SeqCst) != cur_token {
                        return;
                    }

                    if key_clone.kind == ObjectKind::Route || key_clone.kind == ObjectKind::Road {
                        let geom_res = gloo_net::http::Request::get(&geom_url).send().await;

                        if token_check.load(std::sync::atomic::Ordering::SeqCst) != cur_token {
                            return;
                        }

                        match geom_res {
                            Ok(resp) if resp.ok() => {
                                if let Ok(mut geom_val) = resp.json::<serde_json::Value>().await {
                                    if token_check.load(std::sync::atomic::Ordering::SeqCst)
                                        != cur_token
                                    {
                                        return;
                                    }

                                    if let Some(props) = geom_val
                                        .get_mut("properties")
                                        .and_then(|p| p.as_object_mut())
                                    {
                                        if let Some(ref col) = detail_obj.color {
                                            props.insert(
                                                "color".into(),
                                                serde_json::Value::String(col.clone()),
                                            );
                                        }
                                        props.insert(
                                            "title".into(),
                                            serde_json::Value::String(detail_obj.title.clone()),
                                        );
                                    }

                                    if let Ok(js_feat) = js_sys::JSON::parse(
                                        &serde_json::to_string(&geom_val).unwrap_or_default(),
                                    ) {
                                        if let Ok(m_lock) = map_cell_async.lock() {
                                            if let Some(ref m) = *m_lock {
                                                m.set_selection(&key_clone, &js_feat);
                                                m.fit_bounds([
                                                    [detail_obj.bounds[0], detail_obj.bounds[1]],
                                                    [detail_obj.bounds[2], detail_obj.bounds[3]],
                                                ]);
                                            }
                                        }
                                    }

                                    set_detail_data.set(Some(detail_obj));
                                    set_detail_loading.set(false);
                                    set_detail_error.set(None);
                                } else {
                                    if token_check.load(std::sync::atomic::Ordering::SeqCst)
                                        == cur_token
                                    {
                                        if let Ok(m_lock) = map_cell_async.lock() {
                                            if let Some(ref m) = *m_lock {
                                                m.fit_bounds([
                                                    [detail_obj.bounds[0], detail_obj.bounds[1]],
                                                    [detail_obj.bounds[2], detail_obj.bounds[3]],
                                                ]);
                                            }
                                        }
                                        set_detail_data.set(Some(detail_obj));
                                        set_detail_loading.set(false);
                                        set_detail_error
                                            .set(Some("Failed to parse geometry".to_string()));
                                    }
                                }
                            }
                            _ => {
                                if token_check.load(std::sync::atomic::Ordering::SeqCst)
                                    == cur_token
                                {
                                    if let Ok(m_lock) = map_cell_async.lock() {
                                        if let Some(ref m) = *m_lock {
                                            m.fit_bounds([
                                                [detail_obj.bounds[0], detail_obj.bounds[1]],
                                                [detail_obj.bounds[2], detail_obj.bounds[3]],
                                            ]);
                                        }
                                    }
                                    set_detail_data.set(Some(detail_obj));
                                    set_detail_loading.set(false);
                                    set_detail_error
                                        .set(Some("Failed to load geometry".to_string()));
                                }
                            }
                        }
                    } else {
                        // Place
                        if token_check.load(std::sync::atomic::Ordering::SeqCst) != cur_token {
                            return;
                        }

                        let (lon, lat) = if let Some(ref coords) = detail_obj.coordinates {
                            (coords[0], coords[1])
                        } else {
                            (detail_obj.bounds[0], detail_obj.bounds[1])
                        };

                        let point_feat = serde_json::json!({
                            "type": "Feature",
                            "properties": {
                                "title": detail_obj.title
                            },
                            "geometry": {
                                "type": "Point",
                                "coordinates": [lon, lat]
                            }
                        });

                        if let Ok(js_feat) = js_sys::JSON::parse(
                            &serde_json::to_string(&point_feat).unwrap_or_default(),
                        ) {
                            if let Ok(m_lock) = map_cell_async.lock() {
                                if let Some(ref m) = *m_lock {
                                    m.set_selection(&key_clone, &js_feat);
                                    m.fit_bounds([[lon, lat], [lon, lat]]);
                                }
                            }
                        }

                        if token_check.load(std::sync::atomic::Ordering::SeqCst) == cur_token {
                            set_detail_data.set(Some(detail_obj));
                            set_detail_loading.set(false);
                            set_detail_error.set(None);
                        }
                    }
                });
            }
        }
    });

    view! {
        <div class="app-layout" id="app">
            // SW update banner
            {move || if sw_update_available.get() {
                view! {
                    <div class="sw-update-banner" role="alert">
                        <span class="sw-update-text">"A new version of Ride Atlas is available."</span>
                        <button
                            type="button"
                            class="btn-sw-update"
                            on:click=move |_| crate::offline::apply_update()
                        >
                            "Update Now"
                        </button>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // URL warning notification
            {move || url_error.get().map(|err| {
                view! {
                    <div class="url-error-banner" role="alert">
                        <span class="url-error-text">{"Warning: "}{err}</span>
                        <button
                            type="button"
                            class="url-error-close"
                            on:click=move |_| set_url_error.set(None)
                        >
                            "✕"
                        </button>
                    </div>
                }
            })}

            // Geometry warning banner
            {move || geometry_error.get().map(|err| {
                view! {
                    <div class="geometry-warning-banner" role="alert">
                        <span class="warning-text">{err}</span>
                        <button
                            type="button"
                            class="btn-retry-geometry"
                            on:click=move |_| on_retry_geometry.run(())
                        >
                            "Retry"
                        </button>
                    </div>
                }
            })}

            // Location notice banner
            {move || location_notice.get().map(|msg| {
                view! {
                    <div class="location-notice-banner" role="status">
                        <span class="location-notice-text">{"Location: "}{msg}</span>
                        <button
                            type="button"
                            class="location-notice-close"
                            aria-label="Dismiss location notice"
                            on:click=move |_| set_location_notice.set(None)
                        >
                            "✕"
                        </button>
                    </div>
                }
            })}

            // Roads limit notice
            {move || if roads_limit_exceeded.get() {
                view! {
                    <div class="roads-limit-notice" role="status">
                        <span>"Showing 200 roads; zoom in for more"</span>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // Desktop sidebar (>= 768px)
            <aside class="sidebar" aria-label="Catalog Navigation">
                {move || {
                    if is_selected_sig.get() {
                        view! {
                            <DetailView
                                detail=detail_sig
                                loading=detail_loading_sig
                                error=detail_error_sig
                                return_route_key=return_route_key_sig
                                on_back_to_results=on_back_to_results
                                on_back_to_route=on_back_to_route
                                on_select_place=on_select_place_from_route
                                on_retry=on_retry_detail
                                is_saved=is_selected_saved_sig
                                on_toggle_save=on_toggle_save_selected
                            />
                        }.into_any()
                    } else {
                        view! {
                            <Controls
                                kind=kind_sig
                                on_kind_change=on_kind_change
                                query=query_sig
                                on_query_change=on_query_change
                                categories=categories_sig
                                on_category_toggle=on_category_toggle
                                min_distance=min_dist_sig
                                max_distance=max_dist_sig
                                on_mileage_change=on_mileage_change
                                places_overlay=places_overlay_sig
                                nearby_routes=nearby_routes_sig
                                on_nearby_routes_toggle=on_nearby_routes_toggle
                                on_places_overlay_toggle=on_places_overlay_toggle
                                saved_only=saved_only_sig
                                on_saved_toggle=on_saved_toggle
                                has_pending_bounds=pending_bounds_sig
                                has_applied_bounds=applied_bounds_sig
                                on_apply_bounds=on_apply_bounds
                                on_clear_bounds=on_clear_bounds
                                on_clear_filters=on_clear_filters
                                is_near_me=is_near_me_sig
                                has_user_location=has_user_location_sig
                                on_locate_me=on_locate_me
                                on_toggle_near_me=on_toggle_near_me
                                sort=sort_sig
                                on_sort_change=on_sort_change
                                has_places=has_places.get()
                                id_prefix="desktop"
                            />
                            <Results
                                loading=Signal::from(loading)
                                error=Signal::from(load_error)
                                on_retry=on_retry
                                items=current_page_summaries
                                total_count=total_results
                                page=page_sig
                                total_pages=total_pages
                                on_page_change=on_page_change
                                selected_key=selected_key_sig
                                on_select=on_select
                                on_clear_filters=on_clear_filters
                                on_clear_bounds=on_clear_bounds
                                saved_keys=saved_keys_sig
                                on_toggle_save=on_toggle_save
                                unavailable_keys=unavailable_keys
                                user_location=user_location_sig
                                kind=kind_sig
                                saved_only=saved_only_sig
                                catalog_places_count=catalog_places_count
                                on_browse_routes=on_browse_routes
                                on_disable_saved_only=on_disable_saved_only
                                id_prefix="desktop"
                            />
                        }.into_any()
                    }
                }}
                <footer class="sidebar-footer">
                    <a class="project-github-link" href="https://github.com/vfedoroff/central-texas-motocycle-routes-map" target="_blank" rel="noopener noreferrer" aria-label="GitHub">"GitHub"<span aria-hidden="true">" ↗"</span></a>
                    <button
                        type="button"
                        class="btn-privacy-settings"
                        on:click=move |_| set_show_privacy_dialog.set(true)
                    >
                        "Privacy & Analytics"
                    </button>
                </footer>
            </aside>

            // Map canvas area
            <main class="map-container" id="map-root">
                <div id="map-canvas" class="map-canvas"></div>
            </main>

            // Mobile bottom sheet (< 768px)
            <section class=mobile_snap_class aria-label="Mobile Catalog Panel">
                <div class="panel-header">
                    <div class="panel-handle" aria-hidden="true"></div>
                    <div class="panel-snap-controls">
                        <a class="project-github-link" href="https://github.com/vfedoroff/central-texas-motocycle-routes-map" target="_blank" rel="noopener noreferrer" aria-label="GitHub">"GitHub"<span aria-hidden="true">" ↗"</span></a>
                        <button
                            type="button"
                            class="snap-btn btn-toggle-view"
                            aria-expanded=move || if mobile_snap.get() == MobileSnapState::Collapsed { "false" } else { "true" }
                            on:click=move |_| {
                                if mobile_snap.get() == MobileSnapState::Collapsed {
                                    set_mobile_snap.set(MobileSnapState::Half);
                                } else {
                                    set_mobile_snap.set(MobileSnapState::Collapsed);
                                }
                            }
                        >
                            {move || if mobile_snap.get() == MobileSnapState::Collapsed {
                                "Show results"
                            } else {
                                "Show map"
                            }}
                        </button>
                        <button
                            type="button"
                            class="snap-btn btn-mobile-privacy"
                            aria-label="Privacy settings"
                            on:click=move |_| set_show_privacy_dialog.set(true)
                        >
                            "Privacy"
                        </button>
                    </div>
                </div>
                <div class="panel-content">
                    {move || {
                        if is_selected_sig.get() {
                            view! {
                                <DetailView
                                    detail=detail_sig
                                    loading=detail_loading_sig
                                    error=detail_error_sig
                                    return_route_key=return_route_key_sig
                                    on_back_to_results=on_back_to_results
                                    on_back_to_route=on_back_to_route
                                    on_select_place=on_select_place_from_route
                                    on_retry=on_retry_detail
                                    is_saved=is_selected_saved_sig
                                    on_toggle_save=on_toggle_save_selected
                                />
                            }.into_any()
                        } else {
                            view! {
                                <Controls
                                    kind=kind_sig
                                    on_kind_change=on_kind_change
                                    query=query_sig
                                    on_query_change=on_query_change
                                    categories=categories_sig
                                    on_category_toggle=on_category_toggle
                                    min_distance=min_dist_sig
                                    max_distance=max_dist_sig
                                    on_mileage_change=on_mileage_change
                                    places_overlay=places_overlay_sig
                                nearby_routes=nearby_routes_sig
                                on_nearby_routes_toggle=on_nearby_routes_toggle
                                    on_places_overlay_toggle=on_places_overlay_toggle
                                    saved_only=saved_only_sig
                                    on_saved_toggle=on_saved_toggle
                                    has_pending_bounds=pending_bounds_sig
                                    has_applied_bounds=applied_bounds_sig
                                    on_apply_bounds=on_apply_bounds
                                    on_clear_bounds=on_clear_bounds
                                    on_clear_filters=on_clear_filters
                                    is_near_me=is_near_me_sig
                                    has_user_location=has_user_location_sig
                                    on_locate_me=on_locate_me
                                    on_toggle_near_me=on_toggle_near_me
                                    sort=sort_sig
                                    on_sort_change=on_sort_change
                                    has_places=has_places.get()
                                    compact=true
                                    on_expand=Callback::new(move |_| set_mobile_snap.set(MobileSnapState::Expanded))
                                    id_prefix="mobile"
                                />
                                <Results
                                    loading=Signal::from(loading)
                                    error=Signal::from(load_error)
                                    on_retry=on_retry
                                    items=current_page_summaries
                                    total_count=total_results
                                    page=page_sig
                                    total_pages=total_pages
                                    on_page_change=on_page_change
                                    selected_key=selected_key_sig
                                    on_select=on_select
                                    on_clear_filters=on_clear_filters
                                    on_clear_bounds=on_clear_bounds
                                    saved_keys=saved_keys_sig
                                    on_toggle_save=on_toggle_save
                                    unavailable_keys=unavailable_keys
                                    user_location=user_location_sig
                                    kind=kind_sig
                                    saved_only=saved_only_sig
                                    catalog_places_count=catalog_places_count
                                    on_browse_routes=on_browse_routes
                                    on_disable_saved_only=on_disable_saved_only
                                    id_prefix="mobile"
                                />
                            }.into_any()
                        }
                    }}
                </div>
            </section>

            // Privacy & Analytics Settings Dialog
            {move || if show_privacy_dialog.get() {
                view! {
                    <div class="privacy-dialog-backdrop" on:click=move |_| set_show_privacy_dialog.set(false)>
                        <div
                            class="privacy-dialog"
                            role="dialog"
                            aria-modal="true"
                            aria-label="Privacy Settings"
                            on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
                        >
                            <header class="privacy-dialog-header">
                                <h3>"Privacy & Analytics Settings"</h3>
                                <button
                                    type="button"
                                    class="btn-close-privacy"
                                    aria-label="Close"
                                    on:click=move |_| set_show_privacy_dialog.set(false)
                                >
                                    "✕"
                                </button>
                            </header>
                            <div class="privacy-dialog-body">
                                <p>
                                    "Ride Atlas respects your privacy. Analytics is strictly opt-in and measures high-level catalog usage (route views and filters) only. No personal identities, search queries, or GPS locations are ever tracked."
                                </p>
                                <div class="privacy-status-section">
                                    <strong>"Current preference: "</strong>
                                    <span class="privacy-status-value">
                                        {match consent_status.get() {
                                            crate::analytics::ConsentStatus::Allowed => "Analytics Allowed",
                                            crate::analytics::ConsentStatus::Declined => "Analytics Declined",
                                            crate::analytics::ConsentStatus::Unset => "Not configured (unset)",
                                        }}
                                    </span>
                                </div>
                                <div class="privacy-toggle-actions">
                                    <button
                                        type="button"
                                        class="btn-toggle-consent-allow"
                                        on:click=move |_| {
                                            crate::analytics::set_consent(crate::analytics::ConsentStatus::Allowed);
                                            set_consent_status.set(crate::analytics::ConsentStatus::Allowed);
                                        }
                                    >
                                        "Allow Analytics"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-toggle-consent-decline"
                                        on:click=move |_| {
                                            crate::analytics::set_consent(crate::analytics::ConsentStatus::Declined);
                                            set_consent_status.set(crate::analytics::ConsentStatus::Declined);
                                        }
                                    >
                                        "Decline Analytics"
                                    </button>
                                </div>
                                <p class="privacy-notice-declined">
                                    "Declining stops further events from being tracked."
                                </p>
                                <div class="privacy-link-container">
                                    <a href="/privacy/index.html" class="privacy-policy-link" target="_blank" rel="noopener">
                                        "Read full Privacy Policy →"
                                    </a>
                                </div>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // Bottom Consent Banner (shown only when consent status is Unset)
            {move || if consent_status.get() == crate::analytics::ConsentStatus::Unset {
                view! {
                    <div class="consent-banner" role="region" aria-label="Privacy and Analytics Consent">
                        <div class="consent-banner-content">
                            <p class="consent-text">
                                "Ride Atlas uses optional analytics to improve route recommendations. No personal data, search queries, or live locations are tracked. "
                                <a href="/privacy/index.html" class="consent-privacy-link" target="_blank" rel="noopener">"Privacy Policy"</a>
                            </p>
                            <div class="consent-actions">
                                <button
                                    type="button"
                                    class="consent-btn consent-btn-allow"
                                    on:click=move |_| {
                                        crate::analytics::set_consent(crate::analytics::ConsentStatus::Allowed);
                                        set_consent_status.set(crate::analytics::ConsentStatus::Allowed);
                                    }
                                >
                                    "Allow"
                                </button>
                                <button
                                    type="button"
                                    class="consent-btn consent-btn-decline"
                                    on:click=move |_| {
                                        crate::analytics::set_consent(crate::analytics::ConsentStatus::Declined);
                                        set_consent_status.set(crate::analytics::ConsentStatus::Declined);
                                    }
                                >
                                    "Decline"
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}
        </div>
    }
}
