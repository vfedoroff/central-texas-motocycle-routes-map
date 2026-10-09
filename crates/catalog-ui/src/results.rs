#[cfg(target_arch = "wasm32")]
use catalog_model::{CatalogSummary, ObjectKey};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::collections::HashSet;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
pub fn category_dark_text_color(category: &str, _fallback_color: &str) -> &'static str {
    match category {
        "Twisties & Canyons" => "#991b1b",
        "Lakes & Rivers" => "#075985",
        "German Towns & Culture" => "#581c87",
        "Plains & Ranchlands" => "#14532d",
        _ => "#1e293b",
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn Results(
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    on_retry: Callback<()>,
    items: Signal<Vec<CatalogSummary>>,
    total_count: Signal<usize>,
    page: Signal<usize>,
    total_pages: Signal<usize>,
    on_page_change: Callback<usize>,
    selected_key: Signal<Option<ObjectKey>>,
    on_select: Callback<ObjectKey>,
    on_clear_filters: Callback<()>,
    on_clear_bounds: Callback<()>,
    saved_keys: Signal<HashSet<ObjectKey>>,
    on_toggle_save: Callback<ObjectKey>,
    unavailable_keys: Signal<Vec<ObjectKey>>,
    user_location: Signal<Option<[f64; 2]>>,
    kind: Signal<catalog_model::ObjectKind>,
    saved_only: Signal<bool>,
    catalog_places_count: Signal<usize>,
    on_browse_routes: Callback<()>,
    on_disable_saved_only: Callback<()>,
    #[prop(default = "desktop")] id_prefix: &'static str,
) -> impl IntoView {
    let is_prev_disabled = move || page.get() == 0;
    let is_next_disabled = move || page.get() + 1 >= total_pages.get();
    let on_prev_click = move |_| {
        if page.get() > 0 {
            on_page_change.run(page.get() - 1);
        }
    };
    let on_next_click = move |_| {
        if page.get() + 1 < total_pages.get() {
            on_page_change.run(page.get() + 1);
        }
    };

    let results_panel_id = format!("{}-catalog-results-panel", id_prefix);
    let tab_routes_id = format!("{}-tab-routes", id_prefix);
    let tab_places_id = format!("{}-tab-places", id_prefix);
    let labelled_by = move || {
        if kind.get() == catalog_model::ObjectKind::Route {
            tab_routes_id.clone()
        } else {
            tab_places_id.clone()
        }
    };

    view! {
        <div class="results-container" id=results_panel_id role="region" aria-labelledby=labelled_by>
            // Loading state
            {move || if loading.get() {
                view! {
                    <div class="state-message state-loading" role="status">
                        <div class="spinner"></div>
                        <p>"Loading catalog..."</p>
                    </div>
                }.into_any()
            } else if let Some(err) = error.get() {
                // Error state with Retry button
                view! {
                    <div class="state-message state-error" role="alert">
                        <p class="error-text">{"Failed to load catalog: "}{err}</p>
                        <button
                            type="button"
                            class="btn-retry"
                            on:click=move |_| on_retry.run(())
                        >
                            "Retry"
                        </button>
                    </div>
                }.into_any()
            } else if total_count.get() == 0 && unavailable_keys.get().is_empty() {
                // Empty state distinguishing empty places catalog, empty saved list, and filtered empty state
                view! {
                    <div class="state-message state-empty">
                        {move || {
                            let current_kind = kind.get();
                            let is_saved_only = saved_only.get();
                            let places_count = catalog_places_count.get();
                            let has_saved = !saved_keys.get().is_empty();

                            if current_kind == catalog_model::ObjectKind::Place && places_count == 0 {
                                // UX-02: Empty places catalog
                                view! {
                                    <p class="empty-title">"No places have been added yet"</p>
                                    <p class="empty-hint">"Points of interest, scenic stops, and destinations will appear here as they are added to the catalog."</p>
                                    <div class="empty-actions">
                                        <button
                                            type="button"
                                            class="btn-action btn-browse-routes"
                                            on:click=move |_| on_browse_routes.run(())
                                        >
                                            "Browse routes"
                                        </button>
                                    </div>
                                }.into_any()
                            } else if is_saved_only && !has_saved {
                                // UX-03: Empty saved list
                                let (title, hint) = if current_kind == catalog_model::ObjectKind::Route {
                                    ("You haven't saved any routes yet", "Tap Save on a route to find it here.")
                                } else {
                                    ("You haven't saved any places yet", "Tap Save on a place to find it here.")
                                };
                                view! {
                                    <p class="empty-title">{title}</p>
                                    <p class="empty-hint">{hint}</p>
                                    <div class="empty-actions">
                                        <button
                                            type="button"
                                            class="btn-action btn-browse-all"
                                            on:click=move |_| on_disable_saved_only.run(())
                                        >
                                            "Browse routes"
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                // Filtered empty state
                                view! {
                                    <p class="empty-title">"No matching results found"</p>
                                    <p class="empty-hint">"Try clearing active filters or searching a wider area."</p>
                                    <div class="empty-actions">
                                        <button
                                            type="button"
                                            class="btn-action btn-clear-filters"
                                            on:click=move |_| on_clear_filters.run(())
                                        >
                                            "Clear filters"
                                        </button>
                                        <button
                                            type="button"
                                            class="btn-action btn-search-all"
                                            on:click=move |_| on_clear_bounds.run(())
                                        >
                                            "Search all areas"
                                        </button>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                }.into_any()
            } else {
                // Results list with header and pagination
                view! {
                    <div class="results-content">
                        <div class="results-header">
                            <span class="results-count">
                                {move || {
                                    let extra = unavailable_keys.get().len();
                                    let count = total_count.get() + extra;
                                    if total_pages.get() > 1 {
                                        format!("{} results (Page {} of {})", count, page.get() + 1, total_pages.get())
                                    } else {
                                        format!("{} results", count)
                                    }
                                }}
                            </span>
                        </div>

                        <ul class="results-list" role="list">
                            {move || items.get().into_iter().map(|summary| {
                                let key = summary.key.clone();
                                let is_selected = {
                                    let key = key.clone();
                                    move || selected_key.get().as_ref() == Some(&key)
                                };
                                let on_click_select = {
                                    let key = key.clone();
                                    move |_| on_select.run(key.clone())
                                };

                                let is_saved_class = {
                                    let key = key.clone();
                                    move || saved_keys.get().contains(&key)
                                };
                                let is_saved_aria = {
                                    let key = key.clone();
                                    move || saved_keys.get().contains(&key)
                                };
                                let is_saved_text = {
                                    let key = key.clone();
                                    move || saved_keys.get().contains(&key)
                                };
                                let on_click_save = {
                                    let key = key.clone();
                                    move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        on_toggle_save.run(key.clone());
                                    }
                                };

                                let cat_badge = summary.category.as_ref().map(|cat| {
                                    let color = summary.color.as_deref().unwrap_or("#64748b");
                                    let text_color = category_dark_text_color(cat, color);
                                    view! {
                                        <span class="badge-category" style=format!("background-color: {}20; color: {};", color, text_color)>
                                            <span class="badge-dot" style=format!("background-color: {};", color)></span>
                                            {cat.clone()}
                                        </span>
                                    }
                                });

                                let distance_badge = summary.distance_mi.map(|dist| {
                                    view! {
                                        <span class="badge-distance">
                                            {format!("{:.1} mi", dist)}
                                        </span>
                                    }
                                });

                                let route_type_badge = summary.route_type.map(|rt| {
                                    let label = match rt {
                                        catalog_model::RouteType::Loop => "Loop",
                                        catalog_model::RouteType::Corridor => "Corridor",
                                    };
                                    view! {
                                        <span class="badge-route-type">{label}</span>
                                    }
                                });

                                let start_badge = summary.start_label.as_ref().map(|start| {
                                    view! {
                                        <span class="badge-start-label">
                                            {format!("Starts at: {}", start)}
                                        </span>
                                    }
                                });

                                let place_badge = summary.place_category.map(|pc| {
                                    view! {
                                        <span class="badge-place">
                                            {format!("{:?}", pc).to_lowercase()}
                                        </span>
                                    }
                                });

                                let proximity_badge = user_location.get().map(|user_loc| {
                                    let dist = crate::search::distance_to_user_mi(&summary, &user_loc);
                                    view! {
                                        <span class="badge-proximity" title="Distance from your location">
                                            {format!("📍 {:.1} mi away", dist)}
                                        </span>
                                    }
                                });

                                let maps_link = if summary.key.kind == catalog_model::ObjectKind::Place {
                                    summary.point.map(|point| {
                                        view! {
                                            <a
                                                class="btn-card-maps"
                                                aria-label="Open in Google Maps"
                                                href=format!("https://www.google.com/maps/search/?api=1&query={},{}", point[1], point[0])
                                                target="_blank"
                                                rel="noopener noreferrer"
                                                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
                                                on:keydown=move |ev: web_sys::KeyboardEvent| ev.stop_propagation()
                                            >
                                                "Open in Google Maps"
                                            </a>
                                        }
                                    })
                                } else {
                                    None
                                };

                                let card_id = format!("{}-card-{}", id_prefix, summary.key.id);
                                let title_for_save = summary.title.clone();
                                let on_keydown_select = {
                                    let key = key.clone();
                                    move |ev: web_sys::KeyboardEvent| {
                                        let k = ev.key();
                                        if k == "Enter" || k == " " {
                                            if let Some(target) = ev.target() {
                                                if let Ok(target_el) = target.dyn_into::<web_sys::Element>() {
                                                    if target_el.class_list().contains("btn-card-save")
                                                        || target_el.class_list().contains("btn-card-maps")
                                                    {
                                                        return;
                                                    }
                                                }
                                            }
                                            ev.prevent_default();
                                            on_select.run(key.clone());
                                        }
                                    }
                                };

                                view! {
                                    <li class="result-item">
                                        <article
                                            id=card_id
                                            class=move || if is_selected() { "result-card card-selected" } else { "result-card" }
                                            role="button"
                                            tabindex="0"
                                            data-kind=format!("{:?}", summary.key.kind).to_lowercase()
                                            data-id=summary.key.id.clone()
                                            on:click=on_click_select
                                            on:keydown=on_keydown_select
                                        >
                                            <div class="card-header">
                                                <h2 class="card-title">{summary.title.clone()}</h2>
                                                <button
                                                    type="button"
                                                    class=move || if is_saved_class() { "btn-card-save is-saved" } else { "btn-card-save" }
                                                    aria-label={
                                                        let title_clone = title_for_save.clone();
                                                        move || if is_saved_aria() {
                                                            format!("Remove {} from saved", title_clone)
                                                        } else {
                                                            format!("Save {}", title_clone)
                                                        }
                                                    }
                                                    on:click=on_click_save
                                                >
                                                    {move || if is_saved_text() { "★ Saved" } else { "☆ Save" }}
                                                </button>
                                            </div>
                                            <div class="card-badges">
                                                {cat_badge}
                                                {route_type_badge}
                                                {distance_badge}
                                                {start_badge}
                                                {place_badge}
                                                {proximity_badge}
                                            </div>
                                            <p class="card-summary">{summary.summary.clone()}</p>
                                            {maps_link}
                                        </article>
                                    </li>
                                }
                            }).collect_view()}

                            // Unavailable saved objects (if in saved_only mode)
                            {move || {
                                let unavail = unavailable_keys.get();
                                unavail.into_iter().map(|k| {
                                    let remove_key = k.clone();
                                    view! {
                                        <li class="result-item result-item-unavailable">
                                            <div class="result-card card-unavailable">
                                                <div class="card-header">
                                                    <h2 class="card-title">"Unavailable " {format!("{:?}", k.kind).to_lowercase()}</h2>
                                                    <button
                                                        type="button"
                                                        class="btn-card-save btn-remove-saved is-saved"
                                                        aria-label="Remove unavailable saved item"
                                                        on:click=move |_| on_toggle_save.run(remove_key.clone())
                                                    >
                                                        "Remove"
                                                    </button>
                                                </div>
                                                <p class="card-summary">"This item is no longer available in the catalog."</p>
                                            </div>
                                        </li>
                                    }
                                }).collect_view()
                            }}
                        </ul>

                        // Pagination controls
                        {move || {
                            if total_pages.get() > 1 {
                                view! {
                                    <nav class="pagination-nav" aria-label="Pagination">
                                        <button
                                            type="button"
                                            class="btn-page btn-prev"
                                            disabled=is_prev_disabled
                                            on:click=on_prev_click
                                        >
                                            "Previous"
                                        </button>
                                        <span class="page-indicator">
                                            {move || format!("Page {} of {}", page.get() + 1, total_pages.get())}
                                        </span>
                                        <button
                                            type="button"
                                            class="btn-page btn-next"
                                            disabled=is_next_disabled
                                            on:click=on_next_click
                                        >
                                            "Next"
                                        </button>
                                    </nav>
                                }.into_any()
                            } else {
                                ().into_any()
                            }
                        }}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
