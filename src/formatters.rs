use eframe::egui;
use crate::models::{StintData, StintTireSet, TireCornerData};

pub fn convert_unit(val: &str, to_metric: bool) -> String {
    let clean = val.trim();
    if clean.is_empty() || clean == "—" || clean == "..." { return clean.to_string(); }

    // Split numeric prefix and unit suffix (handles "1.65 km", "1.65km", "75 F", "75°F", "24.5 C", "24.5", etc.)
    let mut split_idx = 0;
    let mut has_digit = false;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            has_digit = true;
            split_idx = i + c.len_utf8();
        } else if has_digit && !c.is_whitespace() {
            break;
        }
    }

    if !has_digit {
        return clean.to_string();
    }

    let num_str = clean[..split_idx].trim();
    let unit_str = clean[split_idx..].trim().to_uppercase();

    let num = match num_str.parse::<f32>() {
        Ok(n) => n,
        Err(_) => return clean.to_string(),
    };

    match unit_str.as_str() {
        "C" | "°C" | "DEG C" | "DEGC" | "DEG" | "°" | "" => {
            if to_metric { format!("{:.0}°C", num) }
            else { format!("{:.0}°F", num * 1.8 + 32.0) }
        },
        "F" | "°F" | "DEG F" | "DEGF" => {
            if to_metric { format!("{:.0}°C", (num - 32.0) / 1.8) }
            else { format!("{:.0}°F", num) }
        },
        "M/S" | "M\\S" | "MPS" => {
            if to_metric { format!("{:.0} km/h", num * 3.6) }
            else { format!("{:.0} mph", num * 2.23694) }
        },
        "KPH" | "KM/H" | "KMH" => {
            if to_metric { format!("{:.0} km/h", num) }
            else { format!("{:.0} mph", num * 0.621371) }
        },
        "MPH" => {
            if to_metric { format!("{:.0} km/h", num * 1.60934) }
            else { format!("{:.0} mph", num) }
        },
        "KM" | "KMS" | "KILOMETERS" | "KILOMETRE" | "KILOMETER" => {
            if to_metric { format!("{:.2} km", num) }
            else { format!("{:.2} mi", num * 0.621371) }
        },
        "MI" | "MI." | "MILE" | "MILES" => {
            if to_metric { format!("{:.2} km", num * 1.60934) }
            else { format!("{:.2} mi", num) }
        },
        "M" | "METERS" | "METRES" => {
            if to_metric { format!("{:.0} m", num) }
            else { format!("{:.0} ft", num * 3.28084) }
        },
        "%" | "PCT" | "PERCENT" => {
            let pct_val = if num <= 1.0 && num > 0.0 { num * 100.0 } else { num };
            format!("{:.0}%", pct_val)
        },
        _ => clean.to_string()
    }
}

pub fn format_wind_dir(raw: &str) -> String {
    let clean = raw.trim();
    if clean.is_empty() || clean == "..." || clean == "—" {
        return String::new();
    }
    let upper = clean.to_uppercase();
    if matches!(
        upper.as_str(),
        "N" | "NE" | "E" | "SE" | "S" | "SW" | "W" | "NW" | "NNE" | "ENE" | "ESE" | "SSE" | "SSW" | "WSW" | "WNW" | "NNW"
    ) {
        return upper;
    }
    let mut num_end = 0;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if num_end > 0 {
        if let Ok(val) = clean[..num_end].parse::<f32>() {
            let unit = clean[num_end..].trim().to_lowercase();
            let deg = if unit.contains("rad") || unit.is_empty() {
                val.to_degrees().rem_euclid(360.0)
            } else {
                val.rem_euclid(360.0)
            };
            let dirs = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
            let idx = (((deg + 22.5) / 45.0).floor() as usize) % 8;
            return dirs[idx].to_string();
        }
    }
    clean.to_string()
}

