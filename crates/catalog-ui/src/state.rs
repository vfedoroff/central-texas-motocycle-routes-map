use std::collections::HashSet;

use catalog_model::{Bounds, CatalogIndex, ObjectKey, ObjectKind};

use crate::search::{
    CatalogFilter, CatalogSort, MileageFilterError, filter_places_overlay, validate_mileage,
};

pub const PAGE_SIZE: usize = 30;

#[derive(Clone, Debug, PartialEq)]
pub struct PriorResultsContext {
    pub filter: CatalogFilter,
    pub page: usize,
    pub sort_by_near_me: bool,
    pub sort: CatalogSort,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogState {
    pub pending_bounds: Option<Bounds>,
    pub viewport_bounds: Option<Bounds>,
    pub filter: CatalogFilter,
    pub page: usize,
    pub selected_key: Option<ObjectKey>,
    pub selection_generation: u64,
    pub filter_generation: u64,
    pub prior_results_context: Option<PriorResultsContext>,
    pub return_route_key: Option<ObjectKey>,
    pub places_overlay_visible: bool,
    pub nearby_routes_visible: bool,
    pub saved_keys: HashSet<ObjectKey>,
    pub remembered_route_categories: Vec<String>,
    pub remembered_min_distance_mi: Option<f64>,
    pub remembered_max_distance_mi: Option<f64>,
    pub prior_viewport_bounds: Option<Bounds>,
    pub user_location: Option<[f64; 2]>,
    pub sort_by_near_me: bool,
    pub sort: CatalogSort,
    pub remembered_route_sort: CatalogSort,
}

impl Default for CatalogState {
    fn default() -> Self {
        Self {
            pending_bounds: None,
            viewport_bounds: None,
            filter: CatalogFilter::default(),
            page: 0,
            selected_key: None,
            selection_generation: 0,
            filter_generation: 0,
            prior_results_context: None,
            return_route_key: None,
            places_overlay_visible: true,
            nearby_routes_visible: false,
            saved_keys: HashSet::new(),
            remembered_route_categories: Vec::new(),
            remembered_min_distance_mi: None,
            remembered_max_distance_mi: None,
            prior_viewport_bounds: None,
            user_location: None,
            sort_by_near_me: false,
            sort: CatalogSort::Title,
            remembered_route_sort: CatalogSort::Title,
        }
    }
}

impl CatalogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_query(&mut self, query: String) {
        if self.filter.query != query {
            self.filter.query = query;
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn set_kind(&mut self, kind: ObjectKind) {
        if self.filter.kind != kind {
            if self.filter.kind == ObjectKind::Route {
                self.remembered_route_categories = self.filter.categories.clone();
                self.remembered_min_distance_mi = self.filter.min_distance_mi;
                self.remembered_max_distance_mi = self.filter.max_distance_mi;
                self.remembered_route_sort = self.sort;
            }

            self.filter.kind = kind;

            if kind == ObjectKind::Route {
                self.filter.categories = self.remembered_route_categories.clone();
                self.filter.min_distance_mi = self.remembered_min_distance_mi;
                self.filter.max_distance_mi = self.remembered_max_distance_mi;
                self.sort = self.remembered_route_sort;
            } else {
                self.filter.categories.clear();
                self.filter.min_distance_mi = None;
                self.filter.max_distance_mi = None;
                self.sort = CatalogSort::Title;
            }

            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn set_categories(&mut self, categories: Vec<String>) {
        if self.filter.categories != categories {
            self.filter.categories = categories.clone();
            if self.filter.kind == ObjectKind::Route {
                self.remembered_route_categories = categories;
            }
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn set_mileage(
        &mut self,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Result<(), MileageFilterError> {
        validate_mileage(min, max)?;
        if self.filter.min_distance_mi != min || self.filter.max_distance_mi != max {
            self.filter.min_distance_mi = min;
            self.filter.max_distance_mi = max;
            if self.filter.kind == ObjectKind::Route {
                self.remembered_min_distance_mi = min;
                self.remembered_max_distance_mi = max;
            }
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
        Ok(())
    }

    pub fn set_saved_only(&mut self, saved_only: bool) {
        if self.filter.saved_only != saved_only {
            self.filter.saved_only = saved_only;
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn set_saved_keys(&mut self, keys: HashSet<ObjectKey>) {
        self.saved_keys = keys;
    }

    pub fn toggle_saved_key(&mut self, key: ObjectKey) -> bool {
        if self.saved_keys.contains(&key) {
            self.saved_keys.remove(&key);
            false
        } else {
            self.saved_keys.insert(key);
            true
        }
    }

    pub fn set_pending_bounds(&mut self, bounds: Option<Bounds>) {
        self.pending_bounds = bounds;
    }

    pub fn apply_pending_bounds(&mut self) {
        if self.filter.applied_bounds != self.pending_bounds {
            self.filter.applied_bounds = self.pending_bounds;
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn clear_bounds(&mut self) {
        let changed = self.filter.applied_bounds.is_some() || self.pending_bounds.is_some();
        self.pending_bounds = None;
        self.filter.applied_bounds = None;
        if changed {
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn set_user_location(&mut self, loc: Option<[f64; 2]>) {
        if self.user_location != loc {
            self.user_location = loc;
            if loc.is_none() {
                self.sort_by_near_me = false;
            }
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn set_sort_by_near_me(&mut self, active: bool) {
        if self.sort_by_near_me != active {
            self.sort_by_near_me = active;
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn set_sort(&mut self, sort: CatalogSort) {
        if self.sort != sort || self.sort_by_near_me {
            self.sort = sort;
            if self.filter.kind == ObjectKind::Route {
                self.remembered_route_sort = sort;
            }
            self.sort_by_near_me = false;
            self.page = 0;
            self.filter_generation = self.filter_generation.wrapping_add(1);
        }
    }

    pub fn clear_filters(&mut self) {
        self.filter.query.clear();
        self.filter.categories.clear();
        self.filter.min_distance_mi = None;
        self.filter.max_distance_mi = None;
        self.filter.saved_only = false;
        self.sort_by_near_me = false;
        self.sort = CatalogSort::Title;
        self.remembered_route_sort = CatalogSort::Title;
        self.filter.applied_bounds = None;
        self.pending_bounds = None;
        self.page = 0;
        self.filter_generation = self.filter_generation.wrapping_add(1);
    }

    pub fn select_key(&mut self, key: Option<ObjectKey>) {
        self.selected_key = key;
        self.selection_generation = self.selection_generation.wrapping_add(1);
    }

    pub fn select_with_prior_context(&mut self, key: ObjectKey) {
        if self.prior_results_context.is_none() {
            self.prior_results_context = Some(PriorResultsContext {
                filter: self.filter.clone(),
                page: self.page,
                sort_by_near_me: self.sort_by_near_me,
                sort: self.sort,
            });
            self.prior_viewport_bounds = self
                .viewport_bounds
                .or(self.filter.applied_bounds)
                .or(self.pending_bounds);
        }
        self.selected_key = Some(key);
        self.selection_generation = self.selection_generation.wrapping_add(1);
    }

    pub fn select_place_from_route(&mut self, place_key: ObjectKey, from_route_key: ObjectKey) {
        if self.prior_results_context.is_none() {
            self.prior_results_context = Some(PriorResultsContext {
                filter: self.filter.clone(),
                page: self.page,
                sort_by_near_me: self.sort_by_near_me,
                sort: self.sort,
            });
            self.prior_viewport_bounds = self
                .viewport_bounds
                .or(self.filter.applied_bounds)
                .or(self.pending_bounds);
        }
        self.return_route_key = Some(from_route_key);
        self.selected_key = Some(place_key);
        self.selection_generation = self.selection_generation.wrapping_add(1);
    }

    pub fn restore_back_to_results(&mut self) {
        if let Some(ctx) = self.prior_results_context.take() {
            self.filter = ctx.filter;
            self.page = ctx.page;
            self.sort_by_near_me = ctx.sort_by_near_me;
            self.sort = ctx.sort;
        }
        self.selected_key = None;
        self.return_route_key = None;
        self.prior_viewport_bounds = None;
        self.selection_generation = self.selection_generation.wrapping_add(1);
    }

    pub fn restore_back_to_route(&mut self) {
        if let Some(route_key) = self.return_route_key.take() {
            self.selected_key = Some(route_key);
            self.selection_generation = self.selection_generation.wrapping_add(1);
        } else {
            self.restore_back_to_results();
        }
    }

    pub fn set_places_overlay_visible(&mut self, visible: bool) {
        self.places_overlay_visible = visible;
    }

    pub fn visible_place_keys(&self, index: &CatalogIndex) -> Vec<ObjectKey> {
        if self.places_overlay_visible || self.filter.kind == ObjectKind::Place {
            let mut keys = filter_places_overlay(
                index,
                &self.filter.query,
                self.filter.applied_bounds.as_ref(),
            );
            if let Some(ref sel) = self.selected_key
                && sel.kind == ObjectKind::Place
                && !keys.contains(sel)
            {
                keys.push(sel.clone());
            }
            keys
        } else if let Some(ref sel) = self.selected_key {
            if sel.kind == ObjectKind::Place {
                vec![sel.clone()]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn current_results(&self, index: &CatalogIndex) -> Vec<ObjectKey> {
        if self.sort_by_near_me {
            crate::search::filter_summaries_with_location(
                index,
                &self.filter,
                Some(&self.saved_keys),
                self.user_location.as_ref(),
            )
        } else {
            let effective_sort = if self.filter.kind == ObjectKind::Place {
                crate::search::CatalogSort::Title
            } else {
                self.sort
            };
            let keys = crate::search::filter_summaries_with_location(
                index,
                &self.filter,
                Some(&self.saved_keys),
                None,
            );
            crate::search::sort_result_keys(index, keys, effective_sort)
        }
    }

    pub fn page_slice<'a>(&self, items: &'a [ObjectKey]) -> &'a [ObjectKey] {
        let start = self.page * PAGE_SIZE;
        if start >= items.len() {
            &[]
        } else {
            let end = (start + PAGE_SIZE).min(items.len());
            &items[start..end]
        }
    }

    pub fn total_pages(&self, total_items: usize) -> usize {
        if total_items == 0 {
            1
        } else {
            total_items.div_ceil(PAGE_SIZE)
        }
    }

    pub fn set_page(&mut self, page: usize) {
        self.page = page;
    }
}
