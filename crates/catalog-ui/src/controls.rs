#[cfg(target_arch = "wasm32")]
use catalog_model::ObjectKind;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlInputElement;

#[cfg(target_arch = "wasm32")]
use crate::search::validate_mileage;

#[cfg(target_arch = "wasm32")]
pub const ROUTE_CATEGORIES: &[(&str, &str)] = &[
    ("Twisties & Canyons", "#e74c3c"),
    ("Lakes & Rivers", "#2980b9"),
    ("German Towns & Culture", "#8e44ad"),
    ("Plains & Ranchlands", "#27ae60"),
];

#[cfg(target_arch = "wasm32")]
#[component]
pub fn Controls(
    kind: Signal<ObjectKind>,
    on_kind_change: Callback<ObjectKind>,
    query: Signal<String>,
    on_query_change: Callback<String>,
    categories: Signal<Vec<String>>,
    on_category_toggle: Callback<String>,
    min_distance: Signal<Option<f64>>,
    max_distance: Signal<Option<f64>>,
    on_mileage_change: Callback<(Option<f64>, Option<f64>)>,
    places_overlay: Signal<bool>,
    on_places_overlay_toggle: Callback<bool>,
    saved_only: Signal<bool>,
    on_saved_toggle: Callback<bool>,
    has_pending_bounds: Signal<bool>,
    #[prop(default = Signal::derive(move || false))] has_applied_bounds: Signal<bool>,
    on_apply_bounds: Callback<()>,
    #[prop(default = Callback::new(move |_| ()))] on_clear_bounds: Callback<()>,
    on_clear_filters: Callback<()>,
    is_near_me: Signal<bool>,
    has_user_location: Signal<bool>,
    on_locate_me: Callback<()>,
    on_toggle_near_me: Callback<()>,
    sort: Signal<crate::search::CatalogSort>,
    on_sort_change: Callback<crate::search::CatalogSort>,
    #[prop(default = false)] compact: bool,
    #[prop(optional)] on_expand: Option<Callback<()>>,
    #[prop(default = true)] has_places: bool,
    #[prop(default = "desktop")] id_prefix: &'static str,
) -> impl IntoView {
    let (mileage_error, set_mileage_error) = signal(Option::<String>::None);
    let (filters_open, set_filters_open) = signal(false);
    let debounce_timer = Rc::new(RefCell::new(Option::<gloo_timers::callback::Timeout>::None));

    let search_input_id = format!("{}-search-input", id_prefix);
    let tab_routes_id = format!("{}-tab-routes", id_prefix);
    let tab_places_id = format!("{}-tab-places", id_prefix);
    let results_panel_id = format!("{}-catalog-results-panel", id_prefix);
    let filters_btn_id = format!("{}-filters-btn", id_prefix);
    let filters_disclosure_id = format!("{}-filters-disclosure", id_prefix);
    let sort_select_id = format!("{}-sort-select", id_prefix);
    let sort_select_ref = NodeRef::<leptos::html::Select>::new();

    Effect::new(move |_| {
        let current_kind = kind.get();
        let current_sort = sort.get();
        let near = is_near_me.get();
        if let Some(el) = sort_select_ref.get() {
            if near {
                el.set_value("near_me");
            } else if current_kind == ObjectKind::Place {
                el.set_value("name");
            } else {
                match current_sort {
                    crate::search::CatalogSort::Title => el.set_value("name"),
                    crate::search::CatalogSort::DistanceAscending => el.set_value("distance_asc"),
                    crate::search::CatalogSort::DistanceDescending => el.set_value("distance_desc"),
                }
            }
        }
    });

    let active_filters_count = Signal::derive(move || {
        let mut count = 0;
        count += categories.get().len();
        if min_distance.get().is_some() || max_distance.get().is_some() {
            count += 1;
        }
        if saved_only.get() {
            count += 1;
        }
        if has_applied_bounds.get() {
            count += 1;
        }
        count
    });

    let tab_routes_id_focus = tab_routes_id.clone();
    let tab_places_id_focus = tab_places_id.clone();
    let on_tabs_keydown = {
        let on_kind_change = on_kind_change.clone();
        move |ev: web_sys::KeyboardEvent| {
            let k = ev.key();
            if k == "ArrowRight" || k == "ArrowLeft" {
                ev.prevent_default();
                let next_kind = if kind.get() == ObjectKind::Route {
                    ObjectKind::Place
                } else {
                    ObjectKind::Route
                };
                on_kind_change.run(next_kind);
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let id = if next_kind == ObjectKind::Route {
                        tab_routes_id_focus.as_str()
                    } else {
                        tab_places_id_focus.as_str()
                    };
                    if let Some(el) = doc
                        .get_element_by_id(id)
                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let _ = el.focus();
                    }
                }
            }
        }
    };

    let handle_input = {
        let debounce_timer = debounce_timer.clone();
        move |ev: web_sys::Event| {
            let target: HtmlInputElement = event_target(&ev);
            let val = target.value();

            // Cancel any pending timer
            if let Some(timer) = debounce_timer.borrow_mut().take() {
                drop(timer);
            }

            let debounce_timer_clone = debounce_timer.clone();
            let timeout = gloo_timers::callback::Timeout::new(150, move || {
                debounce_timer_clone.borrow_mut().take();
                on_query_change.run(val);
            });
            *debounce_timer.borrow_mut() = Some(timeout);
        }
    };

    let handle_min_dist = move |ev: web_sys::Event| {
        let target: HtmlInputElement = event_target(&ev);
        let val_str = target.value();
        let min_val = if val_str.trim().is_empty() {
            None
        } else {
            match val_str.trim().parse::<f64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    set_mileage_error.set(Some("Invalid number format".to_string()));
                    return;
                }
            }
        };

        match validate_mileage(min_val, max_distance.get()) {
            Ok(()) => {
                set_mileage_error.set(None);
                on_mileage_change.run((min_val, max_distance.get()));
            }
            Err(err) => {
                set_mileage_error.set(Some(err.to_string()));
            }
        }
    };

    let handle_max_dist = move |ev: web_sys::Event| {
        let target: HtmlInputElement = event_target(&ev);
        let val_str = target.value();
        let max_val = if val_str.trim().is_empty() {
            None
        } else {
            match val_str.trim().parse::<f64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    set_mileage_error.set(Some("Invalid number format".to_string()));
                    return;
                }
            }
        };

        match validate_mileage(min_distance.get(), max_val) {
            Ok(()) => {
                set_mileage_error.set(None);
                on_mileage_change.run((min_distance.get(), max_val));
            }
            Err(err) => {
                set_mileage_error.set(Some(err.to_string()));
            }
        }
    };

    view! {
        <div class="controls-container">
            // Brand header
            <header class="brand-header">
                <h1 class="brand-title">"Ride Atlas"</h1>
                <span class="brand-subtitle">"Central Texas Routes"</span>
            </header>

            <div class="search-and-tabs">
                // Search input
                <div class="search-bar">
                    <label for=search_input_id.clone() class="sr-only">"Search catalog"</label>
                    <input
                        id=search_input_id
                        type="search"
                        class="input-search"
                        placeholder="Search routes, places..."
                        prop:value=move || query.get()
                        on:input=handle_input
                    />
                </div>

                // Search tabs
                <nav class="tabs-nav" aria-label="Catalog sections" role="tablist" on:keydown=on_tabs_keydown>
                    <button
                        type="button"
                        role="tab"
                        id=tab_routes_id
                        aria-selected=move || if kind.get() == ObjectKind::Route { "true" } else { "false" }
                        aria-controls=results_panel_id.clone()
                        tabindex=move || if kind.get() == ObjectKind::Route { "0" } else { "-1" }
                        class=move || if kind.get() == ObjectKind::Route { "tab-btn tab-active" } else { "tab-btn" }
                        on:click=move |_| on_kind_change.run(ObjectKind::Route)
                    >
                        "Routes"
                    </button>
                    <button
                        type="button"
                        role="tab"
                        id=tab_places_id
                        aria-selected=move || if kind.get() == ObjectKind::Place { "true" } else { "false" }
                        aria-controls=results_panel_id
                        tabindex=move || if kind.get() == ObjectKind::Place { "0" } else { "-1" }
                        class=move || if kind.get() == ObjectKind::Place { "tab-btn tab-active" } else { "tab-btn" }
                        on:click=move |_| on_kind_change.run(ObjectKind::Place)
                    >
                        "Places"
                    </button>
                </nav>
                {if compact {
                    let filters_btn_id_for_btn = filters_btn_id.clone();
                    let filters_btn_id_for_close = filters_btn_id.clone();
                    let disclosure_id = filters_disclosure_id.clone();
                    let on_expand_trigger = on_expand.clone();
                    view! {
                        <button
                            type="button"
                            class=move || if filters_open.get() { "btn-toggle-filters is-open" } else { "btn-toggle-filters" }
                            id=filters_btn_id_for_btn
                            aria-expanded=move || if filters_open.get() { "true" } else { "false" }
                            aria-controls=disclosure_id
                            on:click=move |_| {
                                let next = !filters_open.get();
                                set_filters_open.set(next);
                                if next {
                                    if let Some(ref cb) = on_expand_trigger {
                                        cb.run(());
                                    }
                                } else {
                                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                        if let Some(el) = doc.get_element_by_id(&filters_btn_id_for_close) {
                                            if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                                                let _ = html.focus();
                                            }
                                        }
                                    }
                                }
                            }
                        >
                            {move || {
                                let cnt = active_filters_count.get();
                                if cnt == 0 {
                                    "Filters".to_string()
                                } else {
                                    format!("Filters ({})", cnt)
                                }
                            }}
                        </button>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </div>

            // Compact Search this area button if pending bounds
            {if compact {
                let on_apply = on_apply_bounds.clone();
                view! {
                    {move || if has_pending_bounds.get() {
                        let on_apply_click = on_apply.clone();
                        view! {
                            <div class="compact-search-area-row">
                                <button
                                    type="button"
                                    class="btn-action btn-search-area"
                                    on:click=move |_| on_apply_click.run(())
                                >
                                    "Search this area"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                }.into_any()
            } else {
                ().into_any()
            }}

            // Compact active-filter indicators
            {if compact {
                let on_category_toggle_indicators = on_category_toggle.clone();
                view! {
                    <div
                        class="active-filter-indicators"
                        style=move || {
                            if !filters_open.get() && active_filters_count.get() > 0 {
                                "display: flex;"
                            } else {
                                "display: none;"
                            }
                        }
                    >
                        {move || categories.get().into_iter().map(|cat| {
                            let cat_clone = cat.clone();
                            let on_cat = on_category_toggle_indicators.clone();
                            view! {
                                <button
                                    type="button"
                                    class="chip-active-category"
                                    on:click=move |_| on_cat.run(cat_clone.clone())
                                    aria-label=format!("Remove category {}", cat)
                                >
                                    <span>{cat.clone()}</span>
                                    <span class="chip-remove" aria-hidden="true">" ✕"</span>
                                </button>
                            }
                        }).collect_view()}
                        {move || {
                            let min = min_distance.get();
                            let max = max_distance.get();
                            if min.is_some() || max.is_some() {
                                let text = match (min, max) {
                                    (Some(mn), Some(mx)) => format!("{}-{} mi", mn, mx),
                                    (Some(mn), None) => format!("{}+ mi", mn),
                                    (None, Some(mx)) => format!("≤{} mi", mx),
                                    (None, None) => String::new(),
                                };
                                view! { <span class="indicator-badge">{text}</span> }.into_any()
                            } else {
                                ().into_any()
                            }
                        }}
                        {move || if saved_only.get() {
                            view! { <span class="indicator-badge">"Saved"</span> }.into_any()
                        } else {
                            ().into_any()
                        }}
                        {move || if has_applied_bounds.get() {
                            let on_clear_b = on_clear_bounds.clone();
                            view! {
                                <button
                                    type="button"
                                    class="chip-active-category"
                                    on:click=move |_| on_clear_b.run(())
                                    aria-label="Remove area filter"
                                >
                                    <span>"Area"</span>
                                    <span class="chip-remove" aria-hidden="true">" ✕"</span>
                                </button>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            <div
                id=filters_disclosure_id.clone()
                class=move || {
                    if compact {
                        if filters_open.get() { "filters-disclosure is-open" } else { "filters-disclosure is-collapsed" }
                    } else {
                        "filters-disclosure"
                    }
                }
                style=move || {
                    if compact && !filters_open.get() {
                        "display: none;"
                    } else {
                        "display: block;"
                    }
                }
            >
                // Sort selection
                <div class="filter-section sort-section">
                    <label for=sort_select_id.clone() class="filter-heading">"Sort by"</label>
                    <select
                        id=sort_select_id
                        node_ref=sort_select_ref
                        class="select-sort"
                        prop:value=move || {
                            if is_near_me.get() {
                                "near_me"
                            } else {
                                match sort.get() {
                                    crate::search::CatalogSort::Title => "name",
                                    crate::search::CatalogSort::DistanceAscending => "distance_asc",
                                    crate::search::CatalogSort::DistanceDescending => "distance_desc",
                                }
                            }
                        }
                        on:change={
                            let on_sort_change = on_sort_change.clone();
                            move |ev| {
                                let target: web_sys::HtmlSelectElement = event_target(&ev);
                                let val = target.value();
                                match val.as_str() {
                                    "name" => on_sort_change.run(crate::search::CatalogSort::Title),
                                    "distance_asc" => on_sort_change.run(crate::search::CatalogSort::DistanceAscending),
                                    "distance_desc" => on_sort_change.run(crate::search::CatalogSort::DistanceDescending),
                                    _ => {}
                                }
                            }
                        }
                    >
                        {move || if is_near_me.get() {
                            view! {
                                <option value="near_me" selected=true>"Distance to you (near me)"</option>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}
                        <option value="name" selected=move || sort.get() == crate::search::CatalogSort::Title && !is_near_me.get()>"Name"</option>
                        {move || if kind.get() == ObjectKind::Route {
                            view! {
                                <option value="distance_asc" selected=move || sort.get() == crate::search::CatalogSort::DistanceAscending && !is_near_me.get()>"Distance: shortest first"</option>
                                <option value="distance_desc" selected=move || sort.get() == crate::search::CatalogSort::DistanceDescending && !is_near_me.get()>"Distance: longest first"</option>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}
                    </select>
                </div>

                // Route-specific filters (Category & Mileage)
                <div
                    class="route-filters"
                    style=move || if kind.get() == ObjectKind::Route { "display: block;" } else { "display: none;" }
                >
                    <div class="filter-section">
                        <span class="filter-heading">"Categories"</span>
                        <div class="category-pills">
                            {ROUTE_CATEGORIES.iter().map(|&(cat_name, cat_color)| {
                                let name_str = cat_name.to_string();
                                let is_selected_class = {
                                    let name = name_str.clone();
                                    move || categories.get().contains(&name)
                                };
                                let is_selected_aria = {
                                    let name = name_str.clone();
                                    move || categories.get().contains(&name)
                                };
                                let cat_toggle = {
                                    let name = name_str.clone();
                                    move |_| on_category_toggle.run(name.clone())
                                };

                                view! {
                                    <button
                                        type="button"
                                        class=move || if is_selected_class() { "cat-pill cat-active" } else { "cat-pill" }
                                        aria-pressed=move || if is_selected_aria() { "true" } else { "false" }
                                        style=move || format!("--cat-color: {};", cat_color)
                                        on:click=cat_toggle
                                    >
                                        <span class="cat-dot" style=format!("background-color: {};", cat_color)></span>
                                        {cat_name}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    <div class="filter-section mileage-section">
                        <span class="filter-heading">"Distance (miles)"</span>
                        <div class="mileage-inputs">
                            <label class="mileage-label">
                                <span class="sr-only">"Min miles"</span>
                                <input
                                    type="number"
                                    class="input-mileage"
                                    placeholder="Min mi"
                                    min="0"
                                    prop:value=move || min_distance.get().map(|v| v.to_string()).unwrap_or_default()
                                    on:change=handle_min_dist
                                />
                            </label>
                            <span class="mileage-sep">"–"</span>
                            <label class="mileage-label">
                                <span class="sr-only">"Max miles"</span>
                                <input
                                    type="number"
                                    class="input-mileage"
                                    placeholder="Max mi"
                                    min="0"
                                    prop:value=move || max_distance.get().map(|v| v.to_string()).unwrap_or_default()
                                    on:change=handle_max_dist
                                />
                            </label>
                        </div>
                        {move || mileage_error.get().map(|err| {
                            view! {
                                <div class="inline-error" role="alert">{err}</div>
                            }
                        })}
                    </div>
                </div>

                // Places overlay toggle & actions
                <div class="overlay-and-actions">
                    {move || if has_places && kind.get() == ObjectKind::Route {
                        view! {
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    class="checkbox-places"
                                    prop:checked=move || places_overlay.get()
                                    on:change=move |ev| {
                                        let target: HtmlInputElement = event_target(&ev);
                                        on_places_overlay_toggle.run(target.checked());
                                    }
                                />
                                <span>"Show places on map"</span>
                            </label>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}

                    <label class="checkbox-label">
                        <input
                            type="checkbox"
                            class="checkbox-saved"
                            prop:checked=move || saved_only.get()
                            on:change=move |ev| {
                                let target: HtmlInputElement = event_target(&ev);
                                on_saved_toggle.run(target.checked());
                            }
                        />
                        <span>"Saved only"</span>
                    </label>

                    <div class="filter-actions">
                        <button
                            type="button"
                            class=move || if is_near_me.get() {
                                "btn-action btn-near-me is-active"
                            } else {
                                "btn-action btn-near-me"
                            }
                            aria-pressed=move || if is_near_me.get() { "true" } else { "false" }
                            title=move || if is_near_me.get() {
                                "Proximity sorting active. Click to toggle."
                            } else if has_user_location.get() {
                                "Sort routes by proximity to your location"
                            } else {
                                "Request GPS location to find routes near you"
                            }
                            on:click=move |_| {
                                if has_user_location.get() {
                                    on_toggle_near_me.run(());
                                } else {
                                    on_locate_me.run(());
                                }
                            }
                        >
                            {move || if is_near_me.get() {
                                view! {
                                    <span>
                                        <span class="near-me-dot" aria-hidden="true"></span>
                                        "Near me (Active)"
                                    </span>
                                }.into_any()
                            } else {
                                view! {
                                    <span>
                                        <span aria-hidden="true">"📍 "</span>
                                        "Find routes near me"
                                    </span>
                                }.into_any()
                            }}
                        </button>

                        {if !compact {
                            let on_apply = on_apply_bounds.clone();
                            view! {
                                {move || if has_pending_bounds.get() {
                                    let on_apply_c = on_apply.clone();
                                    view! {
                                        <button
                                            type="button"
                                            class="btn-action btn-search-area"
                                            on:click=move |_| on_apply_c.run(())
                                        >
                                            "Search this area"
                                        </button>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                            }.into_any()
                        } else {
                            ().into_any()
                        }}

                        <button
                            type="button"
                            class="btn-action btn-clear-filters"
                            on:click=move |_| on_clear_filters.run(())
                        >
                            "Clear filters"
                        </button>
                    </div>
                </div>

                {if compact {
                    let filters_btn_id_for_done = filters_btn_id.clone();
                    view! {
                        <div class="filters-disclosure-footer">
                            <button
                                type="button"
                                class="btn-done-filters"
                                on:click=move |_| {
                                    set_filters_open.set(false);
                                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                        if let Some(el) = doc.get_element_by_id(&filters_btn_id_for_done) {
                                            if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                                                let _ = html.focus();
                                            }
                                        }
                                    }
                                }
                            >
                                "Done"
                            </button>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </div>
        </div>
    }
}