pub fn parse_temp_num(raw: &str, to_metric: bool) -> Option<i32> {
    let clean = raw.trim();
    if clean.is_empty() || clean == "-" || clean == "..." || clean == "—" {
        return None;
    }
    let mut split_idx = 0;
    let mut has_digit = false;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            has_digit = true;
            split_idx = i + c.len_utf8();
        } else if has_digit && !c.is_whitespace() {
            break;
        }
    }
    if !has_digit {
        return None;
    }
    let num_str = clean[..split_idx].trim();
    let unit_str = clean[split_idx..].trim().to_uppercase();
    let num = num_str.parse::<f32>().ok()?;

    let final_val = match unit_str.as_str() {
        "C" | "°C" | "DEG C" | "DEGC" | "DEG" | "°" | "" => {
            if to_metric { num } else { num * 1.8 + 32.0 }
        }
        "F" | "°F" | "DEG F" | "DEGF" => {
            if to_metric { (num - 32.0) / 1.8 } else { num }
        }
        _ => num,
    };
    Some(final_val.round() as i32)
}

pub fn parse_wind_speed(raw: &str, to_metric: bool) -> (Option<i32>, &'static str) {
    let default_unit = if to_metric { "km/h" } else { "mph" };
    let clean = raw.trim();
    if clean.is_empty() || clean == "-" || clean == "..." || clean == "—" {
        return (None, default_unit);
    }
    let mut split_idx = 0;
    let mut has_digit = false;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            has_digit = true;
            split_idx = i + c.len_utf8();
        } else if has_digit && !c.is_whitespace() {
            break;
        }
    }
    if !has_digit {
        return (None, default_unit);
    }
    let num_str = clean[..split_idx].trim();
    let unit_str = clean[split_idx..].trim().to_uppercase();
    let num = match num_str.parse::<f32>() {
        Ok(n) => n,
        Err(_) => return (None, default_unit),
    };

    let (converted_val, unit) = match unit_str.as_str() {
        "M/S" | "M\\S" | "MPS" => {
            if to_metric { (num * 3.6, "km/h") } else { (num * 2.23694, "mph") }
        }
        "KPH" | "KM/H" | "KMH" => {
            if to_metric { (num, "km/h") } else { (num * 0.621371, "mph") }
        }
        "MPH" => {
            if to_metric { (num * 1.60934, "km/h") } else { (num, "mph") }
        }
        _ => {
            if to_metric { (num, "km/h") } else { (num, "mph") }
        }
    };
    (Some(converted_val.round().max(0.0) as i32), unit)
}

pub fn parse_wind_direction_rad(raw: &str) -> Option<f32> {
    let clean = raw.trim();
    if clean.is_empty() || clean == "..." || clean == "—" || clean == "-" {
        return None;
    }
    let upper = clean.to_uppercase();
    let named_rad = match upper.as_str() {
        "N" => Some(0.0),
        "NNE" => Some(22.5_f32.to_radians()),
        "NE" => Some(45.0_f32.to_radians()),
        "ENE" => Some(67.5_f32.to_radians()),
        "E" => Some(90.0_f32.to_radians()),
        "ESE" => Some(112.5_f32.to_radians()),
        "SE" => Some(135.0_f32.to_radians()),
        "SSE" => Some(157.5_f32.to_radians()),
        "S" => Some(180.0_f32.to_radians()),
        "SSW" => Some(202.5_f32.to_radians()),
        "SW" => Some(225.0_f32.to_radians()),
        "WSW" => Some(247.5_f32.to_radians()),
        "W" => Some(270.0_f32.to_radians()),
        "WNW" => Some(292.5_f32.to_radians()),
        "NW" => Some(315.0_f32.to_radians()),
        "NNW" => Some(337.5_f32.to_radians()),
        _ => None,
    };
    if let Some(rad) = named_rad {
        return Some(rad);
    }

    let mut num_end = 0;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if num_end > 0 {
        if let Ok(val) = clean[..num_end].parse::<f32>() {
            let unit = clean[num_end..].trim().to_lowercase();
            let rad = if unit.contains("deg") || unit.contains("°") {
                val.to_radians()
            } else {
                val
            };
            return Some(rad.rem_euclid(2.0 * std::f32::consts::PI));
        }
    }
    None
}

pub fn calculate_relative_wind_rad(wind_dir_rad: f32, yaw_north: f32) -> f32 {
    let two_pi = 2.0 * std::f32::consts::PI;
    // In iRacing telemetry, both WindDir and YawNorth are defined in radians clockwise from North (0=North, pi/2=East).
    // WindDir represents where the wind blows FROM. The air flow vector is (WindDir + PI).
    // Relative to the car heading (YawNorth), the air movement vector is (WindDir - YawNorth + PI).
    let rad = (wind_dir_rad - yaw_north + std::f32::consts::PI).rem_euclid(two_pi);
    if (rad - two_pi).abs() < 0.0001 || rad >= two_pi {
        0.0
    } else {
        rad
    }
}

