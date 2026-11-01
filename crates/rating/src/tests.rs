use std::sync::OnceLock;

use super::*;

fn data() -> &'static RefData {
    static D: OnceLock<RefData> = OnceLock::new();
    D.get_or_init(RefData::load)
}

/// A touchdown on KSEA 16L, `off_ft` right of the centerline.
fn landing(t_ms: i64, vs: f64, g: f64, off_ft: f64) -> LandingReport {
    // 16L runs almost due south along longitude -122.3080.
    let hdg = 180.0;
    let lat: f64 = 47.4500;
    let lon = -122.30800 - off_ft / (60.0 * 6076.12 * lat.to_radians().cos());
    LandingReport {
        t_ms,
        lat,
        lon,
        vs_fpm: vs,
        vs_source: "sim".into(),
        g_peak: g,
        pitch_deg: 3.0,
        bank_deg: 0.0,
        gs_kt: 130.0,
        ias_kt: 130.0,
        hdg_deg: hdg,
        headwind_kt: 10.0,
        crosswind_kt: 0.0,
        bounces: 0,
        touch_and_go: false,
    }
}

fn smooth_handling(minutes: f64) -> Handling {
    let mut h = Handling::default();
    h.airborne_s = minutes * 60.0;
    h.g_s[4] = h.airborne_s;
    h.bank_s[0] = h.airborne_s;
    h.pitch_s[0] = h.airborne_s;
    h
}

fn flight(id: i64, vs: f64, h: Option<Handling>) -> RatingFlight {
    RatingFlight {
        id,
        start_ms: id * 86_400_000,
        sim: "msfs2024".into(),
        source: String::new(),
        status: "complete".into(),
        icao_type: "A320".into(),
        title: "Airbus A320neo".into(),
        dep_icao: "KPDX".into(),
        arr_icao: "KSEA".into(),
        takeoff_ms: None,
        landing_ms: None,
        total_min: 60,
        landings: vec![landing(id * 86_400_000 + 3_000_000, vs, 1.1, 0.0)],
        estimated_landing: None,
        handling: h,
    }
}

#[test]
fn curves_are_straight_lines_between_points() {
    assert_eq!(curve(0.0, scales::RATE), 100.0);
    assert_eq!(curve(120.0, scales::RATE), 95.0);
    assert_eq!(curve(5000.0, scales::RATE), 0.0);
}

#[test]
fn the_documents_rating_examples() {
    // Rating 1,000 expects 75; K 32; a score of 88 adds 4.16.
    assert_eq!(expected_for(1000.0), 75.0);
    assert!((32.0 * (88.0 - expected_for(1000.0)) / 100.0 - 4.16).abs() < 1e-9);
    assert!((32.0_f64 * (60.0 - 75.0) / 100.0 + 4.8).abs() < 1e-9);
    // Pilot A: 750 expects 70, K 40 (flights 6 to 20): +8.
    assert_eq!(expected_for(750.0), 70.0);
    assert_eq!(k_for(5), 40.0);
    assert!((k_for(5) * (90.0 - 70.0) / 100.0 - 8.0).abs() < 1e-9);
    assert_eq!(expected_for(1750.0), 90.0);
    assert_eq!(expected_for(2400.0), 95.0);
    assert_eq!(expected_for(100.0), 50.0);
    assert_eq!((k_for(0), k_for(4), k_for(20), k_for(50)), (48.0, 48.0, 32.0, 24.0));
}

#[test]
fn length_bonus_steps() {
    assert_eq!(length_bonus(10.0), 0.0);
    assert_eq!(length_bonus(15.0), 2.0);
    assert_eq!(length_bonus(119.0), 8.0);
    assert_eq!(length_bonus(120.0), 12.0);
    assert_eq!(length_bonus(600.0), 20.0);
}

#[test]
fn ranks_follow_xp() {
    assert_eq!(rank_for(0.0).0.title, "Student Pilot");
    assert_eq!(rank_for(499.0).1.unwrap().title, "Solo Pilot");
    assert_eq!(rank_for(500.0).0.insignia.stripes, 1);
    assert_eq!(rank_for(12_500.0).0.insignia.wings, Some("gold"));
    let (top, next) = rank_for(40_000.0);
    assert!(top.insignia.oak_leaf && next.is_none());
}

