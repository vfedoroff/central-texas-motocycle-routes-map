use catalog_model::{NavigationMode, ObjectDetail, ObjectKey, ObjectKind, RouteType};
use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

#[component]
pub fn DetailView(
    detail: Signal<Option<ObjectDetail>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    return_route_key: Signal<Option<ObjectKey>>,
    on_back_to_results: Callback<()>,
    on_back_to_route: Callback<()>,
    on_select_place: Callback<ObjectKey>,
    on_retry: Callback<()>,
    is_saved: Signal<bool>,
    on_toggle_save: Callback<()>,
) -> impl IntoView {
    Effect::new(move |_| {
        // Move keyboard focus to primary back button when detail view opens
        gloo_timers::callback::Timeout::new(40, move || {
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Ok(nodes) =
                    doc.query_selector_all(".btn-back-to-results, .btn-back-to-route")
                {
                    let mut fallback = None;
                    for i in 0..nodes.length() {
                        if let Some(node) = nodes.item(i) {
                            if let Ok(el) = node.dyn_into::<web_sys::HtmlElement>() {
                                if el.offset_parent().is_some() || el.client_width() > 0 {
                                    let _ = el.focus();
                                    return;
                                }
                                if fallback.is_none() {
                                    fallback = Some(el);
                                }
                            }
                        }
                    }
                    if let Some(el) = fallback {
                        let _ = el.focus();
                    }
                }
            }
        })
        .forget();
    });

    let (copy_status, set_copy_status) = signal(Option::<String>::None);
    let (show_manual_copy, set_show_manual_copy) = signal(false);
    let (manual_url, set_manual_url) = signal(String::new());

    let on_copy_route_link = move |route_id: String, title: String| {
        set_copy_status.set(None);
        let mut deep_url = String::new();
        if let Some(win) = web_sys::window() {
            if let Ok(origin) = win.location().origin() {
                deep_url = format!("{}/routes/{}/", origin.trim_end_matches('/'), route_id);
            }
        }
        if deep_url.is_empty() {
            deep_url = format!("/routes/{}/", route_id);
        }

        let url_to_copy = deep_url.clone();
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(win) = web_sys::window() {
                let nav = win.navigator();
                if let Ok(value) =
                    js_sys::Reflect::get(nav.as_ref(), &wasm_bindgen::JsValue::from_str("share"))
                {
                    if let Some(share) = value.dyn_ref::<js_sys::Function>() {
                        let data = js_sys::Object::new();
                        let _ = js_sys::Reflect::set(&data, &"title".into(), &title.into());
                        let _ =
                            js_sys::Reflect::set(&data, &"url".into(), &url_to_copy.clone().into());
                        if let Ok(result) = share.call1(nav.as_ref(), &data) {
                            let promise = js_sys::Promise::resolve(&result);
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(err) =
                                    wasm_bindgen_futures::JsFuture::from(promise).await
                                {
                                    let name = js_sys::Reflect::get(&err, &"name".into())
                                        .ok()
                                        .and_then(|v| v.as_string());
                                    if name.as_deref() != Some("AbortError") {
                                        match wasm_bindgen_futures::JsFuture::from(
                                            nav.clipboard().write_text(&url_to_copy),
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                set_copy_status
                                                    .set(Some("Link copied".to_string()));
                                                set_show_manual_copy.set(false);
                                            }
                                            Err(_) => {
                                                set_show_manual_copy.set(true);
                                                set_manual_url.set(url_to_copy);
                                            }
                                        }
                                    }
                                }
                            });
                            return;
                        }
                    }
                }
                let clipboard = nav.clipboard();
                let promise = clipboard.write_text(&url_to_copy);
                let url_for_fallback = url_to_copy.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match wasm_bindgen_futures::JsFuture::from(promise).await {
                        Ok(_) => {
                            set_copy_status.set(Some("Link copied".to_string()));
                            set_show_manual_copy.set(false);
                        }
                        Err(_) => {
                            set_show_manual_copy.set(true);
                            set_manual_url.set(url_for_fallback);
                        }
                    }
                });
            } else {
                set_show_manual_copy.set(true);
                set_manual_url.set(url_to_copy);
            }
        }
    };

    view! {
        <article class="detail-view">
            <header class="detail-nav-header">
                {move || {
                    if return_route_key.get().is_some() {
                        view! {
                            <button
                                type="button"
                                class="btn-back-to-route"
                                on:click=move |_| on_back_to_route.run(())
                            >
                                "← Back to route"
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <button
                                type="button"
                                class="btn-back-to-results"
                                on:click=move |_| on_back_to_results.run(())
                            >
                                "← Back to results"
                            </button>
                        }.into_any()
                    }
                }}
                {move || {
                    let has_valid_object = detail.get().is_some();
                    let currently_saved = is_saved.get();
                    if has_valid_object || currently_saved {
                        let item_title = detail.get().map(|d| d.title.clone()).unwrap_or_else(|| "object".to_string());
                        view! {
                            <button
                                type="button"
                                class=move || if is_saved.get() { "btn-detail-save is-saved" } else { "btn-detail-save" }
                                aria-label={
                                    let title = item_title.clone();
                                    move || if is_saved.get() {
                                        format!("Remove {} from saved", title)
                                    } else {
                                        format!("Save {}", title)
                                    }
                                }
                                on:click=move |_| on_toggle_save.run(())
                            >
                                {move || if is_saved.get() { "★ Saved" } else { "☆ Save" }}
                            </button>
                        }.into_any()
                    } else {
                        view! { <span class="detail-header-spacer"></span> }.into_any()
                    }
                }}
                <button
                    type="button"
                    class="btn-close-detail"
                    aria-label="Close details"
                    on:click=move |_| on_back_to_results.run(())
                >
                    "✕"
                </button>
            </header>

            {move || {
                if loading.get() {
                    view! {
                        <div class="detail-loading" role="status">
                            <span class="loading-spinner"></span>
                            <span>"Loading details..."</span>
                        </div>
                    }.into_any()
                } else if let Some(err_msg) = error.get() {
                    let is_not_found = err_msg == "Object not found";
                    view! {
                        <div class="detail-error" role="alert">
                            <p class="detail-error-message">{err_msg}</p>
                            {if !is_not_found {
                                view! {
                                    <button
                                        type="button"
                                        class="btn-retry-detail"
                                        on:click=move |_| on_retry.run(())
                                    >
                                        "Retry"
                                    </button>
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        type="button"
                                        class="btn-retry-detail"
                                        on:click=move |_| on_back_to_results.run(())
                                    >
                                        "Back to results"
                                    </button>
                                }.into_any()
                            }}
                        </div>
                    }.into_any()
                } else if let Some(d) = detail.get() {
                    let page_url = match d.key.kind {
                        ObjectKind::Route => format!("/routes/{}/", d.key.id),
                        ObjectKind::Road => format!("/roads/{}/", d.key.id),
                        ObjectKind::Place => format!("/places/{}/", d.key.id),
                    };

                    view! {
                        <div class="detail-content">
                            <div class="detail-header-info">
                                <h2 class="detail-title">{d.title.clone()}</h2>

                                {d.category.as_ref().map(|cat| {
                                    let col = d.color.clone().unwrap_or_else(|| "#3b82f6".to_string());
                                    let badge_col = match cat.as_str() {
                                        "Twisties & Canyons" => "#b91c1c",
                                        "Lakes & Rivers" => "#1d4ed8",
                                        "German Towns & Culture" => "#7e22ce",
                                        "Plains & Ranchlands" => "#15803d",
                                        _ => col.as_str(),
                                    };
                                    view! {
                                        <span class="detail-category-badge" style=format!("background-color: {};", badge_col)>
                                            {cat.clone()}
                                        </span>
                                    }
                                })}

                                {d.place_category.as_ref().map(|pcat| {
                                    view! {
                                        <span class="detail-place-category-badge">
                                            {format!("{:?}", pcat)}
                                        </span>
                                    }
                                })}

                                <p class="detail-summary">{d.summary.clone()}</p>
                                <div class="detail-share-actions">
                                {if d.key.kind == ObjectKind::Route {
                                    let rid = d.key.id.clone();
                                    let share_title = d.title.clone();
                                    let on_copy = on_copy_route_link.clone();
                                    view! {
                                        <button
                                            type="button"
                                            class="detail-link btn-copy-route-link"
                                            on:click=move |_| on_copy(rid.clone(), share_title.clone())
                                        >
                                            <svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: middle; margin-right: 0.4rem">
                                                <path d="M12 16V3m-5 5 5-5 5 5M5 13v7h14v-7" />
                                            </svg>"Share ride"
                                        </button>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}

                                <div class="share-status" role="status" aria-live="polite">
                                    {move || copy_status.get().unwrap_or_default()}
                                </div>

                                {move || if show_manual_copy.get() {
                                    view! {
                                        <div class="manual-copy-container">
                                            <label for="manual-copy-input" class="manual-copy-label">"Copy this link manually:"</label>
                                            <input
                                                id="manual-copy-input"
                                                type="text"
                                                readonly
                                                class="input-manual-copy"
                                                prop:value=move || manual_url.get()
                                                on:focus=move |ev| {
                                                    if let Ok(input) = event_target::<web_sys::HtmlInputElement>(&ev).dyn_into::<web_sys::HtmlInputElement>() {
                                                        input.select();
                                                    }
                                                }
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    ().into_any()
                                }}
                                </div>
                            </div>

                            // Route stats
                            {d.distance_mi.map(|dist| {
                                view! {
                                    <div class="detail-stat detail-distance">
                                        <strong>"Distance: "</strong>
                                        <span>{format!("{:.1} mi", dist)}</span>
                                    </div>
                                }
                            })}

                            {d.route_type.as_ref().map(|rtype| {
                                let rtype_label = match rtype {
                                    RouteType::Loop => "Loop",
                                    RouteType::Corridor => "Corridor",
                                };
                                view! {
                                    <div class="detail-stat detail-route-type">
                                        <strong>"Route type: "</strong>
                                        <span>{rtype_label}</span>
                                    </div>
                                }
                            })}

                            {d.start_label.as_ref().map(|start| {
                                view! {
                                    <div class="detail-stat detail-start">
                                        <strong>"Starts at: "</strong>
                                        <span>{start.clone()}</span>
                                    </div>
                                }
                            })}

                            {(d.via.is_some() || d.waypoints.as_ref().map(|w| !w.is_empty()).unwrap_or(false)).then(|| {
                                view! {
                                    <div class="detail-itinerary">
                                        <h3 class="detail-section-title">"Route Itinerary"</h3>
                                        {d.via.as_ref().map(|via| {
                                            view! {
                                                <div class="detail-stat detail-via">
                                                    <strong>"Via: "</strong>
                                                    <span>{via.clone()}</span>
                                                </div>
                                            }
                                        })}
                                        {d.waypoints.as_ref().map(|wps| {
                                            view! {
                                                <div class="detail-stat detail-waypoints">
                                                    <strong>"Waypoints: "</strong>
                                                    <span>{wps.join(" → ")}</span>
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            })}

                            // Google Maps Navigation Links (ahead of exports & secondary actions)
                            {if !d.navigation_links.is_empty() || (d.key.kind == ObjectKind::Place && d.coordinates.is_some()) {
                                let links = d.navigation_links.clone();
                                let maps_kind = match d.key.kind {
                                    ObjectKind::Route => "route",
                                    ObjectKind::Road => "road",
                                    ObjectKind::Place => "place",
                                };
                                let maps_id = d.key.id.clone();
                                let start_label_opt = d.start_label.clone();
                                view! {
                                    <div class=if d.key.kind == ObjectKind::Place { "detail-nav-links place-actions" } else { "detail-nav-links" }>
                                        <span class="nav-links-label"><strong>"Navigation:"</strong></span>
                                        {if d.key.kind == ObjectKind::Place {
                                            d.coordinates.map(|point| view! {
                                                <a
                                                    href=format!("https://www.google.com/maps/search/?api=1&query={},{}", point[1], point[0])
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                    class="detail-link detail-place-maps-link"
                                                    aria-label="Open in Google Maps"
                                                >
                                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M14 3h7v7M21 3l-9 9M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5"/></svg>
                                                    "Google Maps"
                                                </a>
                                            }).into_any()
                                        } else {
                                            ().into_any()
                                        }}
                                        {links.into_iter().map(|lnk| {
                                            let maps_kind_item = maps_kind;
                                            let maps_id_item = maps_id.clone();
                                            let is_start = lnk.mode == NavigationMode::Start;
                                            let start_label_for_link = start_label_opt.clone();
                                            let link_label = if is_start {
                                                "Directions to start".to_string()
                                            } else {
                                                lnk.label.clone()
                                            };
                                            let is_place = d.key.kind == ObjectKind::Place;
                                            let accessible_label = link_label.clone();
                                            let link_label = if is_place { "Get directions".to_string() } else { link_label };
                                            view! {
                                                <div class=if is_start { "nav-start-block" } else { "nav-link-block" }>
                                                    <div class="nav-link-action-row">
                                                        <a
                                                            href=lnk.url
                                                            target="_blank"
                                                            rel="noopener"
                                                            class="detail-link detail-maps-link"
                                                            aria-label=accessible_label
                                                            on:click=move |_| {
                                                                #[cfg(target_arch = "wasm32")]
                                                                crate::analytics::track_maps_click(maps_kind_item, &maps_id_item);
                                                            }
                                                        >
                                                            {is_place.then(|| view! {
                                                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="m12 3 9 18-9-4-9 4 9-18Z"/></svg>
                                                            })}
                                                            {link_label}
                                                        </a>
                                                        {if is_start {
                                                            if let Some(ref start_txt) = start_label_for_link {
                                                                view! {
                                                                    <span class="nav-start-place">{format!("Starts at: {}", start_txt)}</span>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <span class="nav-start-note">"Opens directions to the route's starting coordinate."</span>
                                                                }.into_any()
                                                            }
                                                        } else {
                                                            ().into_any()
                                                        }}
                                                    </div>
                                                    {if is_start {
                                                        view! {
                                                            <p class="nav-start-explainer">"This gets you to the start; it does not follow the full ride."</p>
                                                        }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                </div>
                                            }
                                        }).collect_view()}
                                        {(d.key.kind == ObjectKind::Route).then(|| view! {
                                            <p class="nav-route-note">"Google Maps calculates the route, so it may differ from the ride shown here."</p>
                                        })}
                                    </div>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            // Action buttons / links (Exports, Copy Link, Full Page, Offline)
                            <div class="detail-action-links">
                                {(d.gpx_url.is_some() || d.geometry_url.is_some()).then(|| {
                                    let gpx_opt = d.gpx_url.clone();
                                    let geom_opt = d.geometry_url.clone();
                                    let route_id = d.key.id.clone();
                                    view! {
                                        <details class="export-route-disclosure">
                                            <summary class="export-route-summary">"Export route"</summary>
                                            <div class="export-links">
                                                {gpx_opt.map(|gpx| {
                                                    let rid = route_id.clone();
                                                    view! {
                                                        <a
                                                            href=gpx
                                                            download
                                                            class="detail-link detail-gpx-download"
                                                            on:click=move |_| {
                                                                #[cfg(target_arch = "wasm32")]
                                                                crate::analytics::track_gpx_download(&rid);
                                                            }
                                                        >
                                                            "Download GPX"
                                                        </a>
                                                    }
                                                })}
                                                {geom_opt.map(|geom| {
                                                    view! {
                                                        <a
                                                            href=geom
                                                            download
                                                            class="detail-link detail-geojson-download"
                                                        >
                                                            "Download GeoJSON"
                                                        </a>
                                                    }
                                                })}
                                            </div>
                                        </details>
                                    }
                                })}

                                {(d.key.kind == ObjectKind::Road).then(|| view! {
                                    <a
                                        href=page_url
                                        target="_blank"
                                        rel="noopener"
                                        class="detail-link detail-page-link detail-link-secondary"
                                    >
                                        "View Full Page ↗"
                                    </a>
                                })}


                            </div>

                            // Body HTML
                            {if !d.body_html.trim().is_empty() {
                                let body = d.body_html.clone();
                                view! {
                                    <div class="detail-body" inner_html=body></div>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            // Author Note
                            {d.author_note.as_ref().map(|note| {
                                let status_str = format!("{:?}", note.visit_status);
                                let rec_str = note.recommendation.map(|r| format!("{:?}", r));
                                let vdate = note.visited_on.clone();
                                let note_text = note.text.clone();

                                view! {
                                    <section class="detail-notes">
                                        <h3 class="section-title">"Ride notes"</h3>
                                        <div class="note-meta">
                                            <p><strong>"Visit status: "</strong> {status_str}</p>
                                            {rec_str.map(|r| view! {
                                                <p><strong>"Recommendation: "</strong> {r}</p>
                                            })}
                                            {vdate.map(|vd| view! {
                                                <p><strong>"Visited on: "</strong> {vd}</p>
                                            })}
                                        </div>
                                        <p class="author-note-text">{note_text}</p>
                                    </section>
                                }
                            })}

                            // Photos
                            {if !d.photos.is_empty() {
                                let photos = d.photos.clone();
                                view! {
                                    <section class="detail-photos">
                                        <h3 class="section-title">"Photos"</h3>
                                        <div class="detail-photo-gallery">
                                            {photos.into_iter().map(|p| {
                                                let img_url = p.variants.last().map(|v| v.url.clone())
                                                    .or_else(|| p.variants.first().map(|v| v.url.clone()))
                                                    .unwrap_or_default();
                                                let alt_text = p.alt.clone();
                                                let credit = p.credit.clone();
                                                let rights = p.rights.clone();

                                                view! {
                                                    <figure class="detail-photo-card">
                                                        <img
                                                            src=img_url
                                                            alt=alt_text
                                                            loading="lazy"
                                                            class="detail-photo-img"
                                                        />
                                                        {if !credit.is_empty() || !rights.is_empty() {
                                                            let caption = format!("{} ({})", credit, rights);
                                                            view! {
                                                                <figcaption class="photo-caption">{caption}</figcaption>
                                                            }.into_any()
                                                        } else {
                                                            view! {}.into_any()
                                                        }}
                                                    </figure>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </section>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            // Route stops
                            {if !d.stops.is_empty() {
                                let stops = d.stops.clone();
                                view! {
                                    <section class="detail-stops">
                                        <h3 class="section-title">"Route stops"</h3>
                                        <ul class="stops-list">
                                            {stops.into_iter().map(|stop| {
                                                let stop_key = stop.key.clone();
                                                let stop_title = stop.title.clone();
                                                let stop_note = stop.note.clone();

                                                view! {
                                                    <li class="stop-item">
                                                        <button
                                                            type="button"
                                                            class="stop-select-btn"
                                                            on:click=move |_| on_select_place.run(stop_key.clone())
                                                        >
                                                            <strong class="stop-title">{stop_title}</strong>
                                                            {stop_note.map(|n| view! {
                                                                <span class="stop-note">{n}</span>
                                                            })}
                                                        </button>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    </section>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            // Nearby places
                            {if !d.nearby_places.is_empty() {
                                let nearby = d.nearby_places.clone();
                                view! {
                                    <section class="detail-nearby">
                                        <h3 class="section-title">"Near this route"</h3>
                                        <ul class="nearby-list">
                                            {nearby.into_iter().map(|place| {
                                                let place_key = place.key.clone();
                                                let place_title = place.title.clone();
                                                let dist_mi = place.distance_m / 1609.344;

                                                view! {
                                                    <li class="nearby-item">
                                                        <button
                                                            type="button"
                                                            class="nearby-select-btn"
                                                            on:click=move |_| on_select_place.run(place_key.clone())
                                                        >
                                                            <strong class="nearby-title">{place_title}</strong>
                                                            <span class="nearby-dist">{format!(" ({:.1} mi)", dist_mi)}</span>
                                                        </button>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    </section>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            // Related objects
                            {if !d.related.is_empty() {
                                let related = d.related.clone();
                                view! {
                                    <section class="detail-related">
                                        <h3 class="section-title">"Related"</h3>
                                        <ul class="related-list">
                                            {related.into_iter().map(|rel| {
                                                let rel_key = rel.key.clone();
                                                let rel_title = rel.title.clone();

                                                view! {
                                                    <li class="related-item">
                                                        <button
                                                            type="button"
                                                            class="related-select-btn"
                                                            on:click=move |_| on_select_place.run(rel_key.clone())
                                                        >
                                                            {rel_title}
                                                        </button>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    </section>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </article>
    }
}