pub fn describe_relative_wind(rel_rad: f32) -> &'static str {
    let deg = rel_rad.to_degrees().rem_euclid(360.0);
    if deg <= 22.5 || deg >= 337.5 {
        "Tailwind (Pushing from behind)"
    } else if deg > 22.5 && deg < 67.5 {
        "Quartering Tailwind (Left → Right)"
    } else if deg >= 67.5 && deg <= 112.5 {
        "Crosswind (Left → Right)"
    } else if deg > 112.5 && deg < 157.5 {
        "Quartering Headwind (Left → Right)"
    } else if deg >= 157.5 && deg <= 202.5 {
        "Headwind (Directly into front)"
    } else if deg > 202.5 && deg < 247.5 {
        "Quartering Headwind (Right → Left)"
    } else if deg >= 247.5 && deg <= 292.5 {
        "Crosswind (Right → Left)"
    } else {
        "Quartering Tailwind (Right → Left)"
    }
}

pub fn draw_wind_indicator(
    ui: &mut egui::Ui,
    speed_num: Option<i32>,
    speed_unit: &str,
    rel_wind_rad: Option<f32>,
    compass_str: &str,
    scale: f32,
) -> egui::Response {
    let spd_val = speed_num.unwrap_or(0);
    let has_wind = spd_val > 0;
    let spd_str = speed_num.map_or("—".to_string(), |s| s.to_string());

    // Generous sizing for crystal clarity
    let widget_size = (34.0 * scale).round().max(24.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(widget_size, widget_size), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let center = rect.center();
    let r = widget_size * 0.44;

    let cyan_bright = egui::Color32::from_rgb(0, 229, 200);      // #00E5C8 vibrant motorsport teal
    let cyan_glow = egui::Color32::from_rgb(34, 211, 238);        // #22D3EE bright cyan
    let cyan_dim = egui::Color32::from_rgba_unmultiplied(0, 229, 200, 75);
    let bg_circle = egui::Color32::from_rgb(10, 14, 20);         // Deep dark contrast backing
    let gold_nose = egui::Color32::from_rgb(250, 204, 21);       // Forward reference notch (car nose)

    // Dark circular background with subtle outer shadow/fill
    painter.circle_filled(center, r + 2.0 * scale, bg_circle);

    // Draw circular ring
    let ring_stroke = if has_wind {
        egui::Stroke::new(1.8 * scale, cyan_bright)
    } else {
        egui::Stroke::new(1.2 * scale, cyan_dim)
    };
    painter.circle_stroke(center, r, ring_stroke);

    // Forward car-nose reference pip at 12 o'clock (0° = straight ahead)
    // Drawn as a sleek, distinct gold triangle situated on the outer rim
    // This prevents any collision or "wonky" overlapping with the wind arrow when pointing near 12 o'clock!
    let pip_tip = center + egui::vec2(0.0, -r);
    let pip_top_left = center + egui::vec2(-2.5 * scale, -(r + 3.5 * scale));
    let pip_top_right = center + egui::vec2(2.5 * scale, -(r + 3.5 * scale));
    painter.add(egui::Shape::convex_polygon(
        vec![pip_tip, pip_top_left, pip_top_right],
        gold_nose,
        egui::Stroke::new(0.5 * scale, egui::Color32::from_rgb(40, 32, 6)),
    ));

    // If there is wind direction, draw the prominent directional pointer arrowhead and vector line
    if let (true, Some(rad)) = (has_wind, rel_wind_rad) {
        // rad: 0 is Forward (UP), PI/2 is Right (RIGHT), PI is Backward (DOWN), 3PI/2 is Left (LEFT)
        let dir_vec = egui::vec2(rad.sin(), -rad.cos());

        // Arrowhead: tip reaches the inner perimeter of the ring (never protruding or colliding with the nose pip)
        let tip = center + dir_vec * r;
        let arrow_len = 6.0 * scale;
        let base_center = center + dir_vec * (r - arrow_len);
        let normal = egui::vec2(-dir_vec.y, dir_vec.x);
        let half_w = 4.0 * scale;

        let left = base_center + normal * half_w;
        let right = base_center - normal * half_w;
        let notch = base_center + dir_vec * (1.6 * scale);

        // Vector pointer line starts outside the center text area so it doesn't cross the digits
        let clear_radius = (9.2 * scale).round();
        let stem_start = center + dir_vec * clear_radius;
        let stem_end = notch;
        if (stem_end - stem_start).dot(dir_vec) > 0.5 {
            painter.line_segment(
                [stem_start, stem_end],
                egui::Stroke::new(2.0 * scale, cyan_bright),
            );
        }

        // Prominent arrowhead chevron with crisp border
        painter.add(egui::Shape::convex_polygon(
            vec![tip, left, notch, right],
            cyan_glow,
            egui::Stroke::new(1.0 * scale, egui::Color32::WHITE),
        ));
    }

    // Center wind speed text: bold 14pt typography for both single and double digits
    let font_size = (14.0 * scale).round().max(10.5);
    let text_color = if has_wind {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_rgb(139, 147, 163)
    };

    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        spd_str,
        egui::FontId::proportional(font_size),
        text_color,
    );

    let tooltip_text = if let Some(spd) = speed_num {
        let rel_desc = rel_wind_rad.map_or("", describe_relative_wind);
        let compass_info = if !compass_str.is_empty() {
            format!(" · True Compass: {}", compass_str)
        } else {
            String::new()
        };
        let deg_info = rel_wind_rad.map_or(String::new(), |r| {
            format!(" ({:.0}° rel)", r.to_degrees().rem_euclid(360.0))
        });
        if !rel_desc.is_empty() {
            format!("Wind: {} {} ({}){}{}", spd, speed_unit, rel_desc, compass_info, deg_info)
        } else {
            format!("Wind: {} {}{}{}", spd, speed_unit, compass_info, deg_info)
        }
    } else {
        "Wind: Awaiting telemetry".to_string()
    };

    response.on_hover_text(tooltip_text)
}

