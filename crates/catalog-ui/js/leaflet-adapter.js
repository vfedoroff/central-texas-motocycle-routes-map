/**
 * Ride Atlas Leaflet Map Adapter
 * Implements the map contract between WebAssembly/Rust and Leaflet.
 *
 * Contract:
 * - createMap(elementId, config) -> Adapter
 * - setOverview(key, feature)
 * - removeOverview(key)
 * - setPlaces(points)
 * - setSelection(key, feature)
 * - clearSelection()
 * - fitBounds(bounds)
 * - resize()
 * - destroy()
 */

(function (root, factory) {
  if (typeof define === 'function' && define.amd) {
    define([], factory);
  } else if (typeof module === 'object' && module.exports) {
    module.exports = factory();
  } else {
    root.RideAtlasMap = factory();
    root.createMap = root.RideAtlasMap.createMap;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
  'use strict';

  const DEFAULT_OVERVIEW_BOUNDS = [
    [-100.2055, 29.6695],
    [-96.5996, 30.9865]
  ];

  function keyToString(key) {
    if (!key) return '';
    return key.kind + ':' + key.id;
  }

  function geoJsonBoundsToLeaflet(bounds) {
    // GeoJSON ordering: [[min_lon, min_lat], [max_lon, max_lat]]
    // Leaflet ordering: [[min_lat, min_lon], [max_lat, max_lon]]
    if (!bounds || bounds.length < 2) return null;
    return [
      [bounds[0][1], bounds[0][0]],
      [bounds[1][1], bounds[1][0]]
    ];
  }

  function leafletBoundsToGeoJson(leafletBounds) {
    return [
      [leafletBounds.getWest(), leafletBounds.getSouth()],
      [leafletBounds.getEast(), leafletBounds.getNorth()]
    ];
  }

  function createMap(elementId, config) {
    config = config || {};

    const container = typeof elementId === 'string'
      ? document.getElementById(elementId)
      : elementId;

    if (!container) {
      throw new Error('Container element not found: ' + elementId);
    }

    // Defensive teardown of any previous map on this container
    if (container._rideAtlasAdapter) {
      container._rideAtlasAdapter.destroy();
    } else if (container._leaflet_id && typeof L !== 'undefined') {
      container._leaflet_id = null;
    }

    if (typeof L === 'undefined') {
      throw new Error('Leaflet library (L) is not loaded');
    }

    let destroyed = false;

    // Canvas renderer for performance with many route lines
    // Keep thin route strokes legible while allowing imprecise mouse/touch selection.
    const canvasRenderer = L.canvas({ padding: 0.5, tolerance: 12 });

    // Initial view coordinates: Central Texas default
    const defaultCenter = [30.45, -98.35];
    const defaultZoom = 8;

    const map = L.map(container, {
      center: defaultCenter,
      zoom: defaultZoom,
      zoomControl: true,
      preferCanvas: true
    });

    // Basemap tile layers matching legacy app
    const baseLayers = {
      'Topographic': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/World_Topo_Map/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri, USGS | &copy; OpenStreetMap contributors',
          maxZoom: 19
        }
      ),
      'Streets': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/World_Street_Map/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri | &copy; OpenStreetMap contributors',
          maxZoom: 19
        }
      ),
      'Dark': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/Canvas/World_Dark_Gray_Base/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri, HERE, Garmin | &copy; OpenStreetMap contributors',
          maxZoom: 16
        }
      ),
      'Satellite': L.tileLayer(
        'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
        {
          attribution: 'Esri, Maxar, Earthstar Geographics',
          maxZoom: 18
        }
      ),
      'OpenStreetMap': L.tileLayer(
        'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
        {
          attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
          maxZoom: 19
        }
      )
    };

    baseLayers['Topographic'].addTo(map);
    L.control.layers(baseLayers, null, { position: 'topright' }).addTo(map);

    const initialOverviewBounds = config.initialBounds || DEFAULT_OVERVIEW_BOUNDS;
    let lastOverviewBounds = initialOverviewBounds;
    let userPannedOverview = false;
    let isProgrammaticMove = false;

    function fitOverview() {
      if (destroyed || !lastOverviewBounds || userPannedOverview) return;
      const llBounds = geoJsonBoundsToLeaflet(lastOverviewBounds);
      if (llBounds) {
        const padding = computeFitPadding();
        isProgrammaticMove = true;
        try {
          map.fitBounds(llBounds, {
            paddingTopLeft: padding.paddingTopLeft,
            paddingBottomRight: padding.paddingBottomRight,
            animate: false
          });
        } finally {
          queueMicrotask(() => { isProgrammaticMove = false; });
        }
      }
    }

    // Initial bounds fitting
    fitOverview();
    let overviewFitTimer = setTimeout(() => {
      overviewFitTimer = null;
      if (!destroyed && !userPannedOverview && selectionLayer === null) {
        fitOverview();
      }
    }, 80);

    // Layer groups
    const overviewGroup = L.featureGroup().addTo(map);
    const overviewMap = new Map();

    const userLocationGroup = L.featureGroup().addTo(map);

    let selectionLayer = null;
    let startMarker = null;
    let selectedKey = null;
    let lastSelectionBounds = null;
    let userPannedSelection = false;

    // Marker cluster group for places
    let placesGroup = null;
    if (typeof L.markerClusterGroup === 'function') {
      placesGroup = L.markerClusterGroup({
        maxClusterRadius: 40,
        showCoverageOnHover: false,
        spiderfyOnMaxZoom: true,
        chunkedLoading: true
      });
    } else {
      placesGroup = L.featureGroup();
    }
    placesGroup.addTo(map);

    // Map locate control button
    const LocateControl = L.Control.extend({
      options: { position: 'topleft' },
      onAdd: function () {
        const btn = L.DomUtil.create('button', 'leaflet-bar leaflet-control leaflet-control-locate');
        btn.setAttribute('type', 'button');
        btn.setAttribute('title', 'Locate me on map');
        btn.setAttribute('aria-label', 'Locate me on map');
        btn.innerHTML = '<span class="locate-icon" aria-hidden="true">📍</span>';
        L.DomEvent.disableClickPropagation(btn);
        L.DomEvent.on(btn, 'click', function (e) {
          L.DomEvent.preventDefault(e);
          adapter.locateUser();
        });
        return btn;
      }
    });
    new LocateControl().addTo(map);

    // Moveend listener for boundsChanged
    function publishBounds(initial) {
      if (destroyed) return;
      if (typeof config.onBoundsChanged === 'function') {
        const bounds = leafletBoundsToGeoJson(map.getBounds());
        config.onBoundsChanged(bounds, initial);
      }
    }

    function noteUserMapInteraction() {
      if (!isProgrammaticMove) {
        if (overviewFitTimer) {
          clearTimeout(overviewFitTimer);
          overviewFitTimer = null;
        }
        if (adapter.hasSelection()) {
          userPannedSelection = true;
        } else {
          userPannedOverview = true;
        }
      }
    }

    function handleMoveEnd() {
      const programmatic = isProgrammaticMove;
      isProgrammaticMove = false;
      if (!programmatic) {
        if (overviewFitTimer) {
          clearTimeout(overviewFitTimer);
          overviewFitTimer = null;
        }
        if (adapter.hasSelection()) {
          userPannedSelection = true;
        } else {
          userPannedOverview = true;
        }
      }
      publishBounds(programmatic);
    }
    function handleResize() { publishBounds(true); }
    map.on('moveend', handleMoveEnd);
    map.on('resize', handleResize);
    map.on('movestart', noteUserMapInteraction);
    map.on('dragstart', noteUserMapInteraction);
    map.on('zoomstart', noteUserMapInteraction);

    function computeFitPadding() {
      const padTop = 24;
      const padLeft = 24;
      const padRight = 24;
      let padBottom = 24;

      if (window.innerWidth < 768) {
        const bottomPanel = document.querySelector('.bottom-panel');
        let panelHeight = 0;
        if (bottomPanel) {
          if (bottomPanel.classList.contains('snap-expanded')) {
            panelHeight = window.innerHeight * 0.58;
          } else {
            const rect = bottomPanel.getBoundingClientRect();
            panelHeight = rect.height;
            if (panelHeight <= 0) {
              if (bottomPanel.classList.contains('snap-collapsed')) {
                panelHeight = window.innerHeight * 0.20;
              } else {
                panelHeight = window.innerHeight * 0.58;
              }
            }
          }
        } else {
          panelHeight = window.innerHeight * 0.58;
        }
        const maxAllowedPadding = Math.max(24, window.innerHeight - 100);
        padBottom = Math.min(Math.round(panelHeight) + 16, maxAllowedPadding);
      }

      return {
        paddingTopLeft: [padLeft, padTop],
        paddingBottomRight: [padRight, padBottom]
      };
    }

    // Adapter instance
    const adapter = {
      map: map,

      setOverview: function (key, feature) {
        if (destroyed || !key || !feature) return;
        const kStr = keyToString(key);

        if (overviewMap.has(kStr)) {
          overviewGroup.removeLayer(overviewMap.get(kStr));
          overviewMap.delete(kStr);
        }

        const color = (feature.properties && feature.properties.color)
          ? feature.properties.color
          : '#2563eb';

        const layer = L.geoJSON(feature, {
          renderer: canvasRenderer,
          style: {
            color: color,
            weight: 3.5,
            opacity: 0.85
          }
        });

        // Click selection on line
        layer.on('click', function (e) {
          if (destroyed) return;
          if (e && e.originalEvent) {
            L.DomEvent.stopPropagation(e.originalEvent);
          }
          if (typeof config.onSelect === 'function') {
            config.onSelect(key);
          }
        });

        // Popup with safe textContent
        if (feature.properties && feature.properties.title) {
          const titleDiv = document.createElement('div');
          titleDiv.className = 'map-popup-title';
          titleDiv.textContent = feature.properties.title;
          layer.bindPopup(titleDiv);
        }

        overviewGroup.addLayer(layer);
        overviewMap.set(kStr, layer);
      },

      removeOverview: function (key) {
        if (destroyed || !key) return;
        const kStr = keyToString(key);
        if (overviewMap.has(kStr)) {
          const layer = overviewMap.get(kStr);
          overviewGroup.removeLayer(layer);
          overviewMap.delete(kStr);
        }
      },

      setPlaces: function (points) {
        if (destroyed) return;
        placesGroup.clearLayers();
        if (!Array.isArray(points)) return;

        points.forEach(function (pt) {
          if (!pt || !pt.coordinates || pt.coordinates.length < 2) return;
          const lat = pt.coordinates[1];
          const lon = pt.coordinates[0];

          const marker = L.marker([lat, lon]);

          if (pt.title) {
            const popupDiv = document.createElement('div');
            popupDiv.className = 'place-popup';
            const titleEl = document.createElement('strong');
            titleEl.textContent = pt.title;
            popupDiv.appendChild(titleEl);
            marker.bindPopup(popupDiv);
          }

          marker.on('click', function (e) {
            if (destroyed) return;
            if (e && e.originalEvent) {
              L.DomEvent.stopPropagation(e.originalEvent);
            }
            if (typeof config.onSelect === 'function') {
              config.onSelect(pt.key);
            }
          });

          placesGroup.addLayer(marker);
        });
      },

      setSelection: function (key, feature) {
        if (destroyed) return;
        this.clearSelection();
        if (!key || !feature) return;

        userPannedSelection = false;
        selectedKey = key;

        if (feature.geometry && feature.geometry.type === 'Point') {
          const coords = feature.geometry.coordinates;
          const lat = coords[1];
          const lon = coords[0];

          selectionLayer = L.circleMarker([lat, lon], {
            radius: 12,
            color: '#f59e0b',
            fillColor: '#fbbf24',
            fillOpacity: 0.9,
            weight: 3
          }).addTo(map);
        } else {
          // LineString / MultiLineString
          const color = (feature.properties && feature.properties.color)
            ? feature.properties.color
            : '#2563eb';

          selectionLayer = L.geoJSON(feature, {
            renderer: canvasRenderer,
            style: {
              color: color,
              weight: 6,
              opacity: 1.0
            }
          }).addTo(map);

          let firstCoord = null;
          if (feature.geometry) {
            if (feature.geometry.type === 'LineString' && Array.isArray(feature.geometry.coordinates) && feature.geometry.coordinates.length > 0) {
              firstCoord = feature.geometry.coordinates[0];
            } else if (feature.geometry.type === 'MultiLineString' && Array.isArray(feature.geometry.coordinates) && feature.geometry.coordinates.length > 0 && Array.isArray(feature.geometry.coordinates[0]) && feature.geometry.coordinates[0].length > 0) {
              firstCoord = feature.geometry.coordinates[0][0];
            }
          }

          if (firstCoord && firstCoord.length >= 2) {
            const startLat = firstCoord[1];
            const startLon = firstCoord[0];
            const startIcon = L.divIcon({
              className: 'route-start-marker-container',
              html: '<span class="route-start-marker" aria-label="Route start">Start</span>',
              iconSize: [44, 22],
              iconAnchor: [22, 11]
            });
            startMarker = L.marker([startLat, startLon], {
              icon: startIcon,
              title: 'Start: ' + (feature.properties && feature.properties.title ? feature.properties.title : 'Route start'),
              keyboard: false
            }).addTo(map);
          }
        }

        if (feature.properties && feature.properties.title) {
          const popupDiv = document.createElement('div');
          popupDiv.className = 'selection-popup';
          popupDiv.textContent = feature.properties.title;
          selectionLayer.bindPopup(popupDiv);
        }
      },

      hasSelection: function () {
        return selectionLayer !== null;
      },

      getSelectedKey: function () {
        return selectedKey;
      },

      getSelectionPopupContent: function () {
        if (!selectionLayer) return null;
        const popup = selectionLayer.getPopup ? selectionLayer.getPopup() : null;
        if (!popup) return null;
        const content = popup.getContent();
        if (typeof content === 'string') return content;
        if (content && content.textContent) return content.textContent;
        return null;
      },

      clearSelection: function () {
        if (selectionLayer) {
          map.removeLayer(selectionLayer);
          selectionLayer = null;
        }
        if (startMarker) {
          map.removeLayer(startMarker);
          startMarker = null;
        }
        selectedKey = null;
        lastSelectionBounds = null;
        userPannedSelection = false;
      },

      fitBounds: function (bounds) {
        if (destroyed || !bounds) return;
        if (selectedKey !== null) {
          lastSelectionBounds = bounds;
        }
        const llBounds = geoJsonBoundsToLeaflet(bounds);
        if (llBounds) {
          const padding = computeFitPadding();
          isProgrammaticMove = true;
          try {
            map.fitBounds(llBounds, {
              paddingTopLeft: padding.paddingTopLeft,
              paddingBottomRight: padding.paddingBottomRight,
              animate: false
            });
          } finally {
            queueMicrotask(() => { isProgrammaticMove = false; });
          }
        }
      },

      reframeSelection: function () {
        if (destroyed || !this.hasSelection() || !lastSelectionBounds) return;
        userPannedSelection = false;
        this.fitBounds(lastSelectionBounds);
      },

      reframeOverview: function () {
        if (destroyed || this.hasSelection() || !lastOverviewBounds) return;
        const bottomPanel = document.querySelector('.bottom-panel');
        if (bottomPanel && bottomPanel.classList.contains('snap-expanded')) return;
        userPannedOverview = false;
        fitOverview();
      },

      locateUser: function (onSuccess, onError) {
        if (destroyed) return;
        if (typeof navigator === 'undefined' || !navigator.geolocation) {
          const msg = 'Geolocation is not supported by your browser';
          if (typeof onError === 'function') onError(msg);
          if (typeof config.onLocationError === 'function') config.onLocationError(msg);
          return;
        }

        navigator.geolocation.getCurrentPosition(
          function (pos) {
            if (destroyed) return;
            const lat = pos.coords.latitude;
            const lon = pos.coords.longitude;
            const accuracy = pos.coords.accuracy || 0;
            adapter.setUserLocation(lat, lon, accuracy);
            if (typeof onSuccess === 'function') onSuccess(lat, lon, accuracy);
            if (typeof config.onUserLocation === 'function') config.onUserLocation(lat, lon, accuracy);
          },
          function (err) {
            if (destroyed) return;
            let msg = 'Unable to retrieve your location';
            if (err.code === 1) msg = 'Location access was denied. Please allow location permissions to find routes near you.';
            else if (err.code === 2) msg = 'Location information is currently unavailable.';
            else if (err.code === 3) msg = 'Location request timed out. Please try again.';
            if (typeof onError === 'function') onError(msg);
            if (typeof config.onLocationError === 'function') config.onLocationError(msg);
          },
          { enableHighAccuracy: true, timeout: 10000, maximumAge: 60000 }
        );
      },

      setUserLocation: function (lat, lon, accuracy) {
        if (destroyed) return;
        userLocationGroup.clearLayers();

        if (typeof accuracy === 'number' && accuracy > 0) {
          L.circle([lat, lon], {
            radius: Math.min(accuracy, 8000),
            color: '#2563eb',
            weight: 1,
            fillColor: '#3b82f6',
            fillOpacity: 0.15,
            interactive: false
          }).addTo(userLocationGroup);
        }

        const pulseHtml = '<div class="user-pulse-marker"><div class="user-pulse-ring"></div><div class="user-pulse-dot"></div></div>';
        const userIcon = L.divIcon({
          className: 'user-location-icon-container',
          html: pulseHtml,
          iconSize: [24, 24],
          iconAnchor: [12, 12]
        });

        const marker = L.marker([lat, lon], {
          icon: userIcon,
          zIndexOffset: 1000
        }).addTo(userLocationGroup);

        marker.bindPopup('<div class="user-location-popup"><strong>Your Location</strong></div>');

        const currentZoom = map.getZoom();
        map.setView([lat, lon], currentZoom < 10 ? 11 : currentZoom);
      },

      clearUserLocation: function () {
        if (destroyed) return;
        userLocationGroup.clearLayers();
      },

      resize: function () {
        if (destroyed) return;
        map.invalidateSize();
      },

      destroy: function () {
        if (destroyed) return;
        destroyed = true;

        if (overviewFitTimer) {
          clearTimeout(overviewFitTimer);
          overviewFitTimer = null;
        }

        map.off('moveend', handleMoveEnd);
        map.off('resize', handleResize);
        map.off('movestart', noteUserMapInteraction);
        map.off('dragstart', noteUserMapInteraction);
        map.off('zoomstart', noteUserMapInteraction);

        this.clearSelection();

        overviewMap.forEach(function (layer) {
          overviewGroup.removeLayer(layer);
        });
        overviewMap.clear();

        userLocationGroup.clearLayers();
        map.removeLayer(userLocationGroup);

        placesGroup.clearLayers();
        map.removeLayer(placesGroup);
        map.removeLayer(overviewGroup);

        document.removeEventListener('transitionend', handleTransitionEnd);

        map.remove();

        container._rideAtlasAdapter = null;
        container._leaflet_id = null;
      }
    };

    function handleTransitionEnd(e) {
      if (destroyed) return;
      if (e.target && e.target.classList && e.target.classList.contains('bottom-panel')) {
        if (e.propertyName === 'height') {
          if (e.target.classList.contains('snap-expanded')) {
            return;
          }
          if (adapter.hasSelection() && lastSelectionBounds && !userPannedSelection) {
            adapter.fitBounds(lastSelectionBounds);
          } else if (!adapter.hasSelection() && lastOverviewBounds && !userPannedOverview) {
            fitOverview();
          }
        }
      }
    }
    document.addEventListener('transitionend', handleTransitionEnd);

    container._rideAtlasAdapter = adapter;
    queueMicrotask(() => publishBounds(true));
    return adapter;
  }

  return {
    createMap: createMap
  };
});
