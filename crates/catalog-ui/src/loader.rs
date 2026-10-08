use catalog_model::{Bounds, ObjectKey};
use std::collections::{HashMap, HashSet, VecDeque};

pub const MAX_CONCURRENT_REQUESTS: usize = 4;
pub const MAX_OVERVIEW_CACHE: usize = 200;
pub const MAX_ATTACHED_LAYERS: usize = 200;

#[derive(Clone, Debug, PartialEq)]
pub struct OverviewCandidate {
    pub key: ObjectKey,
    pub overview_url: String,
    pub bounds: Bounds,
    pub color: Option<String>,
    pub center_distance_sq: f64,
}

/// Compute distance squared between bounds center and viewport center.
pub fn distance_to_viewport_center_sq(obj_bounds: &Bounds, vp_bounds: &Bounds) -> f64 {
    let obj_cx = (obj_bounds[0] + obj_bounds[2]) / 2.0;
    let obj_cy = (obj_bounds[1] + obj_bounds[3]) / 2.0;

    let vp_cx = (vp_bounds[0] + vp_bounds[2]) / 2.0;
    let vp_cy = (vp_bounds[1] + vp_bounds[3]) / 2.0;

    let dx = obj_cx - vp_cx;
    let dy = obj_cy - vp_cy;
    dx * dx + dy * dy
}

/// Sort and cap overview candidates prioritizing:
/// 1) selected key,
/// 2) distance from viewport center,
/// 3) stable key.
pub fn sort_and_cap_candidates(
    mut candidates: Vec<OverviewCandidate>,
    selected_key: Option<&ObjectKey>,
    limit: usize,
) -> (Vec<OverviewCandidate>, bool) {
    candidates.sort_by(|a, b| {
        let a_sel = selected_key == Some(&a.key);
        let b_sel = selected_key == Some(&b.key);
        if a_sel != b_sel {
            return b_sel.cmp(&a_sel); // true comes before false
        }
        match a.center_distance_sq.partial_cmp(&b.center_distance_sq) {
            Some(ord) if ord != std::cmp::Ordering::Equal => ord,
            _ => (&a.key.kind, &a.key.id).cmp(&(&b.key.kind, &b.key.id)),
        }
    });

    let exceeded = candidates.len() > limit;
    candidates.truncate(limit);
    (candidates, exceeded)
}