pub fn format_time(t: f32) -> String {
    if t <= 0.0 { 
        "-".to_string() 
    } else { 
        let mins = (t / 60.0).floor() as i32;
        let secs = t % 60.0;
        if mins > 0 {
            format!("{}:{:06.3}", mins, secs)
        } else {
            format!("{:.3}", secs)
        }
    }
}

pub fn format_laps_column_tsv(stint: &StintData) -> String {
    let mut out = String::new();
    for &t in stint.valid_laps().iter() {
        out.push_str(&format!("{:.3}\n", t));
    }
    out
}

pub fn format_stint_laps_spreadsheet_tsv(stint: &StintData) -> String {
    let numbered_laps = stint.numbered_valid_laps();
    let mut out = String::from("Stint Lap\tLap\tLap Time\tAvg Lap Time\n");
    let mut sum = 0.0;
    for (i, &(overall_lap, t)) in numbered_laps.iter().enumerate() {
        sum += t;
        let avg = sum / (i + 1) as f32;
        out.push_str(&format!("{}\t{}\t{:.3}\t{:.3}\n", i + 1, overall_lap, t, avg));
    }
    out
}

pub fn format_stint_laps_tsv(stint: &StintData) -> String {
    let mut out = String::new();
    out.push_str("Lap\tTime (s)\tTime (Formatted)\n");
    for (i, &t) in stint.valid_laps().iter().enumerate() {
        out.push_str(&format!("{}\t{:.3}\t{}\n", i + 1, t, format_time(t)));
    }
    out
}

pub fn format_single_tire_tsv(corner: &str, tire: &TireCornerData, use_metric: bool) -> String {
    let press_str = if tire.pressure > 0.1 {
        if use_metric { format!("{:.2} bar", tire.pressure / 100.0) } else { format!("{:.1} PSI", tire.pressure * 0.145038) }
    } else {
        "—".to_string()
    };
    let temp_str = if tire.temp_m > 1.0 {
        if use_metric {
            format!("{:.0}°·{:.0}°·{:.0}°C", tire.temp_l, tire.temp_m, tire.temp_r)
        } else {
            format!("{:.0}°·{:.0}°·{:.0}°F", tire.temp_l * 1.8 + 32.0, tire.temp_m * 1.8 + 32.0, tire.temp_r * 1.8 + 32.0)
        }
    } else {
        "—".to_string()
    };
    let wear_str = if tire.wear_m > 0.01 {
        format!("{:.0}%·{:.0}%·{:.0}%", tire.wear_l, tire.wear_m, tire.wear_r)
    } else {
        "—".to_string()
    };

    format!("{}\t{}\t{}\t{}", corner, press_str, temp_str, wear_str)
}

