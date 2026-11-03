use super::*;

fn landing(t_ms: i64, vs: f64, g: f64) -> LandingReport {
    let (hdg, lat, lon) = (180.0, 47.45, -122.308);
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
    h.pitch_up_s[0] = h.airborne_s;
    h.pitch_down_s[0] = h.airborne_s;
    h.bank_5[0] = h.airborne_s;
    h.pitch_up_5[0] = h.airborne_s;
    h.pitch_down_5[0] = h.airborne_s;
    // G, bank and pitch only; the other parts have their own tests.
    h.roll_rate_s.clear();
    h.pitch_rate_s.clear();
    h.g_rate_s.clear();
    h.speed_over_s.clear();
    h
}

/// Move `secs` of a flight from level into the 5-degree bin holding `deg`.
fn steep(v: &mut [f64], deg: f64, secs: f64) {
    v[0] -= secs;
    v[(deg / 5.0) as usize] += secs;
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
        takeoff_ms: Some(id * 86_400_000 + 600_000),
        landing_ms: Some(id * 86_400_000 + 600_000 + 60 * 60_000),
        total_min: 60,
        landings: vec![landing(id * 86_400_000 + 3_000_000, vs, 1.1)],
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
fn a_good_landing() {
    let l = score_landing(&[landing(0, -120.0, 1.15)]).unwrap();
    assert!(l.score > 95.0, "{}", l.score);
    // Touchdown rate, G and bounces share the score: 50 : 25 : 25.
    assert_eq!(l.rate.weight, 0.5);
    assert_eq!(l.g.as_ref().unwrap().weight, 0.25);
}

#[test]
fn crosswind_lifts_each_part_of_the_landing() {
    let mut l = landing(0, -330.0, 1.35);
    let calm = score_landing(&[l.clone()]).unwrap();
    l.crosswind_kt = -12.0;
    let windy = score_landing(&[l.clone()]).unwrap();
    assert!((windy.crosswind_bonus - 0.12).abs() < 1e-9);
    // -330 fpm scores 65 before the bonus; 12% more is 73.
    assert_eq!((calm.rate.score, windy.rate.score), (65.0, 73.0));
    assert!(windy.g.as_ref().unwrap().score > calm.g.as_ref().unwrap().score);
    // No bounces already scores 100: it stays at 100.
    assert_eq!(windy.bounce.as_ref().unwrap().score, 100.0);
    assert!((windy.base_score - calm.score).abs() < 1e-9);
    assert!(windy.score > calm.score);
    // Capped at 25 kt.
    l.crosswind_kt = 40.0;
    assert!((score_landing(&[l]).unwrap().crosswind_bonus - 0.25).abs() < 1e-9);
}

#[test]
fn a_hard_landing_scores_low() {
    let l = score_landing(&[landing(0, -650.0, 1.9)]).unwrap();
    // Only the lack of bounces holds it up.
    assert!(l.score < 45.0, "{}", l.score);
}

#[test]
fn go_arounds_are_discarded_and_close_ones_are_bounces() {
    let mut first = landing(0, -900.0, 2.4);
    first.touch_and_go = true;
    let last = landing(300_000, -100.0, 1.1);
    let s = score_landing(&[first.clone(), last.clone()]).unwrap();
    assert_eq!(s.discarded, 1);
    assert_eq!(s.vs_fpm, -100.0);
    // Back on the ground 20 s later: one landing that bounced.
    let mut again = last.clone();
    again.t_ms = first.t_ms + 20_000;
    let s = score_landing(&[first, again]).unwrap();
    assert_eq!((s.discarded, s.bounces, s.vs_fpm), (0, 1, -900.0));
}

#[test]
fn pitch_limits_are_20_up_and_10_down() {
    // Two minutes at 18 degrees nose up and 8 down: no penalty.
    let mut h = smooth_handling(60.0);
    steep(&mut h.pitch_up_5, 18.0, 120.0);
    steep(&mut h.pitch_down_5, 8.0, 120.0);
    assert_eq!(score_safety(&h, false).unwrap().pitch.unwrap().score, 100.0);
    // Two minutes at 22 degrees up costs a little; 12 down about as much.
    let mut up = smooth_handling(60.0);
    steep(&mut up.pitch_up_5, 22.0, 120.0);
    let up = score_safety(&up, false).unwrap().pitch.unwrap().score;
    assert!(up < 100.0 && up > 85.0, "{up}");
    let mut down = smooth_handling(60.0);
    steep(&mut down.pitch_down_5, 12.0, 120.0);
    assert_eq!(score_safety(&down, false).unwrap().pitch.unwrap().score, up);
    // Two minutes at 32 degrees nose down is punished hard.
    let mut dive = smooth_handling(60.0);
    steep(&mut dive.pitch_down_5, 32.0, 120.0);
    assert!(score_safety(&dive, false).unwrap().pitch.unwrap().score < 40.0);
}

#[test]
fn bank_past_30_loses_points_and_more_the_steeper() {
    let mut h = smooth_handling(60.0);
    steep(&mut h.bank_5, 28.0, 120.0);
    assert_eq!(score_safety(&h, false).unwrap().bank.unwrap().score, 100.0);
    let at = |deg: f64| {
        let mut h = smooth_handling(60.0);
        steep(&mut h.bank_5, deg, 60.0);
        score_safety(&h, false).unwrap().bank.unwrap().score
    };
    // A minute at each: 32, 37, 47, 62 degrees.
    let (a, b, c, d) = (at(32.0), at(37.0), at(47.0), at(62.0));
    assert!(a < 100.0 && a > 90.0, "{a}");
    // A minute 7 degrees past costs about a quarter; 17 or more, everything.
    assert!(b < a && b > 60.0 && c == 0.0 && d == 0.0, "{a} {b} {c} {d}");
}

#[test]
fn without_g_bank_and_pitch_are_all_of_stability() {
    let mut h = smooth_handling(60.0);
    h.g_s.clear();
    steep(&mut h.bank_5, 37.0, 60.0);
    let s = score_safety(&h, false).unwrap();
    assert!(s.g.is_none());
    assert_eq!((s.bank.as_ref().unwrap().weight, s.pitch.as_ref().unwrap().weight), (0.5, 0.5));
    assert!((s.score - (s.bank.unwrap().score + 100.0) / 2.0).abs() < 1e-9);
}

#[test]
fn older_recordings_still_score() {
    let mut old = smooth_handling(60.0);
    old.bank_5.clear();
    old.pitch_up_5.clear();
    old.pitch_down_5.clear();
    assert_eq!(score_safety(&old, false).unwrap().score, 100.0);
    // Two minutes in the old 45-60 degree band.
    old.bank_s[0] -= 120.0;
    old.bank_s[2] += 120.0;
    assert!(score_safety(&old, false).unwrap().bank.unwrap().score < 30.0);
}

#[test]
fn the_length_bonus_counts_only_wheels_up_to_touchdown() {
    // Four hours on the ground with the engines running, 20 minutes flying.
    let mut f = flight(1, -150.0, None);
    f.total_min = 260;
    f.takeoff_ms = None;
    assert_eq!(airborne_minutes(&f), 0.0, "no takeoff, no bonus");
    f.takeoff_ms = Some(1_000_000);
    f.landing_ms = Some(1_000_000 + 20 * 60_000);
    assert_eq!(airborne_minutes(&f), 20.0);
    // Recorded flights count their airborne time without pauses.
    f.handling = Some(smooth_handling(18.0));
    assert_eq!(airborne_minutes(&f), 18.0);
    assert_eq!(length_bonus(airborne_minutes(&f)), 2.0);
}

#[test]
fn safety_penalises_sustained_steep_flying() {
    let calm = score_safety(&smooth_handling(60.0), false).unwrap();
    assert_eq!(calm.score, 100.0);
    let mut h = smooth_handling(60.0);
    // Two minutes at 80 degrees of bank.
    steep(&mut h.bank_5, 80.0, 120.0);
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
    assert!(score_safety(&h, false).unwrap().score >= 99.0);
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
    let p = profile(&[manual, sparse, rated, crashed]);
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
    assert_eq!((r.xp_bonus, r.k), (8.0, Some(48.0)));
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
    let p = profile(&[f]);
    let s = &p.flights[0];
    assert!(s.performance.is_some(), "{:?}", s.unrated);
    let l = s.landing.as_ref().unwrap();
    assert!(l.bounce.is_none() && l.g.is_some());
    let sf = s.safety.as_ref().unwrap();
    assert!(sf.g.is_none() && (sf.bank.as_ref().unwrap().weight - 0.5).abs() < 1e-9);
    // Counted in full, like any other flight.
    assert_eq!(s.k, Some(48.0));

    // No landing report: the touchdown worked out from the positions counts.
    let mut g = flight(2, -180.0, Some(smooth_handling(60.0)));
    let mut est = g.landings.remove(0);
    est.vs_source = "track".into();
    est.g_peak = 0.0;
    g.estimated_landing = Some(est);
    let p = profile(&[g.clone()]);
    assert_eq!(p.flights[0].landing.as_ref().unwrap().source, "track");
    assert_eq!(p.flights[0].k, Some(48.0));
    // Nothing at all: not rated, but it still earns XP.
    g.estimated_landing = None;
    let p = profile(&[g]);
    assert_eq!(p.flights[0].unrated.as_deref(), Some("Not enough data: no touchdown rate"));
    assert_eq!(p.flights[0].xp_base, UNSCORED_XP);
}

fn with_rates(minutes: f64) -> Handling {
    let mut h = smooth_handling(minutes);
    h.roll_rate_s = vec![0.0; tracker::handling::RATE_BINS];
    h.pitch_rate_s = vec![0.0; tracker::handling::RATE_BINS];
    h.g_rate_s = vec![0.0; tracker::handling::RATE_BINS];
    h.roll_rate_s[2] = h.airborne_s;
    h.pitch_rate_s[1] = h.airborne_s;
    h.g_rate_s[1] = h.airborne_s;
    h
}

#[test]
fn smoothness_costs_quick_jerky_inputs() {
    let calm = score_safety(&with_rates(60.0), false).unwrap();
    assert_eq!(calm.smoothness.as_ref().unwrap().score, 100.0);
    // Twenty seconds of rolling at 14 degrees a second.
    let mut h = with_rates(60.0);
    h.roll_rate_s[2] -= 20.0;
    h.roll_rate_s[14] += 20.0;
    let s = score_safety(&h, false).unwrap().smoothness.unwrap().score;
    assert!(s < 60.0 && s > 30.0, "{s}");
    // Aerobatic aircraft are not judged on it.
    assert!(score_safety(&h, true).unwrap().smoothness.is_none());
}

fn approach(points: &[(f32, f32, f32, f32, f32)]) -> Approach {
    Approach {
        points: points
            .iter()
            .map(|&(t_s, agl_ft, vs_fpm, bank, ias)| tracker::ApproachPoint {
                t_s,
                agl_ft,
                vs_fpm,
                bank_deg: Some(bank),
                ias_kt: Some(ias),
            })
            .collect(),
    }
}

#[test]
fn a_stable_approach_scores_100_and_an_unstable_one_loses() {
    // A minute and a half from 1,000 ft at 700 fpm, wings level, 130 kt.
    let stable: Vec<_> = (0..90).map(|i| (i as f32 - 90.0, 1000.0 - i as f32 * 11.0, -700.0, 2.0, 130.0)).collect();
    let mut h = smooth_handling(60.0);
    h.approach = Some(approach(&stable));
    assert_eq!(score_safety(&h, false).unwrap().approach.unwrap().score, 100.0);
    // Diving at 1,500 fpm for 20 s, turning 25 degrees low down, 20 kt fast.
    let mut rough = stable.clone();
    for p in rough.iter_mut().take(20) {
        p.2 = -1500.0;
    }
    for p in rough.iter_mut().skip(60).take(10) {
        p.3 = 25.0;
        p.4 = 150.0;
    }
    h.approach = Some(approach(&rough));
    let s = score_safety(&h, false).unwrap();
    assert!(s.approach.as_ref().unwrap().score < 40.0, "{s:?}");
    assert_eq!((s.approach_sink_max, s.approach_bank_max, s.approach_speed_dev), (1500.0, 25.0, 20.0));
}

fn real_approach(json: &str) -> Approach {
    serde_json::from_str(json).unwrap()
}

#[test]
fn a_steady_slow_down_on_final_is_fine() {
    // A Comanche into Concrete (K3W5): level at 88 kt about 500 ft above the
    // valley, then a descent slowing steadily to 59 kt by 50 ft. The old
    // rule took 88 kt as the typical speed and scored this approach 0.
    let mut h = smooth_handling(60.0);
    h.approach = Some(real_approach(include_str!("testdata/approach_86.json")));
    let s = score_safety(&h, false).unwrap();
    let a = s.approach.as_ref().unwrap().score;
    eprintln!("K3W5 approach {a}, speed off by up to {:.1} kt", s.approach_speed_dev);
    assert!(a >= 85.0, "{s:?}");
    // A steady descent at 81 kt (into Omak): only a moment at 1,003 fpm.
    h.approach = Some(real_approach(include_str!("testdata/approach_85.json")));
    assert_eq!(score_safety(&h, false).unwrap().approach.unwrap().score, 99.0);
}

#[test]
fn speed_alone_cannot_zero_the_approach() {
    let wild: Vec<_> = (0..90)
        .map(|i| (i as f32 - 90.0, 1000.0 - i as f32 * 11.0, -700.0, 2.0, if i % 10 < 5 { 100.0 } else { 160.0 }))
        .collect();
    let mut h = smooth_handling(60.0);
    h.approach = Some(approach(&wild));
    let a = score_safety(&h, false).unwrap().approach.unwrap().score;
    assert_eq!(a, 50.0);
}

#[test]
fn slowing_far_too_much_costs_a_little() {
    // 130 kt down to 80 kt from 500 ft: well past the 25 kt allowed.
    let pts: Vec<_> = (0..90)
        .map(|i| {
            let agl = 1000.0 - i as f32 * 11.0;
            let ias = if agl > 500.0 { 130.0 } else { 130.0 - (500.0 - agl) / 450.0 * 50.0 };
            (i as f32 - 90.0, agl, -700.0, 2.0, ias)
        })
        .collect();
    let mut h = smooth_handling(60.0);
    h.approach = Some(approach(&pts));
    let a = score_safety(&h, false).unwrap().approach.unwrap().score;
    assert!((60.0..=85.0).contains(&a), "{a}");
}

#[test]
fn the_speed_limit_costs_per_second_over() {
    let mut h = smooth_handling(60.0);
    h.speed_over_s = vec![0.0; tracker::handling::SPEED_BINS];
    assert_eq!(score_safety(&h, false).unwrap().speed.unwrap().score, 100.0);
    // Ten seconds 10-15 kt over: 0.008 x 12.5^2 x 10 = 12.5 points.
    h.speed_over_s[2] = 10.0;
    assert_eq!(score_safety(&h, false).unwrap().speed.unwrap().score, 88.0);
}

#[test]
fn every_score_is_a_whole_number() {
    let mut h = smooth_handling(60.0);
    steep(&mut h.bank_5, 32.0, 7.0);
    let s = score_safety(&h, false).unwrap();
    // A tiny deduction still shows as 99.
    assert_eq!(s.bank.as_ref().unwrap().score, 99.0);
    assert_eq!(s.score.fract(), 0.0);
    let p = profile(&[flight(1, -233.0, Some(h))]);
    assert_eq!(p.flights[0].performance.unwrap().fract(), 0.0);
}
