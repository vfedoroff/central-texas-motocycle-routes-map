(function () {
  'use strict';

  var STORAGE_KEY = 'catalog_analytics_consent';
  var gtagInitialized = false;

  function getConfig() {
    if (window.__CATALOG_CONFIG__) {
      return window.__CATALOG_CONFIG__;
    }
    var elem = document.getElementById('catalog-config');
    if (elem && elem.textContent) {
      try {
        return JSON.parse(elem.textContent);
      } catch (e) {
        console.warn('Failed to parse catalog-config JSON', e);
      }
    }
    return {
      environment: 'development',
      analytics: {
        enabled: false,
        measurement_id: null,
        production_host: 'central-texas-routes-map.netlify.app',
      },
    };
  }

  function getConsent() {
    try {
      var val = localStorage.getItem(STORAGE_KEY);
      if (val === 'allowed') return 'allowed';
      if (val === 'declined') return 'declined';
    } catch (_) {}
    return 'unset';
  }

  function checkGates() {
    var config = getConfig();
    var env = config.environment || 'development';
    var analytics = config.analytics || {};
    var enabled = !!analytics.enabled;
    var measurementId = analytics.measurement_id;
    var productionHost = analytics.production_host;
    var consent = getConsent();
    var currentHost = window.location.hostname;

    // Gate 1: Build environment == "production"
    if (env !== 'production') {
      return { allowed: false, reason: 'env_not_production' };
    }
    // Gate 2: Runtime window.location.hostname == production_host
    if (!productionHost || currentHost !== productionHost) {
      return { allowed: false, reason: 'hostname_mismatch' };
    }
    // Gate 3: analytics.enabled == true && measurement_id is present
    if (!enabled || !measurementId) {
      return { allowed: false, reason: 'analytics_not_enabled' };
    }
    // Gate 4: localStorage.getItem("catalog_analytics_consent") == "allowed"
    if (consent !== 'allowed') {
      return { allowed: false, reason: 'consent_not_allowed' };
    }

    return { allowed: true, measurementId: measurementId };
  }

  function checkAndInitGtag() {
    var gateResult = checkGates();
    if (!gateResult.allowed) {
      return false;
    }

    if (gtagInitialized) {
      return true;
    }

    var measurementId = gateResult.measurementId;

    if (typeof window.gtag !== 'function') {
      window.dataLayer = window.dataLayer || [];
      window.gtag = function () {
        window.dataLayer.push(arguments);
      };

      if (!window.__CATALOG_DISABLE_SCRIPT_INJECTION__) {
        var script = document.createElement('script');
        script.async = true;
        script.src = 'https://www.googletagmanager.com/gtag/js?id=' + encodeURIComponent(measurementId);
        document.head.appendChild(script);
      }
    }

    try {
      window.gtag('js', new Date());
      window.gtag('config', measurementId, {
        send_page_view: false,
        allow_google_signals: false,
        restricted_data_processing: true,
      });
      gtagInitialized = true;
    } catch (e) {
      console.warn('Error configuring gtag', e);
    }

    return true;
  }

  function setConsent(status) {
    try {
      if (status === 'allowed') {
        localStorage.setItem(STORAGE_KEY, 'allowed');
      } else if (status === 'declined') {
        localStorage.setItem(STORAGE_KEY, 'declined');
      } else {
        localStorage.removeItem(STORAGE_KEY);
      }
    } catch (_) {}

    if (status === 'declined') {
      if (typeof window.gtag === 'function') {
        try {
          window.gtag('consent', 'update', {
            analytics_storage: 'denied',
          });
        } catch (_) {}
      }
    } else if (status === 'allowed') {
      checkAndInitGtag();
    }
  }

  function trackEvent(name, params) {
    var gateResult = checkGates();
    if (!gateResult.allowed) {
      return;
    }

    if (!gtagInitialized) {
      if (!checkAndInitGtag()) {
        return;
      }
    }

    // Strict parameter sanitization
    var cleanParams = {};
    if (name === 'catalog_object_open') {
      cleanParams = {
        object_kind: String(params.object_kind || ''),
        object_id: String(params.object_id || ''),
      };
    } else if (name === 'catalog_filter_apply') {
      cleanParams = {
        category: String(params.category || ''),
      };
    } else if (name === 'catalog_search_empty') {
      cleanParams = {
        result_count: 0,
      };
    } else if (name === 'catalog_maps_click') {
      cleanParams = {
        object_kind: String(params.object_kind || ''),
        object_id: String(params.object_id || ''),
      };
    } else if (name === 'catalog_gpx_download') {
      cleanParams = {
        object_kind: 'route',
        object_id: String(params.object_id || ''),
      };
    } else {
      console.warn('Unknown catalog analytics event:', name);
      return;
    }

    try {
      if (typeof window.gtag === 'function') {
        window.gtag('event', name, cleanParams);
      }
    } catch (e) {
      console.warn('Error tracking event', e);
    }
  }

  window.CatalogAnalytics = {
    init: checkAndInitGtag,
    getConsent: getConsent,
    setConsent: setConsent,
    trackEvent: trackEvent,
    isAnalyticsActive: function () {
      return checkGates().allowed;
    },
  };

  // Run initial check on load in case consent is already granted
  if (getConsent() === 'allowed') {
    checkAndInitGtag();
  }
})();