pub fn format_single_tire_temps_tsv(tire: &TireCornerData, use_metric: bool) -> String {
    if tire.temp_m > 1.0 {
        if use_metric {
            format!("{:.0}\t{:.0}\t{:.0}", tire.temp_l, tire.temp_m, tire.temp_r)
        } else {
            format!("{:.0}\t{:.0}\t{:.0}", tire.temp_l * 1.8 + 32.0, tire.temp_m * 1.8 + 32.0, tire.temp_r * 1.8 + 32.0)
        }
    } else {
        "—\t—\t—".to_string()
    }
}

pub fn format_single_tire_wear_tsv(tire: &TireCornerData) -> String {
    if tire.wear_m > 0.01 {
        format!("{:.0}%\t{:.0}%\t{:.0}%", tire.wear_l, tire.wear_m, tire.wear_r)
    } else {
        "—\t—\t—".to_string()
    }
}

pub fn format_temps_excel_tsv(tires: &StintTireSet, use_metric: bool) -> String {
    let unit = if use_metric { "°C" } else { "°F" };
    let mut out = String::new();
    out.push_str(&format!("Corner (OMI / IMO)\tZone 1 ({})\tZone 2 ({})\tZone 3 ({})\n", unit, unit, unit));
    out.push_str(&format!("Left Front (LF · OMI)\t{}\n", format_single_tire_temps_tsv(&tires.lf, use_metric)));
    out.push_str(&format!("Right Front (RF · IMO)\t{}\n", format_single_tire_temps_tsv(&tires.rf, use_metric)));
    out.push_str(&format!("Left Rear (LR · OMI)\t{}\n", format_single_tire_temps_tsv(&tires.lr, use_metric)));
    out.push_str(&format!("Right Rear (RR · IMO)\t{}\n", format_single_tire_temps_tsv(&tires.rr, use_metric)));
    out
}

pub fn format_wear_excel_tsv(tires: &StintTireSet) -> String {
    let mut out = String::new();
    out.push_str("Corner (OMI / IMO)\tZone 1 Wear\tZone 2 Wear\tZone 3 Wear\n");
    out.push_str(&format!("Left Front (LF · OMI)\t{}\n", format_single_tire_wear_tsv(&tires.lf)));
    out.push_str(&format!("Right Front (RF · IMO)\t{}\n", format_single_tire_wear_tsv(&tires.rf)));
    out.push_str(&format!("Left Rear (LR · OMI)\t{}\n", format_single_tire_wear_tsv(&tires.lr)));
    out.push_str(&format!("Right Rear (RR · IMO)\t{}\n", format_single_tire_wear_tsv(&tires.rr)));
    out
}

pub fn format_tires_excel_tsv(tires: &StintTireSet, use_metric: bool) -> String {
    let mut out = String::new();
    out.push_str("Corner\tHot Pressure\tTemps (OMI / IMO)\tWear (OMI / IMO)\n");
    out.push_str(&format_single_tire_tsv("Left Front (LF)", &tires.lf, use_metric));
    out.push('\n');
    out.push_str(&format_single_tire_tsv("Right Front (RF)", &tires.rf, use_metric));
    out.push('\n');
    out.push_str(&format_single_tire_tsv("Left Rear (LR)", &tires.lr, use_metric));
    out.push('\n');
    out.push_str(&format_single_tire_tsv("Right Rear (RR)", &tires.rr, use_metric));
    out.push('\n');
    out
}

