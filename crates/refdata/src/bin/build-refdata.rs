//! Rebuild the embedded reference data from OurAirports CSV files.
//!
//! Download airports.csv, countries.csv and regions.csv from
//! https://github.com/davidmegginson/ourairports-data into one folder, then:
//!
//!   cargo run -p refdata --features build-tool --bin build-refdata -- <csv-folder>
//!
//! It writes crates/refdata/data/{airports,countries,regions}.tsv.zst.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ").trim().to_string()
}

fn read(dir: &Path, name: &str) -> csv::Reader<std::fs::File> {
    csv::Reader::from_path(dir.join(name)).unwrap_or_else(|e| panic!("cannot read {name}: {e}"))
}

fn write(out: &Path, name: &str, body: &str) {
    let z = zstd::encode_all(body.as_bytes(), 19).expect("compress");
    std::fs::write(out.join(name), &z).expect("write");
    println!("{name}: {} KB ({} KB raw)", z.len() / 1024, body.len() / 1024);
}

fn main() {
    let src = PathBuf::from(std::env::args().nth(1).expect("usage: build-refdata <ourairports-csv-folder>"));
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    std::fs::create_dir_all(&out).unwrap();

    let mut rdr = read(&src, "airports.csv");
    let headers = rdr.headers().unwrap().clone();
    let col: HashMap<&str, usize> = headers.iter().enumerate().map(|(i, h)| (h, i)).collect();
    let mut body = String::from("ident\ttype\tname\tlat\tlon\televation_ft\tcountry\tregion\n");
    let mut n = 0;
    for rec in rdr.records() {
        let r = rec.unwrap();
        let get = |k: &str| r.get(col[k]).unwrap_or("");
        let kind = match get("type") {
            "large_airport" => 'L',
            "medium_airport" => 'M',
            "small_airport" => 'S',
            "seaplane_base" => 'W',
            "heliport" => 'H',
            _ => continue, // closed, balloonport
        };
        let icao = get("icao_code");
        let gps = get("gps_code");
        let ident = if !icao.is_empty() {
            icao
        } else if gps.len() == 4 {
            gps
        } else {
            get("ident")
        };
        let (Ok(lat), Ok(lon)) = (get("latitude_deg").parse::<f64>(), get("longitude_deg").parse::<f64>()) else {
            continue;
        };
        body.push_str(&format!(
            "{}\t{}\t{}\t{:.5}\t{:.5}\t{}\t{}\t{}\n",
            clean(ident),
            kind,
            clean(get("name")),
            lat,
            lon,
            get("elevation_ft").parse::<f64>().map(|e| e.round() as i64).unwrap_or(0),
            get("iso_country"),
            get("iso_region"),
        ));
        n += 1;
    }
    println!("{n} airports");
    write(&out, "airports.tsv.zst", &body);

    for (file, code_col, out_name) in [("countries.csv", "code", "countries.tsv.zst"), ("regions.csv", "code", "regions.tsv.zst")] {
        let mut rdr = read(&src, file);
        let headers = rdr.headers().unwrap().clone();
        let ci = headers.iter().position(|h| h == code_col).unwrap();
        let ni = headers.iter().position(|h| h == "name").unwrap();
        let mut body = String::from("code\tname\n");
        for rec in rdr.records() {
            let r = rec.unwrap();
            body.push_str(&format!("{}\t{}\n", clean(&r[ci]), clean(&r[ni])));
        }
        write(&out, out_name, &body);
    }
}
