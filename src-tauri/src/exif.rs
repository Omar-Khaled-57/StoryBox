use std::path::Path;

/// Extract date taken, latitude, and longitude from EXIF metadata.
pub fn extract_exif(path: &Path) -> (Option<chrono::DateTime<chrono::Utc>>, Option<f64>, Option<f64>) {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (None, None, None),
    };
    let mut bufreader = std::io::BufReader::new(file);
    let exif_reader = match exif::Reader::new().read_from_container(&mut bufreader) {
        Ok(r) => r,
        Err(_) => return (None, None, None),
    };

    let date_taken = exif_reader.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .or_else(|| exif_reader.get_field(exif::Tag::DateTimeDigitized, exif::In::PRIMARY))
        .or_else(|| exif_reader.get_field(exif::Tag::DateTime, exif::In::PRIMARY))
        .and_then(|f| value_to_string(&f.value))
        .and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S"))
                .ok()
                .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
        });

    let lat = exif_reader.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)
        .and_then(|f| parse_gps(&f.value));
    let lat_ref = exif_reader.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)
        .and_then(|f| value_to_string(&f.value));
    let lat = lat.map(|v| if lat_ref.as_deref().map_or(true, |r| r == "N") { v } else { -v });

    let lon = exif_reader.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)
        .and_then(|f| parse_gps(&f.value));
    let lon_ref = exif_reader.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)
        .and_then(|f| value_to_string(&f.value));
    let lon = lon.map(|v| if lon_ref.as_deref().map_or(true, |r| r == "E") { v } else { -v });

    (date_taken, lat, lon)
}

fn value_to_string(value: &exif::Value) -> Option<String> {
    match value {
        exif::Value::Ascii(v) => String::from_utf8(v.concat()).ok(),
        _ => None,
    }
}

fn parse_gps(value: &exif::Value) -> Option<f64> {
    match value {
        exif::Value::Rational(rationals) if !rationals.is_empty() => {
            let mut total = 0.0;
            for (i, r) in rationals.iter().enumerate() {
                let val = r.to_f64();
                let divisor = match i {
                    0 => 1.0,
                    1 => 60.0,
                    _ => 3600.0,
                };
                total += val / divisor;
            }
            Some(total)
        }
        _ => None,
    }
}
