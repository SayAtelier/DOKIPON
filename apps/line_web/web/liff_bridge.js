/*
 * LIFF bridge — the ONLY place the LINE LIFF SDK is touched.
 *
 * Contract with the Dart side (`packages/api_client/lib/src/liff_bridge.dart`):
 *   window.__gemarcLiff = {
 *     ready: boolean,        // SDK loaded AND liff.init resolved
 *     isInClient: boolean,   // running inside the LINE app WebView
 *     isLoggedIn: boolean,   // user granted account access
 *     idToken: string|null,  // ONE-SHOT. Send it to the identity service, never persist.
 *     errorCode: string|null // LIFF_ID_NOT_CONFIGURED | SDK_LOAD_FAILED | INIT_FAILED | ...
 *   }
 *
 * Design-atlas invariants honoured here (LIFF_連携・受入設計.md):
 *   LIFF-001  init succeeds → read-only storefront, no automatic profile fetch
 *   LIFF-002  no LIFF ID compiled in → LIFF_ID_NOT_CONFIGURED, product features continue
 *   LIFF-003  SDK network failure → degrade to plain Flutter Web, no redirect loop
 *   LIFF-004  external browser → never call client-only APIs
 *   LIFF-005  not logged in → prompt only when authorization is actually required
 *   LIFF-006  revoked consent → no authorized data is fetched
 *
 * SECURITY: the LIFF Channel Secret is never present in this file or in the
 * bundle. Only the public LIFF ID is compiled in, via --dart-define.
 */
(function () {
  'use strict';

  // Compiled in at build time. Absent in dev builds.
  var LIFF_ID =
    (typeof __GEMARC_LIFF_ID__ !== 'undefined' && __GEMARC_LIFF_ID__) || '';
  var SDK_URL = 'https://sdk.line-apps.com/liff/edge-liff-sdk.js';

  var state = {
    ready: false,
    isInClient: false,
    isLoggedIn: false,
    idToken: null,
    errorCode: null,
  };

  window.__gemarcLiff = state;

  function publish(patch) {
    Object.assign(state, patch);
  }

  function setUnconfigured() {
    publish({
      ready: true,
      isInClient: false,
      isLoggedIn: false,
      idToken: null,
      errorCode: 'LIFF_ID_NOT_CONFIGURED',
    });
  }

  function loadSdk() {
    return new Promise(function (resolve, reject) {
      var existing = document.querySelector('script[data-gemarc-liff]');
      if (existing) {
        resolve();
        return;
      }
      var s = document.createElement('script');
      s.src = SDK_URL;
      s.async = true;
      s.setAttribute('data-gemarc-liff', '1');
      s.onload = function () {
        resolve();
      };
      // LIFF-003: a network failure must degrade, never redirect or reload.
      s.onerror = function () {
        reject(new Error('SDK_LOAD_FAILED'));
      };
      document.head.appendChild(s);
    });
  }

  /**
   * Boot the SDK. Safe to call more than once — subsequent calls are no-ops
   * that just re-publish the existing state.
   */
  window.__gemarcLiffInit = function () {
    if (!LIFF_ID) {
      // LIFF-002. Not an error for the product: browsing still works.
      setUnconfigured();
      return Promise.resolve(state);
    }

    return loadSdk()
      .then(function () {
        if (!window.liff) {
          throw new Error('INIT_FAILED');
        }
        return window.liff.init({ liffId: LIFF_ID });
      })
      .then(function () {
        // LIFF-001: report only the environment. Do NOT auto-fetch a profile.
        publish({
          ready: true,
          isInClient: !!window.liff.isInClient(),
          isLoggedIn: !!window.liff.isLoggedIn(),
          errorCode: null,
        });
        return state;
      })
      .catch(function (err) {
        // LIFF-003: degrade to plain Flutter Web. No redirect, no reload.
        publish({
          ready: false,
          isInClient: false,
          isLoggedIn: false,
          idToken: null,
          errorCode: (err && err.message) || 'INIT_FAILED',
        });
        return state;
      });
  };

  /**
   * Request an id_token and hand it to Dart exactly once.
   *
   * LIFF-004/005/006: returns null rather than forcing a login. The caller
   * decides whether to prompt; a guest session is a valid outcome.
   *
   * The token is cached in `state` so Dart can read it after the async hop, and
   * `__gemarcLiffClearToken` wipes it immediately after the exchange.
   */
  window.__gemarcLiffGetIdToken = function () {
    if (!state.ready || !state.isInClient || !state.isLoggedIn) {
      return Promise.resolve(null); // LIFF-004 / LIFF-005
    }
    return window.liff
      .getIDToken()
      .then(function (r) {
        publish({ idToken: r.id_token });
        return r.id_token;
      })
      .catch(function () {
        publish({ idToken: null });
        return null; // LIFF-006: revoked consent or denial is not fatal
      });
  };

  /** Called by Dart immediately after the token is sent to the server. */
  window.__gemarcLiffClearToken = function () {
    publish({ idToken: null });
  };

  /** A random nonce bound to this page instance, for replay defence. */
  window.__gemarcLiffNonce = function () {
    if (window.crypto && window.crypto.getRandomValues) {
      var a = new Uint8Array(16);
      window.crypto.getRandomValues(a);
      return Array.prototype.map
        .call(a, function (b) {
          return ('0' + b.toString(16)).slice(-2);
        })
        .join('');
    }
    return String(Date.now()) + Math.random().toString(16).slice(2);
  };
})();
