/*
  FlightDeck EFB feasibility test for MSFS 2024.

  An EFB app with two tabs:
    FlightDeck  FlightDeck's own page (http://127.0.0.1:8420/, the "Other
                devices" server on the same PC) in a frame.
    Check       What the EFB's browser engine (Coherent GT) can do: the
                things FlightDeck's page relies on (CSS variables, grid, flex
                gap, WebGL for the map, modern JavaScript, storage), and
                whether FlightDeck's server answers.

  Plain JavaScript, no build step. It uses two globals the EFB provides:
    window.EFB_API   the EFB container (use() registers an app)
    window.msfssdk   @microsoft/msfs-sdk (DisplayComponent, FSComponent, Subject)
  so no part of the SDK is shipped with it.

  The registration pattern follows the MIT-licensed msfs-efb-iframe project
  (github.com/RCCSilva/msfs-efb-iframe), itself built on Asobo's EFB
  Template Sample.

  The EFB browser is old: this file keeps to plain CSS (no variables, grid
  or calc) and ES2017 JavaScript, and uses no special characters in text.
*/
(function () {
  'use strict';

  var APP_NAME = 'FlightDeck';
  // Must match this file's name, its folder and layout.json, or the EFB
  // silently ignores the app.
  var INTERNAL_NAME = 'FlightDeckTest';
  var VERSION = '0.1.0';
  var FLIGHTDECK_URL = 'http://127.0.0.1:8420/';
  var EFB_API_VERSION = '1.0.3';
  var BASE_URL = 'coui://html_ui/efb_ui/efb_apps/' + INTERNAL_NAME;
  var STYLE_ID = 'flightdeck-efb-test-style';

  function getSdk() {
    return window.msfssdk || (typeof msfssdk !== 'undefined' ? msfssdk : null);
  }

  function injectStyle() {
    if (document.getElementById(STYLE_ID)) return;
    var css =
      '.fdt-root{position:absolute;left:0;top:0;right:0;bottom:0;overflow:hidden;background:#1c1e21;' +
      'font:15px/1.4 Arial,sans-serif;color:#ececed;}' +
      '.fdt-bar{position:absolute;left:0;top:0;right:0;height:44px;background:#141619;border-bottom:1px solid #34383d;}' +
      '.fdt-tab{float:left;height:44px;padding:0 18px;border:0;background:transparent;color:#9ba0a7;' +
      'font:bold 15px/44px Arial,sans-serif;cursor:pointer;}' +
      '.fdt-tab.on{color:#f0b23a;border-bottom:3px solid #f0b23a;}' +
      '.fdt-status{float:right;padding:0 14px;font:12px/44px Arial,sans-serif;color:#9ba0a7;white-space:nowrap;}' +
      '.fdt-body{position:absolute;left:0;top:45px;right:0;bottom:0;overflow:hidden;}' +
      '.fdt-frame{position:absolute;left:0;top:0;width:100%;height:100%;border:0;background:#1c1e21;}' +
      '.fdt-check{position:absolute;left:0;top:0;right:0;bottom:0;overflow-y:auto;padding:18px 22px;}' +
      '.fdt-check h2{margin:0 0 4px;font:bold 20px/1.3 Arial,sans-serif;color:#ececed;}' +
      '.fdt-check p{margin:0 0 12px;color:#9ba0a7;font-size:13px;}' +
      '.fdt-row{padding:7px 0;border-bottom:1px solid #2c3034;}' +
      '.fdt-res{display:inline-block;width:54px;font:bold 13px Arial,sans-serif;}' +
      '.fdt-ok{color:#5cc08a;}.fdt-no{color:#e26a5a;}.fdt-info{color:#f0b23a;}' +
      '.fdt-name{display:inline-block;width:250px;}' +
      '.fdt-detail{color:#9ba0a7;font-size:13px;}' +
      '.fdt-msg{position:absolute;left:10%;right:10%;top:30%;padding:18px;text-align:center;' +
      'background:#2c3034;border:1px solid #4a4f57;border-radius:8px;color:#ececed;}' +
      '.fdt-hidden{display:none;}';
    var el = document.createElement('style');
    el.id = STYLE_ID;
    el.type = 'text/css';
    el.appendChild(document.createTextNode(css));
    document.head.appendChild(el);
  }

  // ------------------------------------------------------------------
  // The compatibility check. Each test calls done(result, detail) where
  // result is 'ok', 'no' or 'info'.
  // ------------------------------------------------------------------

  function probeBox(host, style) {
    var el = document.createElement('div');
    el.setAttribute('style', 'position:absolute;left:-9999px;top:0;' + style);
    host.appendChild(el);
    return el;
  }

  function child(parent, style) {
    var el = document.createElement('div');
    el.setAttribute('style', style);
    parent.appendChild(el);
    return el;
  }

  function tests(host) {
    return [
      ['Browser engine', function (done) {
        done('info', navigator.userAgent || 'unknown');
      }],
      ['Screen size (app area)', function (done) {
        done('info', host.offsetWidth + ' x ' + host.offsetHeight + ' px, device pixel ratio ' + (window.devicePixelRatio || 1));
      }],
      ['CSS variables', function (done) {
        var box = probeBox(host, '');
        box.style.setProperty('--fdt-w', '12px');
        var c = child(box, 'width:var(--fdt-w);height:1px;');
        var w = c.offsetWidth;
        host.removeChild(box);
        done(w === 12 ? 'ok' : 'no', 'width came out ' + w + ' px (want 12)');
      }],
      ['CSS grid layout', function (done) {
        var box = probeBox(host, 'display:grid;grid-template-columns:10px 20px;width:30px;');
        child(box, 'height:1px;');
        var b = child(box, 'height:1px;');
        var x = b.offsetLeft;
        var shown = box.style.display;
        host.removeChild(box);
        done(x === 10 ? 'ok' : 'no', 'display=' + (shown || 'rejected') + ', second column at ' + x + ' px (want 10)');
      }],
      ['Flexbox with gap', function (done) {
        var box = probeBox(host, 'display:flex;gap:7px;');
        child(box, 'width:10px;height:1px;');
        var b = child(box, 'width:10px;height:1px;');
        var x = b.offsetLeft;
        host.removeChild(box);
        done(x === 17 ? 'ok' : (x === 10 ? 'no' : 'info'), 'second item at ' + x + ' px (want 17)');
      }],
      ['CSS calc()', function (done) {
        var box = probeBox(host, '');
        var c = child(box, 'width:calc(10px + 5px);height:1px;');
        var w = c.offsetWidth;
        host.removeChild(box);
        done(w === 15 ? 'ok' : 'no', 'width came out ' + w + ' px (want 15)');
      }],
      ['WebGL (the map needs it)', function (done) {
        try {
          var cv = document.createElement('canvas');
          var gl = cv.getContext('webgl') || cv.getContext('experimental-webgl');
          if (!gl) return done('no', 'no WebGL context');
          var info = gl.getExtension('WEBGL_debug_renderer_info');
          var r = info ? gl.getParameter(info.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
          done('ok', String(r) + ', max texture ' + gl.getParameter(gl.MAX_TEXTURE_SIZE));
        } catch (e) {
          done('no', String(e));
        }
      }],
      ['WebGL 2', function (done) {
        try {
          var gl2 = document.createElement('canvas').getContext('webgl2');
          done(gl2 ? 'ok' : 'no', gl2 ? 'available' : 'not available');
        } catch (e) {
          done('no', String(e));
        }
      }],
      ['Optional chaining (a?.b)', function (done) {
        try {
          done(new Function('var a={b:1};return a?.b;')() === 1 ? 'ok' : 'no', '');
        } catch (e) {
          done('no', 'syntax not understood');
        }
      }],
      ['Nullish coalescing (a ?? b)', function (done) {
        try {
          done(new Function('return null ?? 2;')() === 2 ? 'ok' : 'no', '');
        } catch (e) {
          done('no', 'syntax not understood');
        }
      }],
      ['Class fields (class { x = 1 })', function (done) {
        try {
          done(new Function('return new (class { x = 1 })().x;')() === 1 ? 'ok' : 'no', '');
        } catch (e) {
          done('no', 'syntax not understood');
        }
      }],
      ['ES modules', function (done) {
        var s = document.createElement('script');
        done('noModule' in s ? 'ok' : 'no', 'noModule' in s ? 'type="module" known' : 'not supported');
      }],
      ['fetch, Promise, WebSocket', function (done) {
        var have = [];
        var miss = [];
        ['fetch', 'Promise', 'WebSocket', 'ResizeObserver', 'IntersectionObserver'].forEach(function (n) {
          (typeof window[n] !== 'undefined' ? have : miss).push(n);
        });
        done(miss.length ? (miss.indexOf('fetch') >= 0 || miss.indexOf('WebSocket') >= 0 ? 'no' : 'info') : 'ok',
          (have.length ? 'have ' + have.join(', ') : '') + (miss.length ? '; missing ' + miss.join(', ') : ''));
      }],
      ['Local storage (keeps the pairing)', function (done) {
        try {
          localStorage.setItem('fdt-probe', '1');
          var v = localStorage.getItem('fdt-probe');
          localStorage.removeItem('fdt-probe');
          done(v === '1' ? 'ok' : 'no', v === '1' ? 'works' : 'value not kept');
        } catch (e) {
          done('no', String(e));
        }
      }],
      ['FlightDeck server on this PC', function (done) {
        var img = new Image();
        var t0 = Date.now();
        var finished = false;
        var timer = window.setTimeout(function () {
          if (finished) return;
          finished = true;
          done('no', 'no answer from ' + FLIGHTDECK_URL + ' in 4 s. Is Settings, Other devices turned on in FlightDeck?');
        }, 4000);
        img.onload = function () {
          if (finished) return;
          finished = true;
          window.clearTimeout(timer);
          done('ok', 'answered in ' + (Date.now() - t0) + ' ms');
        };
        img.onerror = function () {
          if (finished) return;
          finished = true;
          window.clearTimeout(timer);
          done('no', 'refused or not running: turn on Settings, Other devices in FlightDeck');
        };
        img.src = FLIGHTDECK_URL + 'favicon.svg?t=' + t0;
      }],
      ['Frame rate in the EFB', function (done) {
        var n = 0;
        var t0 = 0;
        var raf = window.requestAnimationFrame;
        if (!raf) return done('info', 'requestAnimationFrame missing');
        function tick(t) {
          if (!t0) t0 = t;
          n += 1;
          if (t - t0 < 1000) raf(tick);
          else done('info', Math.round((n * 1000) / (t - t0)) + ' frames a second');
        }
        raf(tick);
      }],
    ];
  }

  function createViewClass(sdk) {
    return class FlightDeckTestView extends sdk.DisplayComponent {
      constructor(props) {
        super(props);
        this.rootRef = sdk.FSComponent.createRef();
        this.frame = null;
        this.frameLoadedMs = null;
        this.frameStart = 0;
        this.tab = 'deck';
        this.checked = false;
        this.pageKeyActions = new Map();
      }

      render() {
        var h = sdk.FSComponent.buildComponent;
        return h('div', { class: 'fdt-root', ref: this.rootRef },
          h('div', { class: 'fdt-bar' },
            h('button', { class: 'fdt-tab on' }, 'FlightDeck'),
            h('button', { class: 'fdt-tab' }, 'Compatibility check'),
            h('button', { class: 'fdt-tab' }, 'Reload'),
            h('span', { class: 'fdt-status' }, 'FlightDeck EFB test ' + VERSION)
          ),
          h('div', { class: 'fdt-body' },
            h('div', { class: 'fdt-deck' }),
            h('div', { class: 'fdt-check fdt-hidden' })
          )
        );
      }

      onAfterRender() {
        var root = this.rootRef.instance;
        if (!root) return;
        var self = this;
        this.deckEl = root.querySelector('.fdt-deck');
        this.checkEl = root.querySelector('.fdt-check');
        this.statusEl = root.querySelector('.fdt-status');
        this.tabs = root.querySelectorAll('.fdt-tab');
        var names = ['deck', 'check', 'reload'];
        for (var i = 0; i < this.tabs.length; i++) {
          this.tabs[i].setAttribute('data-tab', names[i]);
          this.tabs[i].addEventListener('click', function (e) {
            self.choose(e.currentTarget.getAttribute('data-tab'));
          });
        }
        this.createFrame();
      }

      choose(tab) {
        if (tab === 'reload') {
          if (this.tab === 'deck') this.createFrame();
          else this.runChecks();
          return;
        }
        this.tab = tab;
        for (var i = 0; i < this.tabs.length; i++) {
          var t = this.tabs[i];
          t.className = t.getAttribute('data-tab') === tab ? 'fdt-tab on' : 'fdt-tab';
        }
        this.deckEl.className = tab === 'deck' ? 'fdt-deck' : 'fdt-deck fdt-hidden';
        this.checkEl.className = tab === 'check' ? 'fdt-check' : 'fdt-check fdt-hidden';
        if (tab === 'check' && !this.checked) this.runChecks();
      }

      createFrame() {
        if (!this.deckEl) return;
        if (this.frame && this.frame.parentNode) this.frame.parentNode.removeChild(this.frame);
        var self = this;
        var f = document.createElement('iframe');
        f.className = 'fdt-frame';
        f.setAttribute('frameborder', '0');
        this.frameStart = Date.now();
        this.frameLoadedMs = null;
        f.onload = function () {
          self.frameLoadedMs = Date.now() - self.frameStart;
          self.status('FlightDeck page loaded in ' + self.frameLoadedMs + ' ms');
        };
        this.frame = f;
        this.deckEl.appendChild(f);
        f.src = FLIGHTDECK_URL;
        this.status('Loading ' + FLIGHTDECK_URL + ' ...');
      }

      status(text) {
        if (this.statusEl) this.statusEl.textContent = text;
      }

      runChecks() {
        var el = this.checkEl;
        this.checked = true;
        el.innerHTML = '';
        var head = document.createElement('div');
        head.innerHTML =
          '<h2>Compatibility check</h2>' +
          '<p>What the EFB browser can do. Please take a screenshot of this whole list (scroll if needed) and send it.</p>';
        el.appendChild(head);
        var list = tests(el);
        var frameRow = this.row(el, 'FlightDeck page in the frame');
        // The frame reports "loaded" for an error page too: the FlightDeck
        // tab shows what really came up.
        this.fill(frameRow, 'info',
          this.frameLoadedMs != null ? 'frame finished loading in ' + this.frameLoadedMs + ' ms; see the FlightDeck tab for what it shows' : 'not loaded (yet)');
        var self = this;
        list.forEach(function (t) {
          var row = self.row(el, t[0]);
          try {
            t[1](function (res, detail) {
              self.fill(row, res, detail);
            });
          } catch (e) {
            self.fill(row, 'no', 'test failed: ' + e);
          }
        });
      }

      row(parent, name) {
        var r = document.createElement('div');
        r.className = 'fdt-row';
        r.innerHTML = '<span class="fdt-res fdt-info">...</span><span class="fdt-name"></span><span class="fdt-detail"></span>';
        r.childNodes[1].textContent = name;
        parent.appendChild(r);
        return r;
      }

      fill(row, res, detail) {
        var tag = row.childNodes[0];
        tag.className = 'fdt-res ' + (res === 'ok' ? 'fdt-ok' : res === 'no' ? 'fdt-no' : 'fdt-info');
        tag.textContent = res === 'ok' ? 'OK' : res === 'no' ? 'NO' : 'INFO';
        row.childNodes[2].textContent = detail || '';
      }

      // EFB lifecycle.
      onOpen() {}
      onResume() {}
      onPause() {}
      onClose() {}
      onUpdate() {}
      routeGamepadInteractionEvent() {}
      handlePageKeyAction() {}

      destroy() {
        if (this.frame) {
          this.frame.onload = null;
          this.frame.src = 'about:blank';
        }
        super.destroy();
      }
    };
  }

  function registerApp() {
    var sdk = getSdk();
    var efb = window.EFB_API;
    if (!sdk || !efb || typeof efb.use !== 'function') return false;

    injectStyle();
    var View = createViewClass(sdk);

    class FlightDeckTestApp {
      constructor() {
        this.available = sdk.Subject.create(true);
        this.BootMode = 0; // AppBootMode.COLD
        this.SuspendMode = 0; // AppSuspendMode.SLEEP
        this._isReady = false;
        this._favoriteIndex = -1;
        this.bus = null;
      }

      get name() { return APP_NAME; }
      get internalName() { return INTERNAL_NAME; }
      get icon() { return BASE_URL + '/Assets/app-icon.svg'; }
      get isReady() { return this._isReady; }
      get favoriteIndex() { return this._favoriteIndex; }
      set favoriteIndex(value) { this._favoriteIndex = value; }
      get compatibleAircraftModels() { return undefined; }
      get unitsSettingsManager() { return this._unitsSettingsManager; }
      getIsSearchable() { return true; }
      getIsFavoritable() { return true; }
      getVersion() { return VERSION; }

      async install() {}

      async _install(props) {
        this.bus = props && props.bus;
        this._unitsSettingsManager = props && props.unitsSettingManager;
        this.notificationManager = props && props.notificationManager;
        this.efbSettingsManager = props && props.efbSettingsManager;
        this.options = props && props.options;
        await this.install(props);
        this._isReady = true;
        try {
          if (window.Coherent && typeof window.Coherent.trigger === 'function') {
            window.Coherent.trigger('EFB_APP_INSTALLED', APP_NAME, INTERNAL_NAME, EFB_API_VERSION, VERSION);
          }
        } catch (e) {
          /* informational only */
        }
      }

      render() {
        return sdk.FSComponent.buildComponent(View, { bus: this.bus });
      }
    }

    efb.use(FlightDeckTestApp);
    return true;
  }

  // The EFB and this script load independently and use() throws until the
  // EFB is ready: retry for about 15 s.
  function registerWhenReady(attempt) {
    var ok = false;
    try {
      ok = registerApp();
    } catch (e) {
      ok = false;
    }
    if (!ok && attempt < 150) {
      window.setTimeout(function () {
        registerWhenReady(attempt + 1);
      }, 100);
    }
  }

  registerWhenReady(1);
})();