pub fn format_full_stint_excel_tsv(stint: &StintData, active_buckets: &[usize], use_metric: bool) -> String {
    let mut out = String::new();
    let fuel_conv = if use_metric { 1.0 } else { 0.264172 };
    let fuel_unit = if use_metric { "L" } else { "Gal" };

    out.push_str(&format!("=== STINT S{} SUMMARY ===\n", stint.stint_number));
    out.push_str("Stint #\tLaps Completed\tFastest Lap\tAverage Lap\tAvg Fuel/Lap\tConsistency (σ)\tSetup\tTrack Temp\tAir Temp\n");
    out.push_str(&format!(
        "S{}\t{}\t{}\t{}\t{:.3} {}\t{:.3}\t{}\t{}\t{}\n\n",
        stint.stint_number,
        stint.laps_completed,
        format_time(stint.fast_lap),
        format_time(stint.full_average()),
        stint.fuel_per_lap() * fuel_conv,
        fuel_unit,
        stint.std_dev(),
        stint.setup_name,
        stint.track_temp_at_start,
        stint.air_temp_at_start
    ));

    if !active_buckets.is_empty() {
        out.push_str("=== ROLLING AVERAGES ===\n");
        let headers: Vec<String> = active_buckets.iter().map(|b| format!("AVG {}", b)).collect();
        out.push_str(&headers.join("\t"));
        out.push('\n');
        let vals: Vec<String> = active_buckets.iter().map(|b| {
            let avg = stint.avg_first_n(*b);
            if avg > 0.0 { format_time(avg) } else { "—".to_string() }
        }).collect();
        out.push_str(&vals.join("\t"));
        out.push_str("\n\n");
    }

    if stint.tires.has_data {
        out.push_str("=== 4-CORNER POST-STINT TIRES ===\n");
        out.push_str(&format_tires_excel_tsv(&stint.tires, use_metric));
        out.push('\n');
    }

    out.push_str("=== LAP-BY-LAP TELEMETRY ===\n");
    out.push_str(&format_stint_laps_tsv(stint));

    out
}