/// LRU Cache for parsed overview objects.
#[derive(Clone, Debug)]
pub struct LruCache<K: std::hash::Hash + Eq + Clone, V: Clone> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
                self.order.push_back(key.clone());
            }
            self.map.get(key)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, val: V) -> Option<K> {
        let mut evicted_key = None;
        if self.map.contains_key(&key) {
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
        } else if self.map.len() >= self.capacity
            && let Some(oldest) = self.order.pop_front()
        {
            self.map.remove(&oldest);
            evicted_key = Some(oldest);
        }
        self.order.push_back(key.clone());
        self.map.insert(key, val);
        evicted_key
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.map.remove(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Overview geometry loader state coordinating concurrency, caching, and generation staleness.
#[derive(Clone, Debug)]
pub struct LoaderState {
    pub generation: u64,
    target_filter_generation: u64,
    target_order: Vec<ObjectKey>,
    pub cache: LruCache<ObjectKey, serde_json::Value>,
    pub attached_layers: HashSet<ObjectKey>,
    pub target_keys: HashSet<ObjectKey>,
    pub pending_queue: VecDeque<OverviewCandidate>,
    pub active_requests: usize,
    pub has_error: bool,
}

impl Default for LoaderState {
    fn default() -> Self {
        Self {
            generation: 0,
            target_filter_generation: 0,
            target_order: Vec::new(),
            cache: LruCache::new(MAX_OVERVIEW_CACHE),
            attached_layers: HashSet::new(),
            target_keys: HashSet::new(),
            pending_queue: VecDeque::new(),
            active_requests: 0,
            has_error: false,
        }
    }
}

impl LoaderState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update targets for a new generation.
    /// Returns keys to remove from map and keys to immediately add from cache.
    pub fn set_target_candidates(
        &mut self,
        generation: u64,
        candidates: Vec<OverviewCandidate>,
    ) -> (Vec<ObjectKey>, Vec<(ObjectKey, serde_json::Value)>) {
        let target_order: Vec<_> = candidates.iter().map(|c| c.key.clone()).collect();
        if self.target_filter_generation == generation && self.target_order == target_order {
            return (Vec::new(), Vec::new());
        }
        self.target_filter_generation = generation;
        self.target_order = target_order;
        self.generation = self.generation.wrapping_add(1);
        self.target_keys = candidates.iter().map(|c| c.key.clone()).collect();
        self.pending_queue.clear();
        self.has_error = false;

        // Keys currently attached but not in target_keys should be removed
        let to_remove: Vec<ObjectKey> = self
            .attached_layers
            .iter()
            .filter(|k| !self.target_keys.contains(k))
            .cloned()
            .collect();

        for k in &to_remove {
            self.attached_layers.remove(k);
        }

        let mut to_add = Vec::new();

        // For each candidate: if cached, add to map if not attached; if not cached, enqueue
        for cand in candidates {
            if let Some(cached_val) = self.cache.get(&cand.key).cloned() {
                if !self.attached_layers.contains(&cand.key) {
                    self.attached_layers.insert(cand.key.clone());
                    to_add.push((cand.key, cached_val));
                }
            } else {
                self.pending_queue.push_back(cand);
            }
        }

        (to_remove, to_add)
    }

    /// Try popping the next pending candidate if under concurrency cap
    pub fn next_request(&mut self) -> Option<OverviewCandidate> {
        if self.active_requests < MAX_CONCURRENT_REQUESTS
            && let Some(cand) = self.pending_queue.pop_front()
        {
            self.active_requests += 1;
            return Some(cand);
        }
        None
    }

    /// Record request completion.
    /// Returns (action to take: Option<set on map>, evicted key if any: Option<remove from map>)
    pub fn complete_request(
        &mut self,
        req_generation: u64,
        key: ObjectKey,
        val: Option<serde_json::Value>,
    ) -> (Option<(ObjectKey, serde_json::Value)>, Option<ObjectKey>) {
        if self.active_requests > 0 {
            self.active_requests -= 1;
        }

        // Stale generation check
        if req_generation != self.generation {
            return (None, None);
        }

        match val {
            Some(feature) => {
                let evicted_key = self.cache.insert(key.clone(), feature.clone());
                let mut layer_to_remove = None;
                if let Some(ref ev) = evicted_key
                    && self.attached_layers.remove(ev)
                {
                    layer_to_remove = Some(ev.clone());
                }

                let layer_to_add = if self.target_keys.contains(&key) {
                    self.attached_layers.insert(key.clone());
                    Some((key, feature))
                } else {
                    None
                };

                (layer_to_add, layer_to_remove)
            }
            None => {
                self.has_error = true;
                (None, None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog_model::ObjectKind;

    fn candidates(count: usize) -> Vec<OverviewCandidate> {
        (0..count)
            .map(|i| OverviewCandidate {
                key: ObjectKey {
                    kind: ObjectKind::Route,
                    id: format!("r-{i}"),
                },
                overview_url: format!("/overview/{i}"),
                bounds: [0.0, 0.0, 1.0, 1.0],
                color: None,
                center_distance_sq: i as f64,
            })
            .collect()
    }

    #[test]
    fn viewport_refresh_ignores_old_results_and_preserves_global_slots() {
        let mut loader = LoaderState::new();
        loader.set_target_candidates(0, candidates(8));
        let old_generation = loader.generation;
        let requests: Vec<_> = (0..4).map(|_| loader.next_request().unwrap()).collect();
        let mut next = candidates(8);
        next.reverse();
        loader.set_target_candidates(0, next.clone());
        assert_ne!(loader.generation, old_generation);
        assert!(loader.next_request().is_none());
        let generation = loader.generation;
        loader.set_target_candidates(0, next);
        assert_eq!(
            loader.generation, generation,
            "identical state must not restart requests"
        );
        assert_eq!(loader.pending_queue.len(), 8);
        assert_eq!(loader.active_requests, 4);
        loader.complete_request(
            old_generation,
            requests[0].key.clone(),
            Some(serde_json::json!({})),
        );
        assert!(loader.cache.is_empty());
        let current = loader.next_request().unwrap();
        assert_eq!(loader.active_requests, 4);
        assert!(
            loader
                .complete_request(generation, current.key, Some(serde_json::json!({})))
                .0
                .is_some()
        );
    }

    #[test]
    fn cache_eviction_removes_attached_layer_and_targets_remain_capped() {
        let mut loader = LoaderState::new();
        let (targets, exceeded) =
            sort_and_cap_candidates(candidates(205), None, MAX_ATTACHED_LAYERS);
        assert!(exceeded);
        loader.set_target_candidates(1, targets);
        while let Some(request) = loader.next_request() {
            loader.complete_request(loader.generation, request.key, Some(serde_json::json!({})));
        }
        assert_eq!(loader.cache.len(), 200);
        assert_eq!(loader.attached_layers.len(), 200);
        let oldest = candidates(1)[0].key.clone();
        let extra = ObjectKey {
            kind: ObjectKind::Route,
            id: "extra".into(),
        };
        let (_, removed) =
            loader.complete_request(loader.generation, extra, Some(serde_json::json!({})));
        assert_eq!(removed, Some(oldest.clone()));
        assert!(!loader.attached_layers.contains(&oldest));
        assert_eq!(loader.cache.len(), 200);
    }

    #[test]
    fn test_lru_cache_capacity_and_eviction() {
        let mut cache = LruCache::<String, i32>::new(3);
        assert_eq!(cache.insert("a".into(), 1), None);
        assert_eq!(cache.insert("b".into(), 2), None);
        assert_eq!(cache.insert("c".into(), 3), None);
        assert_eq!(cache.len(), 3);

        // Access "a" so "b" becomes the oldest
        assert_eq!(cache.get(&"a".into()), Some(&1));

        // Insert "d", should evict "b"
        let evicted = cache.insert("d".into(), 4);
        assert_eq!(evicted, Some("b".into()));
        assert!(!cache.contains(&"b".into()));
        assert!(cache.contains(&"a".into()));
        assert!(cache.contains(&"c".into()));
        assert!(cache.contains(&"d".into()));
    }

    #[test]
    fn test_sort_and_cap_candidates_prioritizes_selected_and_center_distance() {
        let k1 = ObjectKey {
            kind: ObjectKind::Route,
            id: "route-1".into(),
        };
        let k2 = ObjectKey {
            kind: ObjectKind::Route,
            id: "route-2".into(),
        };
        let k3 = ObjectKey {
            kind: ObjectKind::Route,
            id: "route-3".into(),
        };

        let c1 = OverviewCandidate {
            key: k1.clone(),
            overview_url: "/url1".into(),
            bounds: [0.0, 0.0, 1.0, 1.0],
            color: None,
            center_distance_sq: 10.0,
        };
        let c2 = OverviewCandidate {
            key: k2.clone(),
            overview_url: "/url2".into(),
            bounds: [0.0, 0.0, 1.0, 1.0],
            color: None,
            center_distance_sq: 2.0,
        };
        let c3 = OverviewCandidate {
            key: k3.clone(),
            overview_url: "/url3".into(),
            bounds: [0.0, 0.0, 1.0, 1.0],
            color: None,
            center_distance_sq: 5.0,
        };

        // When k1 is selected, it must come first despite higher distance
        let (sorted, exceeded) = sort_and_cap_candidates(vec![c1, c2, c3], Some(&k1), 2);
        assert!(exceeded);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].key, k1); // Selected first
        assert_eq!(sorted[1].key, k2); // Next closest (distance 2.0)
    }

    #[test]
    fn test_loader_concurrency_cap_and_stale_generation_check() {
        let mut loader = LoaderState::new();

        let candidates: Vec<OverviewCandidate> = (0..10)
            .map(|i| OverviewCandidate {
                key: ObjectKey {
                    kind: ObjectKind::Route,
                    id: format!("r-{}", i),
                },
                overview_url: format!("/url-{}", i),
                bounds: [0.0, 0.0, 1.0, 1.0],
                color: None,
                center_distance_sq: i as f64,
            })
            .collect();

        loader.set_target_candidates(1, candidates);

        // Should be able to pop exactly 4 requests (concurrency cap = 4)
        let r1 = loader.next_request();
        let r2 = loader.next_request();
        let r3 = loader.next_request();
        let r4 = loader.next_request();
        let r5 = loader.next_request();

        assert!(r1.is_some());
        assert!(r2.is_some());
        assert!(r3.is_some());
        assert!(r4.is_some());
        assert!(r5.is_none()); // Blocked at 4

        // Complete r1 with generation 1
        let key1 = r1.unwrap().key;
        let dummy_feat = serde_json::json!({ "type": "Feature" });
        let (to_add, _) = loader.complete_request(1, key1.clone(), Some(dummy_feat.clone()));
        assert!(to_add.is_some());

        // Now next_request should yield 1 more
        let r6 = loader.next_request();
        assert!(r6.is_some());

        // Test stale generation: if request completes with generation 0, it must be ignored
        let key2 = r2.unwrap().key;
        let (stale_add, _) = loader.complete_request(0, key2.clone(), Some(dummy_feat));
        assert!(stale_add.is_none());
        assert!(!loader.attached_layers.contains(&key2));
    }
}