#[test]
fn a_good_landing_on_the_centerline() {
    let l = score_landing(&[landing(0, -120.0, 1.15, 2.0)], data()).unwrap();
    let rwy = l.runway.as_ref().expect("on KSEA 16L");
    assert_eq!(rwy.runway, "16L");
    assert!(l.centerline.as_ref().unwrap().score > 95.0, "{l:?}");
    assert!(l.score > 95.0, "{}", l.score);
}

#[test]
fn hard_and_off_center_scores_low() {
    let l = score_landing(&[landing(0, -650.0, 1.9, 60.0)], data()).unwrap();
    // Only the lack of bounces holds it up.
    assert!(l.score < 40.0, "{}", l.score);
}

#[test]
fn no_runway_spreads_the_weights() {
    let mut l = landing(0, -120.0, 1.15, 0.0);
    // A field in the middle of nowhere.
    l.lat = 45.0;
    l.lon = -100.0;
    let s = score_landing(&[l], data()).unwrap();
    assert!(s.centerline.is_none());
    let w = s.rate.weight + s.g.as_ref().unwrap().weight + s.bounce.as_ref().unwrap().weight;
    assert!((w - 1.0).abs() < 1e-9);
    assert!((s.rate.weight - 0.35 / 0.75).abs() < 1e-9);
}

#[test]
fn crosswind_rewards_the_centerline_only() {
    let mut calm = landing(0, -120.0, 1.15, 25.0);
    let base = score_landing(&[calm.clone()], data()).unwrap();
    calm.crosswind_kt = 20.0;
    calm.headwind_kt = 0.0;
    let windy = score_landing(&[calm], data()).unwrap();
    assert!((windy.crosswind_kt - 20.0).abs() < 1.5, "{}", windy.crosswind_kt);
    let (a, b) = (base.centerline.unwrap().score, windy.centerline.unwrap().score);
    assert!(b > a && b <= 100.0, "{a} {b}");
    assert_eq!(base.rate, windy.rate);
}

#[test]
fn go_arounds_are_discarded_and_close_ones_are_bounces() {
    let mut first = landing(0, -900.0, 2.4, 50.0);
    first.touch_and_go = true;
    let last = landing(300_000, -100.0, 1.1, 0.0);
    let s = score_landing(&[first.clone(), last.clone()], data()).unwrap();
    assert_eq!(s.discarded, 1);
    assert_eq!(s.vs_fpm, -100.0);
    // Back on the ground 20 s later: one landing that bounced.
    let mut again = last.clone();
    again.t_ms = first.t_ms + 20_000;
    let s = score_landing(&[first, again], data()).unwrap();
    assert_eq!((s.discarded, s.bounces, s.vs_fpm), (0, 1, -900.0));
}

#[test]
fn safety_penalises_sustained_steep_flying() {
    let calm = score_safety(&smooth_handling(60.0), false).unwrap();
    assert_eq!(calm.score, 100.0);
    let mut h = smooth_handling(60.0);
    // Two minutes beyond 75 degrees of bank.
    h.bank_s[0] -= 120.0;
    h.bank_s[4] += 120.0;
    let steep = score_safety(&h, false).unwrap();
    assert!(steep.bank.as_ref().unwrap().score < 60.0, "{steep:?}");
    // The same in an Extra 330: no bank penalty, G carries it all.
    let aero = score_safety(&h, true).unwrap();
    assert!(aero.bank.is_none() && aero.score == 100.0 && aero.g.as_ref().unwrap().weight == 1.0);
}

#[test]
fn turbulence_costs_little() {
    let mut h = smooth_handling(120.0);
    // Ten scattered seconds above 1.6 G.
    h.g_s[4] -= 10.0;
    h.g_s[5] += 10.0;
    assert!(score_safety(&h, false).unwrap().score > 99.0);
}

