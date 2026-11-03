/*
  FlightDeck for the MSFS 2024 EFB.

  Shows what FlightDeck on this PC knows, on the tablet in the cockpit:
    Flight       the current flight as a boarding pass with live altitude,
                 speeds and progress; or, before a flight, Start a flight
                 (with the latest SimBrief plan or without one).
    Last flight  the landing and the pilot score of the last saved flight.
    Pilot        rating, rank and XP.

  It talks to FlightDeck's local server (http://127.0.0.1:8420, this PC only)
  and pairs itself the first time through /api/pair-local, which FlightDeck
  only answers for programs on this same PC. FlightDeck turns that server on
  by itself while the EFB app is installed.

  Plain JavaScript, no build step. It uses two globals the EFB provides:
    window.EFB_API   the EFB container (use() registers an app)
    window.msfssdk   @microsoft/msfs-sdk (DisplayComponent, FSComponent, Subject)

  The EFB browser (Coherent GT) is old: ES2017 at most (no ?. or ??, no
  class fields), no flexbox gap, no ResizeObserver. The EFB draws its clock
  and bell over the top of the app and its dock over the bottom, so the app
  keeps those strips clear.

  The registration pattern follows the MIT-licensed msfs-efb-iframe project
  (github.com/RCCSilva/msfs-efb-iframe), itself built on Asobo's EFB
  Template Sample.
*/
(function () {
  'use strict';

  var APP_NAME = 'FlightDeck';
  // Must match this file's name, its folder and layout.json.
  var INTERNAL_NAME = 'FlightDeck';
  var VERSION = '__FD_VERSION__';
  var SERVER = 'http://127.0.0.1:8420';
  var EFB_API_VERSION = '1.0.3';
  var BASE_URL = 'coui://html_ui/efb_ui/efb_apps/' + INTERNAL_NAME;
  var STYLE_ID = 'flightdeck-efb-style';
  var TOKEN_KEY = 'flightdeck-efb-token';
  var POLL_MS = 2000;
  var RETRY_MS = 4000;

  function getSdk() {
    return window.msfssdk || (typeof msfssdk !== 'undefined' ? msfssdk : null);
  }

  // ------------------------------------------------------------------
  // Look
  // ------------------------------------------------------------------

  function injectStyle() {
    if (document.getElementById(STYLE_ID)) return;
    var css = [
      '.fd-root{position:absolute;left:0;top:0;right:0;bottom:0;overflow:hidden;background:#17191c;',
      'font:16px/1.35 Arial,Helvetica,sans-serif;color:#ececed;}',
      // The EFB's clock and bell sit over the top 50 px, its dock over the bottom 80 px.
      '.fd-top{position:absolute;left:0;right:0;top:56px;height:52px;padding:0 24px;}',
      '.fd-brand{float:left;height:52px;line-height:52px;font-weight:bold;font-size:20px;color:#ececed;}',
      '.fd-brand img{width:28px;height:28px;vertical-align:middle;margin:-4px 10px 0 0;}',
      '.fd-seg{float:right;margin-top:7px;background:#23262a;border:1px solid #363a40;border-radius:20px;padding:3px;}',
      '.fd-seg button{display:inline-block;height:32px;padding:0 20px;margin:0;border:0;border-radius:16px;',
      'background:transparent;color:#a3a8af;font:bold 15px/32px Arial,Helvetica,sans-serif;cursor:pointer;}',
      '.fd-seg button.on{background:#f0b23a;color:#1b1406;}',
      '.fd-dot{float:right;margin:22px 16px 0 0;width:9px;height:9px;border-radius:5px;background:#e26a5a;}',
      '.fd-dot.ok{background:#5cc08a;}',
      '.fd-body{position:absolute;left:0;right:0;top:116px;bottom:84px;overflow-y:auto;padding:0 24px 16px;}',
      '.fd-hidden{display:none;}',
      'h2.fd-h{margin:4px 0 2px;font-size:24px;line-height:1.2;}',
      'p.fd-sub{margin:0 0 14px;color:#a3a8af;font-size:15px;}',
      '.fd-card{background:#202327;border:1px solid #33373c;border-radius:12px;padding:16px 18px;margin-bottom:12px;}',
      '.fd-label{color:#8d939b;font-size:12px;letter-spacing:1px;text-transform:uppercase;}',
      '.fd-big{font-size:30px;font-weight:bold;line-height:1.15;}',
      '.fd-dim{color:#a3a8af;}',
      '.fd-good{color:#5cc08a;}.fd-warn{color:#f0b23a;}.fd-bad{color:#e26a5a;}',
      // Boarding pass.
      '.fd-pass{background:#f4efe3;color:#1b1406;border-radius:14px;overflow:hidden;margin-bottom:12px;}',
      '.fd-pass-head{background:#f0b23a;padding:10px 18px;font-weight:bold;}',
      '.fd-pass-head .fd-r{float:right;font-weight:normal;}',
      '.fd-route{display:table;width:100%;padding:14px 18px 10px;box-sizing:border-box;}',
      '.fd-route > div{display:table-cell;vertical-align:middle;}',
      '.fd-route .fd-icao{font-size:44px;font-weight:bold;line-height:1;letter-spacing:1px;}',
      '.fd-route .fd-name{font-size:13px;color:#6a6253;margin-top:3px;}',
      '.fd-route .fd-mid{text-align:center;width:120px;}',
      '.fd-route .fd-end{text-align:right;}',
      '.fd-plane{display:inline-block;width:38px;height:38px;border-radius:19px;background:#f0b23a;}',
      '.fd-plane img{width:24px;height:24px;margin-top:7px;}',
      '.fd-stub{border-top:2px dashed #cbc2ae;padding:10px 18px 14px;}',
      '.fd-cols{display:table;width:100%;table-layout:fixed;}',
      '.fd-cols > div{display:table-cell;vertical-align:top;}',
      '.fd-pass .fd-label{color:#7c7362;}',
      '.fd-val{font-size:20px;font-weight:bold;}',
      '.fd-bar{height:8px;background:#ddd4c1;border-radius:4px;margin-top:10px;overflow:hidden;}',
      '.fd-bar i{display:block;height:8px;background:#f0b23a;border-radius:4px;}',
      '.fd-card .fd-bar{background:#33373c;}',
      '.fd-barnote{font-size:13px;color:#6a6253;margin-top:4px;}',
      '.fd-card .fd-barnote{color:#a3a8af;}',
      // Readouts.
      '.fd-read{display:table;width:calc(100% + 20px);table-layout:fixed;border-spacing:10px 0;margin:0 -10px 12px;}',
      '.fd-read > div{display:table-cell;background:#202327;border:1px solid #33373c;border-radius:12px;padding:12px 14px;}',
      '.fd-read .fd-unit{font-size:14px;color:#a3a8af;font-weight:normal;margin-left:4px;}',
      // Buttons.
      '.fd-btn{display:inline-block;height:42px;padding:0 22px;margin:0 10px 0 0;border:1px solid #4a4f57;border-radius:10px;',
      'background:#2b2f34;color:#ececed;font:bold 16px/40px Arial,Helvetica,sans-serif;cursor:pointer;}',
      '.fd-btn.fd-primary{background:#f0b23a;border-color:#f0b23a;color:#1b1406;}',
      '.fd-btn.fd-danger{background:#5a2620;border-color:#8a3a30;color:#ffd9d3;}',
      '.fd-btn[disabled]{opacity:0.5;}',
      '.fd-actions{margin-top:4px;}',
      '.fd-plan{font-size:15px;}',
      '.fd-plan .fd-icao2{font-size:26px;font-weight:bold;}',
      '.fd-msg{margin:40px auto 0;max-width:560px;text-align:center;}',
      '.fd-msg img{width:64px;height:64px;opacity:0.9;}',
      '.fd-err{color:#e26a5a;font-size:14px;margin-top:8px;}',
      // Scores.
      '.fd-score{float:left;width:120px;height:120px;border-radius:60px;border:8px solid #33373c;box-sizing:border-box;',
      'text-align:center;margin-right:22px;}',
      '.fd-score b{display:block;font-size:40px;line-height:1;margin-top:26px;}',
      '.fd-score span{font-size:12px;color:#a3a8af;}',
      '.fd-score.fd-good{border-color:#5cc08a;}.fd-score.fd-warn{border-color:#f0b23a;}.fd-score.fd-bad{border-color:#e26a5a;}',
      '.fd-parts{overflow:hidden;}',
      '.fd-part{margin-bottom:10px;}',
      '.fd-part .fd-r{float:right;font-weight:bold;}',
      '.fd-clear{clear:both;}',
      '.fd-rank{font-size:26px;font-weight:bold;color:#f0b23a;}',
    ].join('');
    var el = document.createElement('style');
    el.id = STYLE_ID;
    el.type = 'text/css';
    el.appendChild(document.createTextNode(css));
    document.head.appendChild(el);
  }

  // ------------------------------------------------------------------
  // Formatting
  // ------------------------------------------------------------------

  function esc(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function pad(n) {
    return n < 10 ? '0' + n : String(n);
  }

  /** 14:05z */
  function zulu(ms) {
    if (!ms) return '--:--';
    var d = new Date(ms);
    return pad(d.getUTCHours()) + ':' + pad(d.getUTCMinutes()) + 'z';
  }

  /** 1:05 from minutes. */
  function hm(min) {
    if (min == null || !isFinite(min) || min < 0) return '-:--';
    var m = Math.round(min);
    return Math.floor(m / 60) + ':' + pad(m % 60);
  }

  function num(v) {
    return Math.round(v).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  }

  function signed(v) {
    var r = Math.round(v);
    return (r > 0 ? '+' : '') + num(r);
  }

  function dateOf(ms) {
    var d = new Date(ms);
    var m = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    return d.getUTCDate() + ' ' + m[d.getUTCMonth()] + ' ' + d.getUTCFullYear();
  }

  function tone(score) {
    if (score == null) return '';
    return score >= 80 ? 'fd-good' : score >= 60 ? 'fd-warn' : 'fd-bad';
  }

  /** Great-circle distance in nautical miles. */
  function gc(lat1, lon1, lat2, lon2) {
    var r = Math.PI / 180;
    var a = Math.sin(((lat2 - lat1) * r) / 2);
    var b = Math.sin(((lon2 - lon1) * r) / 2);
    var h = a * a + Math.cos(lat1 * r) * Math.cos(lat2 * r) * b * b;
    return 2 * 3440.065 * Math.asin(Math.sqrt(h));
  }

  function logo(dark) {
    return '<img src="' + BASE_URL + '/Assets/' + (dark ? 'bird-dark.svg' : 'app-icon.svg') + '">';
  }

  function bar(fraction) {
    var p = Math.max(0, Math.min(1, fraction || 0)) * 100;
    return '<div class="fd-bar"><i style="width:' + p.toFixed(1) + '%"></i></div>';
  }

  // ------------------------------------------------------------------
  // FlightDeck's server
  // ------------------------------------------------------------------

  function Client() {
    this.token = null;
    try {
      this.token = window.localStorage.getItem(TOKEN_KEY);
    } catch (e) {
      this.token = null;
    }
    this.pairing = null;
  }

  Client.prototype.pair = function () {
    var self = this;
    if (this.pairing) return this.pairing;
    this.pairing = fetch(SERVER + '/api/pair-local', { method: 'POST' })
      .then(function (r) {
        if (!r.ok) throw new Error('pair ' + r.status);
        return r.json();
      })
      .then(function (j) {
        self.token = j.token;
        try {
          window.localStorage.setItem(TOKEN_KEY, j.token);
        } catch (e) {
          /* kept for this session only */
        }
        self.pairing = null;
        return j.token;
      }, function (e) {
        self.pairing = null;
        throw e;
      });
    return this.pairing;
  };

  /** Runs a FlightDeck command; pairs first, and again if the pairing was removed. */
  Client.prototype.call = function (cmd, args, retried) {
    var self = this;
    var ready = this.token ? Promise.resolve(this.token) : this.pair();
    return ready.then(function (token) {
      return fetch(SERVER + '/api/call/' + cmd, {
        method: 'POST',
        headers: { 'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json' },
        body: JSON.stringify(args || {}),
      });
    }).then(function (r) {
      if (r.status === 401 && !retried) {
        self.token = null;
        return self.call(cmd, args, true);
      }
      return r.json().then(function (j) {
        if (!r.ok) {
          var e = new Error((j && j.error) || 'FlightDeck said no (' + r.status + ')');
          e.answered = true;
          throw e;
        }
        return j;
      });
    });
  };

  // ------------------------------------------------------------------
  // Screens
  // ------------------------------------------------------------------

  function flightScreen(st, ui) {
    if (!st) return '';
    if (st.live) return liveFlight(st, ui);
    var h = '';
    if (st.link === 'connected') {
      h += '<h2 class="fd-h"><span class="fd-good">Connected</span> to ' + esc(st.sim_label || 'the sim') + '</h2>';
      h += '<p class="fd-sub">' + (st.armed
        ? 'Flight started. Tracking begins the moment you are in the cockpit.'
        : 'Tracking starts by itself when the battery is on or an engine is running. Or start a flight here to load your SimBrief plan first.') + '</p>';
    } else {
      h += '<h2 class="fd-h">Ready when you are</h2>';
      h += '<p class="fd-sub">FlightDeck is waiting for the sim to load a flight.</p>';
    }
    if (st.notice) h += '<div class="fd-card fd-warn">' + esc(st.notice) + '</div>';
    if (st.plan) h += planCard(st.plan, 'Flight plan');
    if (!st.armed) h += startCard(st, ui);
    return h;
  }

  function planCard(p, title) {
    var h = '<div class="fd-card fd-plan"><div class="fd-label">' + esc(title) + '</div>';
    h += '<div style="margin-top:6px"><span class="fd-icao2">' + esc(p.origin.icao) + '</span> <span class="fd-dim">to</span> ';
    h += '<span class="fd-icao2">' + esc(p.destination.icao) + '</span>';
    var bits = [];
    if (p.callsign) bits.push(esc(p.callsign));
    if (p.aircraft_icao) bits.push(esc(p.aircraft_icao));
    if (p.distance_nm) bits.push(num(p.distance_nm) + ' nm');
    if (p.ete_min) bits.push(hm(p.ete_min));
    if (bits.length) h += ' <span class="fd-dim" style="margin-left:12px">' + bits.join(' &middot; ') + '</span>';
    h += '</div>';
    h += '<div class="fd-dim" style="font-size:13px;margin-top:4px">' + esc(p.origin.name) + ' to ' + esc(p.destination.name) + '</div>';
    if (p.route) h += '<div class="fd-dim" style="font-size:13px;margin-top:6px;word-wrap:break-word">' + esc(p.route) + '</div>';
    h += '</div>';
    return h;
  }

  function startCard(st, ui) {
    var h = '';
    if (st.plan) {
      h += '<div class="fd-actions"><button class="fd-btn fd-primary" data-act="start-current">Start this flight</button>';
      h += '<button class="fd-btn" data-act="start-none">Start without a plan</button></div>';
    } else if (ui.simbrief === 'loading') {
      h += '<div class="fd-card fd-dim">Looking for your latest SimBrief plan...</div>';
    } else if (ui.simbrief && ui.simbrief.plan) {
      h += planCard(ui.simbrief.plan, 'Your latest SimBrief plan');
      h += '<div class="fd-actions"><button class="fd-btn fd-primary" data-act="start-simbrief">Start with this plan</button>';
      h += '<button class="fd-btn" data-act="start-none">Start without a plan</button>';
      h += '<button class="fd-btn" data-act="simbrief">Refresh</button></div>';
    } else {
      h += '<div class="fd-card"><div class="fd-label">Start a flight</div>';
      h += '<p class="fd-dim" style="margin:6px 0 12px">' + esc(ui.simbrief && ui.simbrief.error
        ? ui.simbrief.error
        : 'Load your latest SimBrief plan, or just start tracking.') + '</p>';
      h += '<button class="fd-btn fd-primary" data-act="simbrief">Load SimBrief plan</button>';
      h += '<button class="fd-btn" data-act="start-none">Start without a plan</button></div>';
    }
    if (ui.actionError) h += '<div class="fd-err">' + esc(ui.actionError) + '</div>';
    return h;
  }

  function liveFlight(st, ui) {
    var live = st.live;
    var p = st.plan;
    var t = st.telemetry;
    var ac = st.aircraft;
    var callsign = p && p.callsign ? String(p.callsign).toUpperCase() : '';
    var reg = (ac && ac.registration) || (p && p.registration) || '';
    var type = (ac && ac.icao_type) || (p && p.aircraft_icao) || '';
    var headline = callsign || reg || type || 'Your flight';
    var from = st.dep ? { icao: st.dep.icao, name: st.dep.name } : p ? { icao: p.origin.icao, name: p.origin.name } : null;
    var to = p ? { icao: p.destination.icao, name: p.destination.name } : null;

    var left = null;
    if (p && t && (p.destination.lat || p.destination.lon)) left = gc(t.lat, t.lon, p.destination.lat, p.destination.lon);
    var flown = live.distance_nm || 0;
    var progress = left == null ? null : flown + left > 0 ? Math.min(1, flown / (flown + left)) : 0;
    var now = (t && t.t_ms) || Date.now();
    var toGo = left != null && t && !t.on_ground && t.gs_kt > 40 ? (left / t.gs_kt) * 60 : null;
    var since = live.takeoff_ms || live.block_out_ms || null;

    var h = '<h2 class="fd-h">' + esc(st.phase_label || 'Flying') + '</h2>';
    h += '<p class="fd-sub">' + esc(st.demo ? 'Demo flight' : st.sim_label || '') + (t && t.paused ? ', paused' : '') + '</p>';

    h += '<div class="fd-pass"><div class="fd-pass-head">' + esc(headline);
    var sub = [];
    if (type && type !== headline) sub.push(esc(type));
    if (reg && reg !== headline) sub.push(esc(reg));
    h += '<span class="fd-r">' + sub.join(' &middot; ') + '</span></div>';
    h += '<div class="fd-route"><div><div class="fd-icao">' + esc(from ? from.icao : '----') + '</div>';
    h += '<div class="fd-name">' + esc(from ? from.name : 'Departure') + '</div></div>';
    h += '<div class="fd-mid"><span class="fd-plane">' + logo(true) + '</span></div>';
    h += '<div class="fd-end"><div class="fd-icao">' + esc(to ? to.icao : '----') + '</div>';
    h += '<div class="fd-name">' + esc(to ? to.name : 'No flight plan') + '</div></div></div>';

    h += '<div class="fd-stub"><div class="fd-cols">';
    h += '<div><div class="fd-label">Out</div><div class="fd-val">' + zulu(live.block_out_ms) + '</div></div>';
    h += '<div><div class="fd-label">Airborne</div><div class="fd-val">' + zulu(live.takeoff_ms) + '</div></div>';
    h += '<div><div class="fd-label">' + (live.takeoff_ms ? 'In the air' : 'Since out') + '</div><div class="fd-val">' +
      (since ? hm((now - since) / 60000) : '-:--') + '</div></div>';
    h += '<div><div class="fd-label">ETA</div><div class="fd-val">' + (toGo != null ? zulu(now + toGo * 60000) : '--:--') + '</div></div>';
    h += '</div>';
    if (progress != null) {
      h += bar(progress);
      h += '<div class="fd-barnote">' + num(flown) + ' nm flown, ' + num(left) + ' nm to go' +
        (toGo != null ? ', ' + hm(toGo) + ' at this speed' : '') + '</div>';
    } else {
      h += '<div class="fd-barnote">' + num(flown) + ' nm flown</div>';
    }
    h += '</div></div>';

    if (t) {
      h += '<div class="fd-read">';
      h += readout('Altitude', num(t.alt_msl_ft), 'ft');
      h += readout('Ground speed', num(t.gs_kt), 'kt');
      h += readout('Vertical speed', signed(t.vs_fpm), 'fpm');
      h += readout('Heading', pad3(t.hdg_true_deg), 'true');
      h += '</div>';
    }
    if (st.last_landing) {
      var l = st.last_landing;
      h += '<div class="fd-card"><div class="fd-label">Touchdown</div><div class="fd-big">' + signed(-Math.abs(l.vs_fpm)) +
        ' <span class="fd-dim" style="font-size:16px">fpm</span> <span style="margin-left:18px">' + l.g_peak.toFixed(2) +
        '</span> <span class="fd-dim" style="font-size:16px">G</span></div>' +
        (l.bounces ? '<div class="fd-dim">' + l.bounces + (l.bounces === 1 ? ' bounce' : ' bounces') + '</div>' : '') + '</div>';
    }
    if (!st.demo) {
      if (ui.confirmEnd) {
        h += '<div class="fd-card"><p style="margin:0 0 10px">End this flight and save it to your logbook?</p>';
        h += '<button class="fd-btn fd-primary" data-act="end-yes">End and save</button>';
        h += '<button class="fd-btn" data-act="end-no">Keep flying</button></div>';
      } else {
        h += '<div class="fd-actions"><button class="fd-btn" data-act="end">End flight</button></div>';
      }
    }
    if (ui.actionError) h += '<div class="fd-err">' + esc(ui.actionError) + '</div>';
    return h;
  }

  function pad3(deg) {
    var d = Math.round(((deg % 360) + 360) % 360) || 360;
    return d < 10 ? '00' + d : d < 100 ? '0' + d : String(d);
  }

  function readout(label, value, unit) {
    return '<div><div class="fd-label">' + label + '</div><div class="fd-big">' + value +
      '<span class="fd-unit">' + unit + '</span></div></div>';
  }

  function lastScreen(d, err) {
    if (err) return '<div class="fd-msg"><p class="fd-dim">' + esc(err) + '</p></div>';
    if (d === undefined) return '<p class="fd-dim">Loading...</p>';
    if (!d) return '<div class="fd-msg">' + logo() + '<h2 class="fd-h">No flights yet</h2><p class="fd-sub">Your last flight shows up here once it is saved.</p></div>';
    var f = d.flight;
    var s = d.score;
    var h = '<h2 class="fd-h">' + esc(f.dep_icao || '----') + ' <span class="fd-dim" style="font-size:18px">to</span> ' + esc(f.arr_icao || '----') + '</h2>';
    var bits = [dateOf(f.start_ms)];
    if (f.icao_type) bits.push(esc(f.icao_type));
    if (f.callsign) bits.push(esc(f.callsign));
    bits.push(hm(f.total_min));
    if (f.distance_nm) bits.push(num(f.distance_nm) + ' nm');
    h += '<p class="fd-sub">' + bits.join(' &middot; ') + '</p>';

    // Landing.
    h += '<div class="fd-card"><div class="fd-label">Landing</div>';
    if (f.landing_rate_fpm != null) {
      h += '<div class="fd-big">' + signed(-Math.abs(f.landing_rate_fpm)) + ' <span class="fd-dim" style="font-size:16px">fpm</span>';
      if (f.landing_g != null) h += ' <span style="margin-left:18px">' + f.landing_g.toFixed(2) + '</span> <span class="fd-dim" style="font-size:16px">G</span>';
      h += '</div>';
      if (f.landing_grade) h += '<div class="fd-warn" style="font-weight:bold;margin-top:2px">' + esc(f.landing_grade) + '</div>';
    } else {
      h += '<div class="fd-dim">No landing was recorded.</div>';
    }
    h += '</div>';

    // Pilot score.
    h += '<div class="fd-card"><div class="fd-label" style="margin-bottom:10px">Pilot score</div>';
    if (s && s.performance != null) {
      var perf = Math.round(s.performance);
      h += '<div class="fd-score ' + tone(perf) + '"><b>' + perf + '</b><span>of 100</span></div><div class="fd-parts">';
      if (s.landing) h += part('Landing', Math.round(s.landing.score));
      if (s.safety) h += part('Flight stability', Math.round(s.safety.score));
      if (s.rating_before != null && s.rating_after != null) {
        var dr = Math.round(s.rating_after) - Math.round(s.rating_before);
        h += '<div class="fd-part">Rating <span class="fd-r">' + num(s.rating_after) + ' <span class="' +
          (dr > 0 ? 'fd-good' : dr < 0 ? 'fd-bad' : 'fd-dim') + '">(' + signed(dr) + ')</span></span></div>';
      }
      h += '<div class="fd-part">XP earned <span class="fd-r fd-warn">+' + num(s.xp) + '</span></div>';
      h += '</div><div class="fd-clear"></div>';
    } else if (s && s.unrated) {
      h += '<p class="fd-dim" style="margin:0">' + esc(s.unrated) + '</p>';
      if (s.xp) h += '<p style="margin:6px 0 0">XP earned <b class="fd-warn">+' + num(s.xp) + '</b></p>';
    } else {
      h += '<p class="fd-dim" style="margin:0">This flight has no score.</p>';
    }
    h += '</div>';
    if (d.standing) h += standingCard(d.standing);
    return h;
  }

  function part(label, score) {
    return '<div class="fd-part">' + label + ' <span class="fd-r ' + tone(score) + '">' + score + '</span>' +
      bar(score / 100) + '</div>';
  }

  function standingCard(s) {
    var h = '<div class="fd-card"><div class="fd-cols"><div><div class="fd-label">Rank</div><div class="fd-rank">' + esc(s.rank.title) + '</div></div>';
    h += '<div style="width:160px"><div class="fd-label">Rating</div><div class="fd-big">' + num(s.rating) + '</div></div>';
    h += '<div style="width:160px"><div class="fd-label">XP</div><div class="fd-big">' + num(s.xp) + '</div></div></div>';
    if (s.next_rank) {
      var span = s.next_rank.min_xp - s.rank.min_xp;
      h += bar(span > 0 ? (s.xp - s.rank.min_xp) / span : 1);
      h += '<div class="fd-barnote">' + num(Math.max(0, s.next_rank.min_xp - s.xp)) + ' XP to ' + esc(s.next_rank.title) + '</div>';
    } else {
      h += '<div class="fd-barnote">Top rank reached</div>';
    }
    h += '</div>';
    return h;
  }

  function pilotScreen(p, err) {
    if (err) return '<div class="fd-msg"><p class="fd-dim">' + esc(err) + '</p></div>';
    if (!p) return '<p class="fd-dim">Loading...</p>';
    var r = p.rating;
    var name = (p.pilot && p.pilot.name) || 'Pilot';
    var h = '<h2 class="fd-h">' + esc(name) + '</h2>';
    var sub = [];
    if (p.pilot && p.pilot.pilot_id) sub.push(esc(p.pilot.pilot_id));
    if (p.pilot && p.pilot.home_icao) sub.push('Home ' + esc(p.pilot.home_icao));
    h += '<p class="fd-sub">' + (sub.join(' &middot; ') || 'Your pilot profile') + '</p>';
    h += standingCard(r);
    h += '<div class="fd-read">';
    h += readout('Rated flights', num(r.rated_flights || 0), '');
    h += readout('Avg score', r.avg_performance != null ? num(r.avg_performance) : '-', '');
    h += readout('Avg landing', r.avg_landing != null ? num(r.avg_landing) : '-', '');
    h += readout('Avg stability', r.avg_safety != null ? num(r.avg_safety) : '-', '');
    h += '</div>';
    if (p.totals) {
      h += '<div class="fd-read">';
      h += readout('Flights', num(p.totals.flights || 0), '');
      h += readout('Hours', hm(p.totals.minutes || 0), '');
      h += readout('Distance', num(p.totals.distance_nm || 0), 'nm');
      h += readout('Landings', num(p.totals.landings || 0), '');
      h += '</div>';
    }
    if (r.rating_peak) h += '<p class="fd-dim" style="font-size:14px">Best rating so far ' + num(r.rating_peak) + '.</p>';
    return h;
  }

  function offlineScreen(why) {
    return '<div class="fd-msg">' + logo() + '<h2 class="fd-h">FlightDeck is not answering</h2>' +
      '<p class="fd-sub">Make sure FlightDeck is running on this PC. This page connects again by itself.</p>' +
      (why ? '<p class="fd-dim" style="font-size:13px">' + esc(why) + '</p>' : '') + '</div>';
  }

  // ------------------------------------------------------------------
  // The view
  // ------------------------------------------------------------------

  function createViewClass(sdk) {
    return class FlightDeckView extends sdk.DisplayComponent {
      constructor(props) {
        super(props);
        this.rootRef = sdk.FSComponent.createRef();
        this.pageKeyActions = new Map();
        this.client = new Client();
        this.tab = 'flight';
        this.status = null;
        this.online = null;
        this.offlineWhy = '';
        this.ui = { simbrief: null, confirmEnd: false, actionError: '', busy: false };
        this.last = undefined;
        this.lastErr = '';
        this.profile = null;
        this.profileErr = '';
        this.shown = '';
        this.timer = null;
        this.socket = null;
        this.socketTried = 0;
        this.visible = true;
        this.alive = true;
      }

      render() {
        var h = sdk.FSComponent.buildComponent;
        return h('div', { class: 'fd-root', ref: this.rootRef },
          h('div', { class: 'fd-top' },
            h('div', { class: 'fd-brand' }, 'FlightDeck'),
            h('div', { class: 'fd-seg' },
              h('button', { class: 'on' }, 'Flight'),
              h('button', {}, 'Last flight'),
              h('button', {}, 'Pilot')
            ),
            h('div', { class: 'fd-dot' })
          ),
          h('div', { class: 'fd-body' })
        );
      }

      onAfterRender() {
        var root = this.rootRef.instance;
        if (!root) return;
        var self = this;
        this.root = root;
        this.body = root.querySelector('.fd-body');
        this.dot = root.querySelector('.fd-dot');
        var brand = root.querySelector('.fd-brand');
        brand.innerHTML = logo() + 'FlightDeck';
        this.tabs = root.querySelectorAll('.fd-seg button');
        var names = ['flight', 'last', 'pilot'];
        for (var i = 0; i < this.tabs.length; i++) {
          this.tabs[i].setAttribute('data-tab', names[i]);
          this.tabs[i].addEventListener('click', function (e) {
            self.choose(e.currentTarget.getAttribute('data-tab'));
          });
        }
        // One listener for every button the screens draw.
        this.body.addEventListener('click', function (e) {
          var el = e.target;
          while (el && el !== self.body && !el.getAttribute('data-act')) el = el.parentNode;
          if (el && el !== self.body) self.act(el.getAttribute('data-act'));
        });
        this.poll();
        this.connectEvents();
      }

      choose(tab) {
        this.tab = tab;
        for (var i = 0; i < this.tabs.length; i++) {
          this.tabs[i].className = this.tabs[i].getAttribute('data-tab') === tab ? 'on' : '';
        }
        if (tab === 'last') this.loadLast();
        if (tab === 'pilot') this.loadProfile();
        this.body.scrollTop = 0;
        this.draw(true);
      }

      // Status: from the event socket when it is open, else every 2 s.
      poll() {
        var self = this;
        if (!this.alive) return;
        window.clearTimeout(this.timer);
        var socketOpen = this.socket && this.socket.readyState === 1;
        if (socketOpen && this.online) {
          this.timer = window.setTimeout(function () { self.poll(); }, RETRY_MS * 2);
          return;
        }
        this.client.call('get_status').then(function (st) {
          self.setOnline(true);
          self.gotStatus(st);
          if (!self.socket) self.connectEvents();
          self.timer = window.setTimeout(function () { self.poll(); }, POLL_MS);
        }, function (e) {
          self.setOnline(false, e && e.answered ? e.message : '');
          self.timer = window.setTimeout(function () { self.poll(); }, RETRY_MS);
        });
      }

      connectEvents() {
        var self = this;
        if (!this.client.token || typeof WebSocket === 'undefined' || this.socket) return;
        // At most one try every 10 s; polling carries on meanwhile.
        if (Date.now() - this.socketTried < 10000) return;
        this.socketTried = Date.now();
        var opened = false;
        var ws;
        try {
          ws = new WebSocket(SERVER.replace('http', 'ws') + '/api/events?token=' + encodeURIComponent(this.client.token));
        } catch (e) {
          return;
        }
        this.socket = ws;
        ws.onopen = function () {
          opened = true;
        };
        ws.onmessage = function (m) {
          var msg;
          try {
            msg = JSON.parse(m.data);
          } catch (e) {
            return;
          }
          if (msg.event === 'status') self.gotStatus(msg.payload);
          else if (msg.event === 'flight-saved') self.flightSaved();
          else if (msg.event === 'flights-changed') {
            self.last = self.tab === 'last' ? self.last : undefined;
            if (self.tab === 'last') self.loadLast();
            if (self.tab === 'pilot') self.loadProfile();
          }
        };
        ws.onclose = function () {
          if (self.socket === ws) self.socket = null;
          // Back to polling, which reconnects the socket once FlightDeck answers.
          if (self.alive && opened) self.poll();
        };
        ws.onerror = function () {
          try { ws.close(); } catch (e) { /* closing anyway */ }
        };
      }

      setOnline(on, why) {
        if (this.online === on && this.offlineWhy === (why || '')) return;
        this.online = on;
        this.offlineWhy = why || '';
        if (this.dot) this.dot.className = on ? 'fd-dot ok' : 'fd-dot';
        this.draw(true);
      }

      gotStatus(st) {
        if (!st) return;
        var wasLive = this.status && this.status.live;
        this.status = st;
        if (!st.live) this.ui.confirmEnd = false;
        if (wasLive !== !!st.live) this.ui.actionError = '';
        if (this.online !== true) this.setOnline(true);
        // Fetch the SimBrief plan once, when there is a flight to start.
        if (!st.live && !st.armed && !st.plan && this.ui.simbrief === null) this.loadSimbrief();
        this.draw(false);
      }

      flightSaved() {
        var self = this;
        this.last = undefined;
        this.profile = null;
        // Show the score of the flight that just ended.
        this.loadLast().then(function () {
          if (self.tab === 'flight') self.choose('last');
        });
      }

      loadSimbrief() {
        var self = this;
        this.ui.simbrief = 'loading';
        this.draw(true);
        this.client.call('simbrief_latest').then(function (plan) {
          self.ui.simbrief = plan ? { plan: plan } : { error: 'No SimBrief plan found.' };
          self.draw(true);
        }, function (e) {
          self.ui.simbrief = { error: (e && e.answered && e.message) || 'Could not load your SimBrief plan.' };
          self.draw(true);
        });
      }

      loadLast() {
        var self = this;
        return this.client.call('list_flights', { limit: 1 }).then(function (rows) {
          if (!rows || !rows.length) return null;
          return self.client.call('get_flight', { id: rows[0].id });
        }).then(function (d) {
          self.last = d || null;
          self.lastErr = '';
          self.draw(true);
        }, function (e) {
          self.lastErr = (e && e.answered && e.message) || 'Could not load your last flight.';
          self.draw(true);
        });
      }

      loadProfile() {
        var self = this;
        return this.client.call('get_pilot_profile').then(function (p) {
          self.profile = p;
          self.profileErr = '';
          self.draw(true);
        }, function (e) {
          self.profileErr = (e && e.answered && e.message) || 'Could not load your pilot profile.';
          self.draw(true);
        });
      }

      act(what) {
        var self = this;
        var ui = this.ui;
        if (ui.busy) return;
        if (what === 'simbrief') return this.loadSimbrief();
        if (what === 'end') {
          ui.confirmEnd = true;
          return this.draw(true);
        }
        if (what === 'end-no') {
          ui.confirmEnd = false;
          return this.draw(true);
        }
        var cmd = null;
        var args = {};
        if (what === 'end-yes') cmd = 'end_flight';
        else if (what === 'start-none') { cmd = 'start_flight'; args = { plan: null, aircraftId: null }; }
        else if (what === 'start-current') { cmd = 'start_flight'; args = { plan: this.status && this.status.plan, aircraftId: null }; }
        else if (what === 'start-simbrief') { cmd = 'start_flight'; args = { plan: ui.simbrief && ui.simbrief.plan, aircraftId: null }; }
        if (!cmd) return;
        ui.busy = true;
        ui.actionError = '';
        this.client.call(cmd, args).then(function () {
          ui.busy = false;
          ui.confirmEnd = false;
          return self.client.call('get_status').then(function (st) { self.gotStatus(st); });
        }).then(null, function (e) {
          ui.busy = false;
          ui.actionError = (e && e.message) || 'That did not work.';
          self.draw(true);
        });
      }

      /** Redraws the open screen when what it shows has changed. */
      draw(force) {
        if (!this.body || !this.visible) return;
        var html;
        if (this.online === false) html = offlineScreen(this.offlineWhy);
        else if (this.online === null) html = '<p class="fd-dim">Connecting to FlightDeck...</p>';
        else if (this.tab === 'flight') html = flightScreen(this.status, this.ui);
        else if (this.tab === 'last') html = lastScreen(this.last, this.lastErr);
        else html = pilotScreen(this.profile, this.profileErr);
        if (!force && html === this.shown) return;
        this.shown = html;
        var top = this.body.scrollTop;
        this.body.innerHTML = html;
        this.body.scrollTop = top;
      }

      // EFB lifecycle.
      onOpen() {
        this.visible = true;
        this.draw(true);
      }
      onResume() {
        this.visible = true;
        if (this.tab === 'last') this.loadLast();
        if (this.tab === 'pilot') this.loadProfile();
        this.draw(true);
      }
      onPause() {
        this.visible = false;
      }
      onClose() {
        this.visible = false;
      }
      onUpdate() {}
      routeGamepadInteractionEvent() {}
      handlePageKeyAction() {}

      destroy() {
        this.alive = false;
        window.clearTimeout(this.timer);
        if (this.socket) {
          try { this.socket.close(); } catch (e) { /* gone anyway */ }
        }
        super.destroy();
      }
    };
  }

  // ------------------------------------------------------------------
  // Registration with the EFB
  // ------------------------------------------------------------------

  function registerApp() {
    var sdk = getSdk();
    var efb = window.EFB_API;
    if (!sdk || !efb || typeof efb.use !== 'function') return false;

    injectStyle();
    var View = createViewClass(sdk);

    class FlightDeckApp {
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

    efb.use(FlightDeckApp);
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