pub fn format_stints_comparison_tsv(
    stints_info: &[(&StintData, &str, &str, &str, &str, bool)],
    active_buckets: &[usize],
    use_metric: bool,
) -> String {
    let mut out = String::new();
    out.push_str("THE SEARCH FOR SPEED — STINT COMPARISON REPORT\n\n");

    // Header row
    out.push_str("METRIC");
    for info in stints_info {
        out.push_str(&format!("\t{} (#{} {})", info.1, info.3, info.2));
    }
    out.push('\n');

    // Stint Number
    out.push_str("Stint Number");
    for info in stints_info {
        out.push_str(&format!("\tS{}", info.0.stint_number));
    }
    out.push('\n');

    // Driver
    out.push_str("Driver");
    for info in stints_info {
        out.push_str(&format!("\t#{} {}", info.3, info.2));
    }
    out.push('\n');

    // Setup
    out.push_str("Setup");
    for info in stints_info {
        out.push_str(&format!("\t{}", info.4));
    }
    out.push('\n');

    // Laps Completed
    out.push_str("Laps Completed");
    for info in stints_info {
        out.push_str(&format!("\t{}", info.0.laps_completed));
    }
    out.push('\n');

    // Fastest Lap
    out.push_str("Fastest Lap");
    for info in stints_info {
        out.push_str(&format!("\t{}", format_time(info.0.fast_lap)));
    }
    out.push('\n');

    // Average Lap
    out.push_str("Average Lap");
    for info in stints_info {
        out.push_str(&format!("\t{}", format_time(info.0.full_average())));
    }
    out.push('\n');

    // Consistency (Std Dev)
    out.push_str("Consistency (Std Dev)");
    for info in stints_info {
        out.push_str(&format!("\t{:.3}", info.0.std_dev()));
    }
    out.push('\n');

    // Pace Fall-off
    out.push_str("Pace Fall-off (1-5 vs Last 5)");
    for info in stints_info {
        out.push_str(&format!("\t{:.3}s", info.0.drop_off()));
    }
    out.push('\n');

    // Rolling Averages
    for &b in active_buckets {
        out.push_str(&format!("Avg {}", b));
        for info in stints_info {
            let avg = info.0.avg_first_n(b);
            let s = if avg > 0.0 { format_time(avg) } else { "—".to_string() };
            out.push_str(&format!("\t{}", s));
        }
        out.push('\n');
    }

    // Fuel per Lap
    let fuel_conv = if use_metric { 1.0 } else { 0.264172 };
    let fuel_unit = if use_metric { "L" } else { "gal" };
    out.push_str(&format!("Fuel / Lap ({})", fuel_unit));
    for info in stints_info {
        if info.5 {
            let f = info.0.fuel_per_lap() * fuel_conv;
            out.push_str(&format!("\t{:.3}", f));
        } else {
            out.push_str("\t—");
        }
    }
    out.push('\n');

    // Track Temp
    out.push_str("Track Temp");
    for info in stints_info {
        out.push_str(&format!("\t{}", convert_unit(&info.0.track_temp_at_start, use_metric)));
    }
    out.push('\n');

    // Air Temp
    out.push_str("Air Temp");
    for info in stints_info {
        out.push_str(&format!("\t{}", convert_unit(&info.0.air_temp_at_start, use_metric)));
    }
    out.push_str("\n\n");

    // Lap-by-Lap Table
    let valid_laps_per_stint: Vec<Vec<f32>> = stints_info.iter().map(|info| info.0.valid_laps()).collect();
    let max_laps = valid_laps_per_stint.iter().map(|v| v.len()).max().unwrap_or(0);

    out.push_str("LAP-BY-LAP COMPARISON\n");
    out.push_str("Stint Lap");
    for info in stints_info {
        out.push_str(&format!("\t{} Time", info.1));
    }
    if stints_info.len() >= 2 {
        out.push_str(&format!("\tDelta ({} vs {})", stints_info[1].1, stints_info[0].1));
    }
    if stints_info.len() >= 3 {
        out.push_str(&format!("\tDelta ({} vs {})", stints_info[2].1, stints_info[0].1));
    }
    out.push('\n');

    for lap_idx in 0..max_laps {
        out.push_str(&format!("L{}", lap_idx + 1));
        let mut lap_times = Vec::new();
        for v in &valid_laps_per_stint {
            if lap_idx < v.len() {
                let t = v[lap_idx];
                lap_times.push(Some(t));
                out.push_str(&format!("\t{:.3}", t));
            } else {
                lap_times.push(None);
                out.push_str("\t—");
            }
        }

        // Deltas
        if lap_times.len() >= 2 {
            match (lap_times[0], lap_times[1]) {
                (Some(t0), Some(t1)) => {
                    let d = t1 - t0;
                    let sign = if d >= 0.0 { "+" } else { "" };
                    out.push_str(&format!("\t{}{:.3}", sign, d));
                }
                _ => out.push_str("\t—"),
            }
        }
        if lap_times.len() >= 3 {
            match (lap_times[0], lap_times[2]) {
                (Some(t0), Some(t2)) => {
                    let d = t2 - t0;
                    let sign = if d >= 0.0 { "+" } else { "" };
                    out.push_str(&format!("\t{}{:.3}", sign, d));
                }
                _ => out.push_str("\t—"),
            }
        }
        out.push('\n');
    }

    out
}

pub fn compute_overlay_laps_remain(
    session_laps_total: i32,
    session_laps_remain: i32,
    player_lap: i32,
    is_white: bool,
    is_checkered: bool,
) -> i32 {
    if is_checkered {
        if session_laps_total > 0 && player_lap >= session_laps_total {
            0
        } else if session_laps_remain == 0 {
            0
        } else {
            1
        }
    } else if is_white {
        1
    } else if session_laps_total > 0 && session_laps_total < 10000 {
        (session_laps_total - player_lap).max(0)
    } else if session_laps_remain > 0 && session_laps_remain < 10000 {
        session_laps_remain
    } else {
        0
    }
}

pub fn format_overlay_race_remain_strings(
    laps_remain: i32,
    player_lap: i32,
    is_checkered: bool,
    session_laps_total: i32,
) -> (String, String) {
    let rem_str = if laps_remain > 1 {
        format!("{} laps", laps_remain)
    } else if laps_remain == 1 {
        "1 lap".to_string()
    } else if is_checkered || (session_laps_total > 0 && player_lap >= session_laps_total) {
        "Finished".to_string()
    } else {
        "Unlimited".to_string()
    };

    let lap_done_str = if player_lap <= 0 {
        "0 laps done".to_string()
    } else if player_lap == 1 {
        "1 lap done".to_string()
    } else {
        format!("Lap {} done", player_lap)
    };

    (rem_str, lap_done_str)
}