#[test]
fn aerobatic_aircraft_are_recognised() {
    assert!(is_aerobatic("E330", ""));
    assert!(is_aerobatic("", "Pitts Special S-2S"));
    assert!(is_aerobatic("ZZZZ", "Boeing F/A-18E Super Hornet"));
    assert!(!is_aerobatic("C172", "Cessna Skyhawk"));
}

#[test]
fn the_profile_rates_only_what_it_can() {
    let mut manual = flight(1, -150.0, None);
    manual.sim = "manual".into();
    // Recorded before handling was kept, and its positions were too sparse.
    let sparse = flight(2, -150.0, Some(Handling::none()));
    let rated = flight(3, -100.0, Some(smooth_handling(90.0)));
    let mut crashed = flight(4, -100.0, Some(smooth_handling(30.0)));
    crashed.status = "partial".into();
    let p = profile(&[manual, sparse, rated, crashed], data());
    assert_eq!(p.rated_flights, 1);
    assert_eq!(p.flights.len(), 4);
    // Newest first.
    assert_eq!(p.flights[0].flight_id, 4);
    assert_eq!(p.flights[0].unrated.as_deref(), Some("Ended before landing"));
    assert_eq!(p.flights[0].xp, 0.0);
    assert_eq!(p.flights[3].unrated.as_deref(), Some("Entered by hand"));
    assert_eq!(p.flights[2].unrated.as_deref(), Some("Not enough data: too few positions in the air"));
    // Unrated flights still count towards XP and rank.
    assert_eq!(p.flights[3].xp, UNSCORED_XP + 8.0);
    let r = &p.flights[1];
    let perf = r.performance.unwrap();
    assert!(perf > 95.0, "{perf}");
    assert_eq!((r.confidence, r.xp_bonus, r.k), (1.0, 8.0, Some(48.0)));
    assert!(r.estimated.is_empty(), "{:?}", r.estimated);
    assert!((p.rating - (1000.0 + 48.0 * (perf - 75.0) / 100.0)).abs() < 1e-9);
    assert_eq!(p.history.len(), 2);
    assert_eq!(p.xp, 2.0 * (UNSCORED_XP + 8.0) + perf.round() + 8.0);
}

#[test]
fn imported_flights_are_rated_with_what_they_have() {
    // A Volanta flight: touchdown rate and G, no bounces; bank and pitch
    // worked out from its track, no G in flight.
    let mut f = flight(1, -180.0, None);
    f.source = "volanta".into();
    f.landings[0].vs_source = "volanta".into();
    let mut h = smooth_handling(60.0);
    h.source = tracker::handling_source::TRACK.into();
    h.g_s.clear();
    f.handling = Some(h);
    let p = profile(&[f], data());
    let s = &p.flights[0];
    assert!(s.performance.is_some(), "{:?}", s.unrated);
    let l = s.landing.as_ref().unwrap();
    assert!(l.bounce.is_none() && l.g.is_some());
    let sf = s.safety.as_ref().unwrap();
    assert!(sf.g.is_none() && (sf.bank.as_ref().unwrap().weight - 0.5).abs() < 1e-9);
    // Worked out from the track: half the weight on the rating.
    assert_eq!((s.confidence, s.k), (0.5, Some(24.0)));
    assert!(s.estimated.iter().any(|e| e.contains("rate of turn")));
    assert!(s.estimated.iter().any(|e| e.contains("bounces")));

    // No landing report: the touchdown worked out from the positions counts.
    let mut g = flight(2, -180.0, Some(smooth_handling(60.0)));
    let mut est = g.landings.remove(0);
    est.vs_source = "track".into();
    est.g_peak = 0.0;
    g.estimated_landing = Some(est);
    let p = profile(&[g.clone()], data());
    assert_eq!(p.flights[0].landing.as_ref().unwrap().source, "track");
    assert_eq!(p.flights[0].confidence, 0.5);
    // Nothing at all: not rated, but it still earns XP.
    g.estimated_landing = None;
    let p = profile(&[g], data());
    assert_eq!(p.flights[0].unrated.as_deref(), Some("Not enough data: no touchdown rate"));
    assert_eq!(p.flights[0].xp_base, UNSCORED_XP);
}
