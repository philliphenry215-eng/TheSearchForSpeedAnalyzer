use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use image::{Rgba, RgbaImage};
use std::io::Cursor;

use crate::{StintData, StintTireSet, TireCornerData, WarRoomState, convert_unit, format_time, format_wind_dir};
use crate::settings::AppSettings;

pub struct DiscordStintEntry<'a> {
    pub slot_label: Option<&'a str>, // None for Single Stint, Some("STINT A") for Compare
    pub driver_name: &'a str,
    pub car_num: &'a str,
    pub setup_name: &'a str,
    pub is_player: bool,
    pub stint: &'a StintData,
    pub accent_rgb: [u8; 3],
}

#[derive(Clone, Copy)]
struct Color([u8; 4]);

impl Color {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }
    const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }
}

fn blend_pixel(img: &mut RgbaImage, x: i32, y: i32, color: Color, coverage: f32) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let alpha = ((color.0[3] as f32 / 255.0) * coverage).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
    let dst = img.get_pixel_mut(x as u32, y as u32);
    let inv = 1.0 - alpha;
    dst[0] = (color.0[0] as f32 * alpha + dst[0] as f32 * inv).round() as u8;
    dst[1] = (color.0[1] as f32 * alpha + dst[1] as f32 * inv).round() as u8;
    dst[2] = (color.0[2] as f32 * alpha + dst[2] as f32 * inv).round() as u8;
    dst[3] = 255;
}

fn sdf_rounded_box(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let hx = (w * 0.5 - r).max(0.0);
    let hy = (h * 0.5 - r).max(0.0);
    let dx = (px - cx).abs() - hx;
    let dy = (py - cy).abs() - hy;
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    let inside = dx.max(dy).min(0.0);
    outside + inside - r
}

fn fill_rounded_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let min_x = (x.floor() as i32 - 1).max(0);
    let min_y = (y.floor() as i32 - 1).max(0);
    let max_x = ((x + w).ceil() as i32 + 1).min(img.width() as i32 - 1);
    let max_y = ((y + h).ceil() as i32 + 1).min(img.height() as i32 - 1);

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let d = sdf_rounded_box(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, r);
            if d < 0.5 {
                let cov = (0.5 - d).clamp(0.0, 1.0);
                blend_pixel(img, px, py, color, cov);
            }
        }
    }
}

fn stroke_rounded_rect(
    img: &mut RgbaImage,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    thickness: f32,
    color: Color,
) {
    let half_t = thickness * 0.5;
    let min_x = (x.floor() as i32 - 2).max(0);
    let min_y = (y.floor() as i32 - 2).max(0);
    let max_x = ((x + w).ceil() as i32 + 2).min(img.width() as i32 - 1);
    let max_y = ((y + h).ceil() as i32 + 2).min(img.height() as i32 - 1);

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let d = sdf_rounded_box(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, r);
            let dist_to_edge = (d.abs() - half_t).max(0.0);
            if dist_to_edge < 1.0 {
                let cov = (1.0 - dist_to_edge).clamp(0.0, 1.0);
                blend_pixel(img, px, py, color, cov);
            }
        }
    }
}

fn fill_circle(img: &mut RgbaImage, cx: f32, cy: f32, r: f32, color: Color) {
    let min_x = (cx - r - 1.0).floor() as i32;
    let max_x = (cx + r + 1.0).ceil() as i32;
    let min_y = (cy - r - 1.0).floor() as i32;
    let max_y = (cy + r + 1.0).ceil() as i32;
    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let dist = ((px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2)).sqrt();
            let d = dist - r;
            if d < 0.5 {
                let cov = (0.5 - d).clamp(0.0, 1.0);
                blend_pixel(img, px, py, color, cov);
            }
        }
    }
}

fn layout_glyphs(font: &FontRef, px_scale: f32, text: &str) -> (Vec<ab_glyph::Glyph>, f32) {
    let scale = PxScale::from(px_scale);
    let scaled = font.as_scaled(scale);
    let mut glyphs = Vec::new();
    let mut caret_x = 0.0f32;
    let mut last_id = None;

    for ch in text.chars() {
        if ch.is_control() {
            continue;
        }
        let id = scaled.glyph_id(ch);
        if let Some(prev) = last_id {
            caret_x += scaled.kern(prev, id);
        }
        let g = id.with_scale_and_position(scale, point(caret_x, scaled.ascent()));
        caret_x += scaled.h_advance(id);
        last_id = Some(id);
        glyphs.push(g);
    }
    (glyphs, caret_x)
}

fn measure_text(font: &FontRef, px_scale: f32, text: &str) -> f32 {
    layout_glyphs(font, px_scale, text).1
}

fn truncate_to_width(font: &FontRef, px_scale: f32, text: &str, max_w: f32) -> String {
    if measure_text(font, px_scale, text) <= max_w {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if measure_text(font, px_scale, &candidate) <= max_w {
            return candidate;
        }
    }
    "…".to_string()
}

fn draw_text(
    img: &mut RgbaImage,
    font: &FontRef,
    x: f32,
    y: f32,
    px_scale: f32,
    color: Color,
    text: &str,
) -> f32 {
    let (glyphs, width) = layout_glyphs(font, px_scale, text);
    for g in glyphs {
        if let Some(outlined) = font.outline_glyph(g) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                if cov > 0.01 {
                    let px = x.round() as i32 + bounds.min.x as i32 + gx as i32;
                    let py = y.round() as i32 + bounds.min.y as i32 + gy as i32;
                    blend_pixel(img, px, py, color, cov);
                }
            });
        }
    }
    width
}

fn draw_text_right(
    img: &mut RgbaImage,
    font: &FontRef,
    right_x: f32,
    y: f32,
    px_scale: f32,
    color: Color,
    text: &str,
) -> f32 {
    let w = measure_text(font, px_scale, text);
    draw_text(img, font, right_x - w, y, px_scale, color, text);
    w
}

pub fn render_discord_stint_card(
    entries: &[DiscordStintEntry],
    s: &WarRoomState,
    settings: &AppSettings,
) -> Result<RgbaImage, String> {
    if entries.is_empty() {
        return Err("No stint data to render".to_string());
    }

    let font_regular = FontRef::try_from_slice(include_bytes!("../assets/fonts/Inter-Medium.ttf"))
        .map_err(|e| format!("Font error: {}", e))?;
    let font_bold = FontRef::try_from_slice(include_bytes!("../assets/fonts/Inter-SemiBold.ttf"))
        .map_err(|e| format!("Font error: {}", e))?;
    let font_display = FontRef::try_from_slice(include_bytes!("../assets/fonts/RacingSansOne-Regular.ttf"))
        .map_err(|e| format!("Font error: {}", e))?;

    let is_metric = settings.use_metric;
    let is_compare = entries.len() > 1 || entries[0].slot_label.is_some();

    let buckets: Vec<usize> = if settings.active_avg_buckets.is_empty() {
        vec![5, 10, 15, 20, 25, 30]
    } else {
        settings.active_avg_buckets.clone()
    };
    let avg_rows = ((buckets.len() + 2) / 3).max(1);

    let num_cols = entries.len();
    let pad_x = 22.0f32;
    let col_gap = 16.0f32;
    let col_w = match num_cols {
        1 => 696.0f32,
        2 => 480.0f32,
        _ => 448.0f32,
    };

    let img_w = (pad_x * 2.0 + num_cols as f32 * col_w + (num_cols.saturating_sub(1)) as f32 * col_gap).round() as u32;
    let avg_section_h = 22.0 + (avg_rows as f32 * 40.0) + ((avg_rows.saturating_sub(1)) as f32 * 8.0);
    let col_top = 114.0f32;
    let tire_section_top = col_top + 74.0 + 10.0 + 86.0 + 8.0 + 30.0 + 14.0 + avg_section_h + 14.0;
    let tire_section_h = 22.0 + 92.0 + 10.0 + 92.0;
    let img_h = (tire_section_top + tire_section_h + 22.0).round() as u32;

    let bg_canvas = Color::rgb(11, 14, 20);
    let panel_bg = Color::rgb(18, 22, 31);
    let card_bg = Color::rgb(24, 29, 40);
    let border_subtle = Color::rgba(255, 255, 255, 22);
    let text_white = Color::rgb(242, 245, 250);
    let text_muted = Color::rgb(142, 152, 170);
    let primary_gold = Color::rgb(245, 184, 46);
    let status_green = Color::rgb(75, 225, 110);
    let status_amber = Color::rgb(255, 175, 45);

    let mut img = RgbaImage::from_pixel(img_w, img_h, Rgba(bg_canvas.0));

    // Outer frame border
    stroke_rounded_rect(
        &mut img,
        1.0,
        1.0,
        img_w as f32 - 2.0,
        img_h as f32 - 2.0,
        12.0,
        1.5,
        Color::rgba(245, 184, 46, 70),
    );

    // --- 1. TOP HEADER & SESSION WEATHER / TRACK STATE BANNER ---
    let header_title = if is_compare {
        "SEARCH FOR SPEED  ·  STINT COMPARISON"
    } else {
        "SEARCH FOR SPEED  ·  STINT REPORT"
    };
    draw_text(&mut img, &font_display, pad_x, 18.0, 22.0, primary_gold, header_title);

    let track_display = if !s.track_name.is_empty() && s.track_name != "Waiting for Sim..." && s.track_name != "..." {
        s.track_name.as_str()
    } else {
        "Session Telemetry"
    };
    let car_display = if !s.car_name.is_empty() && s.car_name != "..." && s.car_name != "—" {
        s.car_name.as_str()
    } else {
        ""
    };
    let sess_type_display = if !s.session_type.is_empty() && s.session_type != "..." && s.session_type != "Offline" {
        s.session_type.as_str()
    } else {
        ""
    };

    let mut context_parts: Vec<&str> = vec![track_display];
    if !car_display.is_empty() {
        context_parts.push(car_display);
    }
    if !sess_type_display.is_empty() {
        context_parts.push(sess_type_display);
    }
    let context_line = context_parts.join("  ·  ");
    let title_px_w = measure_text(&font_display, 22.0, header_title);
    let max_ctx_w = (img_w as f32 - pad_x * 2.0 - title_px_w - 24.0).max(200.0);
    let context_trunc = truncate_to_width(&font_bold, 13.5, &context_line, max_ctx_w);
    draw_text_right(&mut img, &font_bold, img_w as f32 - pad_x, 22.0, 13.5, text_white, &context_trunc);

    // Session Weather & Track State Bar (Shared across the entire card)
    let weather_bar_y = 54.0f32;
    let weather_bar_h = 44.0f32;
    let weather_bar_w = img_w as f32 - pad_x * 2.0;
    fill_rounded_rect(&mut img, pad_x, weather_bar_y, weather_bar_w, weather_bar_h, 8.0, panel_bg);
    stroke_rounded_rect(&mut img, pad_x, weather_bar_y, weather_bar_w, weather_bar_h, 8.0, 1.0, border_subtle);

    let rubber_state = if !s.track_state.is_empty() && s.track_state != "..." && s.track_state != "—" {
        s.track_state.clone()
    } else {
        "Clean Track".to_string()
    };
    let sky_str = if !s.weather_type.is_empty() && s.weather_type != "..." {
        s.weather_type.clone()
    } else {
        "Clear".to_string()
    };
    let raw_trk = if !s.track_temp_raw.is_empty() && s.track_temp_raw != "..." && s.track_temp_raw != "—" {
        &s.track_temp_raw
    } else if !entries[0].stint.track_temp_at_start.is_empty() {
        &entries[0].stint.track_temp_at_start
    } else {
        "—"
    };
    let raw_air = if !s.air_temp_raw.is_empty() && s.air_temp_raw != "..." && s.air_temp_raw != "—" {
        &s.air_temp_raw
    } else if !entries[0].stint.air_temp_at_start.is_empty() {
        &entries[0].stint.air_temp_at_start
    } else {
        "—"
    };
    let trk_temp = convert_unit(raw_trk, is_metric);
    let air_temp = convert_unit(raw_air, is_metric);
    let wind_spd = if !s.wind_speed_raw.is_empty() && s.wind_speed_raw != "..." {
        convert_unit(&s.wind_speed_raw, is_metric)
    } else {
        "—".to_string()
    };
    let wind_dir = format_wind_dir(&s.wind_dir_raw);
    let rh_str = if !s.relative_humidity_raw.is_empty() && s.relative_humidity_raw != "..." {
        convert_unit(&s.relative_humidity_raw, is_metric)
    } else {
        "—".to_string()
    };

    let wind_combined = if wind_dir.is_empty() || wind_spd == "—" {
        wind_spd
    } else {
        format!("{} {}", wind_spd, wind_dir)
    };

    // Draw Session Weather & Rubber Pills inside the Weather Bar with proportional widths
    let weather_items: [(&str, String, Color, f32); 6] = [
        ("TRACK STATE", rubber_state, primary_gold, 0.24),
        ("WEATHER", sky_str, text_white, 0.16),
        ("TRACK TEMP", trk_temp, text_white, 0.15),
        ("AIR TEMP", air_temp, text_white, 0.14),
        ("WIND", wind_combined, text_white, 0.17),
        ("HUMIDITY", rh_str, text_white, 0.14),
    ];

    let mut cur_cell_x = pad_x;
    for (idx, (lbl, val, val_col, weight)) in weather_items.iter().enumerate() {
        let cell_w = weather_bar_w * (*weight);
        let cx = cur_cell_x + 12.0;
        if idx > 0 {
            fill_rounded_rect(
                &mut img,
                cur_cell_x,
                weather_bar_y + 8.0,
                1.0,
                weather_bar_h - 16.0,
                0.5,
                border_subtle,
            );
        }
        draw_text(&mut img, &font_bold, cx, weather_bar_y + 7.0, 10.0, text_muted, lbl);
        let val_trunc = truncate_to_width(&font_bold, 13.5, val, cell_w - 18.0);
        draw_text(&mut img, &font_bold, cx, weather_bar_y + 21.0, 13.5, *val_col, &val_trunc);
        cur_cell_x += cell_w;
    }

    // Precompute best metrics across stints for comparison highlighting
    let best_fast_lap = entries
        .iter()
        .map(|e| e.stint.fast_lap)
        .filter(|&t| t > 0.0)
        .fold(f32::INFINITY, f32::min);
    let best_full_avg = entries
        .iter()
        .map(|e| e.stint.full_average())
        .filter(|&t| t > 0.0)
        .fold(f32::INFINITY, f32::min);

    let fuel_conv = if is_metric { 1.0 } else { 0.264172 };
    let fuel_unit = if is_metric { "L" } else { "gal" };

    // --- 2. RENDER EACH STINT COLUMN ---
    for (col_idx, entry) in entries.iter().enumerate() {
        let x0 = pad_x + col_idx as f32 * (col_w + col_gap);
        let accent = Color::rgb(entry.accent_rgb[0], entry.accent_rgb[1], entry.accent_rgb[2]);
        let accent_border = Color::rgba(entry.accent_rgb[0], entry.accent_rgb[1], entry.accent_rgb[2], 170);

        // A. Stint & Driver & Setup Card
        let header_card_y = col_top;
        let header_card_h = 74.0f32;
        fill_rounded_rect(&mut img, x0, header_card_y, col_w, header_card_h, 10.0, panel_bg);
        stroke_rounded_rect(&mut img, x0, header_card_y, col_w, header_card_h, 10.0, 1.5, accent_border);

        fill_circle(&mut img, x0 + 18.0, header_card_y + 22.0, 5.0, accent);

        let stint_title = match entry.slot_label {
            Some(slot) => format!("{} · S{} · #{} {}", slot, entry.stint.stint_number, entry.car_num, entry.driver_name),
            None => format!("STINT S{} · #{} {}", entry.stint.stint_number, entry.car_num, entry.driver_name),
        };
        let title_trunc = truncate_to_width(&font_bold, 16.5, &stint_title, col_w - 125.0);
        draw_text(&mut img, &font_bold, x0 + 30.0, header_card_y + 12.0, 16.5, accent, &title_trunc);

        // Laps badge on top-right of Stint Header Card
        let laps_badge_text = format!("{} LAPS", entry.stint.laps_completed);
        let badge_w = measure_text(&font_display, 14.5, &laps_badge_text) + 18.0;
        let badge_x = x0 + col_w - badge_w - 12.0;
        fill_rounded_rect(
            &mut img,
            badge_x,
            header_card_y + 10.0,
            badge_w,
            24.0,
            6.0,
            Color::rgba(entry.accent_rgb[0], entry.accent_rgb[1], entry.accent_rgb[2], 32),
        );
        draw_text(&mut img, &font_display, badge_x + 9.0, header_card_y + 14.0, 14.5, text_white, &laps_badge_text);

        // Setup row inside Stint Header Card
        draw_text(&mut img, &font_bold, x0 + 14.0, header_card_y + 44.0, 12.0, text_muted, "SETUP:");
        let setup_trunc = truncate_to_width(&font_bold, 13.5, entry.setup_name, col_w - 82.0);
        draw_text(&mut img, &font_bold, x0 + 68.0, header_card_y + 43.0, 13.5, text_white, &setup_trunc);

        // B. 3 Hero Lap Time Metric Cards (Fastest Lap | Last Lap | Full Average)
        let hero_y = header_card_y + header_card_h + 10.0;
        let hero_h = 86.0f32;
        let hero_gap = 8.0f32;
        let hero_w = (col_w - hero_gap * 2.0) / 3.0;

        let valid_laps = entry.stint.valid_laps();
        let last_lap_val = valid_laps.last().copied().unwrap_or(entry.stint.last_lap);
        let full_avg_val = entry.stint.full_average();

        // 1) Fastest Lap Box
        fill_rounded_rect(&mut img, x0, hero_y, hero_w, hero_h, 8.0, card_bg);
        stroke_rounded_rect(&mut img, x0, hero_y, hero_w, hero_h, 8.0, 1.0, border_subtle);
        draw_text(&mut img, &font_bold, x0 + 10.0, hero_y + 9.0, 10.5, text_muted, "FASTEST LAP");
        let fast_str = if entry.stint.fast_lap > 0.0 {
            format_time(entry.stint.fast_lap)
        } else {
            "—".to_string()
        };
        let is_best_fast = entry.stint.fast_lap > 0.0 && (entry.stint.fast_lap - best_fast_lap).abs() < 0.0001;
        let fast_color = if is_best_fast || !is_compare { status_green } else { text_white };
        draw_text(&mut img, &font_display, x0 + 10.0, hero_y + 28.0, 23.0, fast_color, &fast_str);
        if is_compare && entry.stint.fast_lap > 0.0 && best_fast_lap < f32::INFINITY {
            if is_best_fast {
                draw_text(&mut img, &font_bold, x0 + 10.0, hero_y + 61.0, 11.5, status_green, "★ BEST PACE");
            } else {
                let d = entry.stint.fast_lap - best_fast_lap;
                draw_text(&mut img, &font_bold, x0 + 10.0, hero_y + 61.0, 11.5, status_amber, &format!("+{:.3}s", d));
            }
        } else {
            draw_text(&mut img, &font_regular, x0 + 10.0, hero_y + 61.0, 11.0, text_muted, "Stint Best");
        }

        // 2) Last Lap Box
        let hx1 = x0 + hero_w + hero_gap;
        fill_rounded_rect(&mut img, hx1, hero_y, hero_w, hero_h, 8.0, card_bg);
        stroke_rounded_rect(&mut img, hx1, hero_y, hero_w, hero_h, 8.0, 1.0, border_subtle);
        draw_text(&mut img, &font_bold, hx1 + 10.0, hero_y + 9.0, 10.5, text_muted, "LAST LAP");
        let last_str = if last_lap_val > 0.0 {
            format_time(last_lap_val)
        } else {
            "—".to_string()
        };
        draw_text(&mut img, &font_display, hx1 + 10.0, hero_y + 28.0, 23.0, text_white, &last_str);
        let last_sub = if entry.stint.laps_completed > 0 {
            format!("Stint Lap {}", entry.stint.laps_completed)
        } else {
            "No laps".to_string()
        };
        draw_text(&mut img, &font_regular, hx1 + 10.0, hero_y + 61.0, 11.0, text_muted, &last_sub);

        // 3) Full Average Box
        let hx2 = x0 + (hero_w + hero_gap) * 2.0;
        fill_rounded_rect(&mut img, hx2, hero_y, hero_w, hero_h, 8.0, card_bg);
        stroke_rounded_rect(&mut img, hx2, hero_y, hero_w, hero_h, 8.0, 1.0, border_subtle);
        draw_text(&mut img, &font_bold, hx2 + 10.0, hero_y + 9.0, 10.5, text_muted, "FULL AVERAGE");
        let avg_str = if full_avg_val > 0.0 {
            format_time(full_avg_val)
        } else {
            "—".to_string()
        };
        let is_best_avg = full_avg_val > 0.0 && (full_avg_val - best_full_avg).abs() < 0.0001;
        let avg_color = if is_best_avg && is_compare { status_green } else { text_white };
        draw_text(&mut img, &font_display, hx2 + 10.0, hero_y + 28.0, 23.0, avg_color, &avg_str);
        if is_compare && full_avg_val > 0.0 && best_full_avg < f32::INFINITY {
            if is_best_avg {
                draw_text(&mut img, &font_bold, hx2 + 10.0, hero_y + 61.0, 11.5, status_green, "★ BEST AVG");
            } else {
                let d = full_avg_val - best_full_avg;
                draw_text(&mut img, &font_bold, hx2 + 10.0, hero_y + 61.0, 11.5, status_amber, &format!("+{:.3}s", d));
            }
        } else {
            draw_text(&mut img, &font_regular, hx2 + 10.0, hero_y + 61.0, 11.0, text_muted, "All Green Laps");
        }

        // Secondary Stats Strip (Consistency, Fall-off, Fuel/Lap)
        let strip_y = hero_y + hero_h + 8.0;
        let strip_h = 30.0f32;
        fill_rounded_rect(&mut img, x0, strip_y, col_w, strip_h, 6.0, panel_bg);
        stroke_rounded_rect(&mut img, x0, strip_y, col_w, strip_h, 6.0, 1.0, border_subtle);

        let std_dev_str = if entry.stint.std_dev() > 0.0001 {
            format!("Consistency σ: {:.3}s", entry.stint.std_dev())
        } else {
            "Consistency σ: —".to_string()
        };
        let drop_val = entry.stint.drop_off();
        let drop_str = if drop_val.abs() > 0.0001 {
            let sign = if drop_val >= 0.0 { "+" } else { "" };
            format!("Fall-off: {}{:.3}s", sign, drop_val)
        } else {
            "Fall-off: —".to_string()
        };
        let fuel_rate = entry.stint.fuel_per_lap();
        let fuel_str = if entry.is_player && fuel_rate > 0.0005 {
            format!("Fuel/Lap: {:.3} {}", fuel_rate * fuel_conv, fuel_unit)
        } else {
            "Fuel/Lap: —".to_string()
        };
        let strip_line = format!("{}    ·    {}    ·    {}", std_dev_str, drop_str, fuel_str);
        draw_text(&mut img, &font_regular, x0 + 12.0, strip_y + 7.0, 12.0, text_muted, &strip_line);

        // C. Rolling Averages Grid (Avg 5, 10, 15, 20, 25, 30)
        let avg_header_y = strip_y + strip_h + 14.0;
        draw_text(&mut img, &font_bold, x0, avg_header_y, 11.5, text_muted, "ROLLING AVERAGES");

        let avg_grid_y = avg_header_y + 22.0;
        let avg_cell_gap = 8.0f32;
        let avg_cell_w = (col_w - avg_cell_gap * 2.0) / 3.0;
        let avg_cell_h = 40.0f32;

        for (b_idx, &bucket) in buckets.iter().enumerate() {
            let r = b_idx / 3;
            let c = b_idx % 3;
            let cx = x0 + c as f32 * (avg_cell_w + avg_cell_gap);
            let cy = avg_grid_y + r as f32 * (avg_cell_h + 8.0);

            fill_rounded_rect(&mut img, cx, cy, avg_cell_w, avg_cell_h, 7.0, card_bg);
            stroke_rounded_rect(&mut img, cx, cy, avg_cell_w, avg_cell_h, 7.0, 1.0, border_subtle);

            let lbl = format!("AVG {}", bucket);
            draw_text(&mut img, &font_bold, cx + 10.0, cy + 12.0, 11.5, text_muted, &lbl);

            let val = entry.stint.avg_first_n(bucket);
            let best_in_bucket = entries
                .iter()
                .map(|e| e.stint.avg_first_n(bucket))
                .filter(|&v| v > 0.0)
                .fold(f32::INFINITY, f32::min);
            let is_bucket_best = is_compare && val > 0.0 && (val - best_in_bucket).abs() < 0.0001;

            let val_str = if val > 0.0 { format_time(val) } else { "—".to_string() };
            let val_col = if is_bucket_best {
                status_green
            } else if val > 0.0 {
                text_white
            } else {
                text_muted
            };
            draw_text_right(&mut img, &font_bold, cx + avg_cell_w - 10.0, cy + 11.0, 14.0, val_col, &val_str);
        }

        // D. Post-Stint 4-Corner Tires (2x2 Side-by-Side: Fronts on top row, Rears on bottom row)
        draw_text(
            &mut img,
            &font_bold,
            x0,
            tire_section_top,
            11.5,
            text_muted,
            "POST-STINT TIRE WEAR & TEMPERATURES (FRONTS / REARS)",
        );

        let tires_y = tire_section_top + 22.0;
        let tire_gap = 10.0f32;
        let tire_w = (col_w - tire_gap) * 0.5;
        let tire_h = 92.0f32;

        let empty_tires = StintTireSet::default();
        let has_tires = entry.is_player && entry.stint.tires.has_data && entry.stint.laps_completed > 0;
        let t_set = if has_tires { &entry.stint.tires } else { &empty_tires };

        let corners: [(&str, &TireCornerData, usize, usize); 4] = [
            ("LF · Left Front", &t_set.lf, 0, 0),
            ("RF · Right Front", &t_set.rf, 1, 0),
            ("LR · Left Rear", &t_set.lr, 0, 1),
            ("RR · Right Rear", &t_set.rr, 1, 1),
        ];

        for (title, data, gx, gy) in corners {
            let tx = x0 + gx as f32 * (tire_w + tire_gap);
            let ty = tires_y + gy as f32 * (tire_h + tire_gap);
            draw_tire_card_on_image(
                &mut img,
                &font_regular,
                &font_bold,
                tx,
                ty,
                tire_w,
                tire_h,
                title,
                data,
                is_metric,
                primary_gold,
                text_white,
                text_muted,
            );
        }
    }

    Ok(img)
}

fn draw_tire_card_on_image(
    img: &mut RgbaImage,
    _font_regular: &FontRef,
    font_bold: &FontRef,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    corner_title: &str,
    data: &TireCornerData,
    use_metric: bool,
    primary_gold: Color,
    text_white: Color,
    text_muted: Color,
) {
    let avg_wear = if data.wear_m > 0.01 {
        (data.wear_l + data.wear_m + data.wear_r) / 3.0
    } else {
        0.0
    };

    let wear_color = if avg_wear >= 90.0 {
        Color::rgb(75, 225, 110)
    } else if avg_wear >= 80.0 {
        primary_gold
    } else if avg_wear >= 70.0 {
        Color::rgb(255, 165, 50)
    } else if avg_wear > 0.0 {
        Color::rgb(255, 75, 75)
    } else {
        text_muted
    };

    let border_col = if avg_wear > 0.0 {
        Color::rgba(wear_color.0[0], wear_color.0[1], wear_color.0[2], 95)
    } else {
        Color::rgba(255, 255, 255, 22)
    };

    fill_rounded_rect(img, x, y, w, h, 9.0, Color::rgb(25, 29, 38));
    stroke_rounded_rect(img, x, y, w, h, 9.0, 1.2, border_col);

    let press_str = if data.pressure > 0.1 {
        if use_metric {
            format!("{:.2} bar", data.pressure / 100.0)
        } else {
            format!("{:.1} psi", data.pressure * 0.145038)
        }
    } else {
        "— psi".to_string()
    };

    let is_left = corner_title.starts_with("LF") || corner_title.starts_with("LR");
    let zone_label = if is_left { "O·M·I" } else { "I·M·O" };

    let title_w = draw_text(img, font_bold, x + 10.0, y + 8.0, 13.0, text_white, corner_title);
    if avg_wear > 0.0 {
        draw_text(
            img,
            font_bold,
            x + 16.0 + title_w,
            y + 8.0,
            13.0,
            wear_color,
            &format!("{:.0}%", avg_wear),
        );
    }
    draw_text_right(img, font_bold, x + w - 10.0, y + 9.0, 11.5, text_muted, &press_str);

    // Temps
    let temp_str = if data.temp_m > 1.0 {
        if use_metric {
            format!("{:.0}° · {:.0}° · {:.0}°C", data.temp_l, data.temp_m, data.temp_r)
        } else {
            format!(
                "{:.0}° · {:.0}° · {:.0}°F",
                data.temp_l * 1.8 + 32.0,
                data.temp_m * 1.8 + 32.0,
                data.temp_r * 1.8 + 32.0
            )
        }
    } else {
        "— · — · —".to_string()
    };
    draw_text(
        img,
        font_bold,
        x + 10.0,
        y + 31.0,
        9.5,
        text_muted,
        &format!("TEMPS ({})", zone_label),
    );
    draw_text(img, font_bold, x + 10.0, y + 44.0, 13.5, text_white, &temp_str);

    // Wear
    let wear_str = if data.wear_m > 0.01 {
        format!("{:.0}% · {:.0}% · {:.0}%", data.wear_l, data.wear_m, data.wear_r)
    } else {
        "—% · —% · —%".to_string()
    };
    draw_text(
        img,
        font_bold,
        x + 10.0,
        y + 62.0,
        9.5,
        text_muted,
        &format!("WEAR ({})", zone_label),
    );
    draw_text(img, font_bold, x + 10.0, y + 74.0, 13.5, wear_color, &wear_str);
}

pub fn encode_rgba_to_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    img.write_to(&mut cursor, image::ImageOutputFormat::Png)
        .map_err(|e| format!("PNG encode failed: {}", e))?;
    Ok(png_bytes)
}

pub fn encode_rgba_to_cf_dib(img: &RgbaImage) -> Vec<u8> {
    let width = img.width();
    let height = img.height();
    let pixel_bytes_len = (width * height * 4) as usize;
    let mut dib = Vec::with_capacity(40 + pixel_bytes_len);

    // BITMAPINFOHEADER (40 bytes)
    dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
    dib.extend_from_slice(&(width as i32).to_le_bytes()); // biWidth
    dib.extend_from_slice(&(height as i32).to_le_bytes()); // biHeight (positive = bottom-up DIB)
    dib.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    dib.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB (0)
    dib.extend_from_slice(&(pixel_bytes_len as u32).to_le_bytes()); // biSizeImage
    dib.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
    dib.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
    dib.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    dib.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

    // Bottom-up BGRA pixels
    for y in (0..height).rev() {
        for x in 0..width {
            let p = img.get_pixel(x, y);
            dib.push(p[2]); // B
            dib.push(p[1]); // G
            dib.push(p[0]); // R
            dib.push(p[3]); // A
        }
    }

    dib
}

pub fn copy_rgba_image_to_clipboard(img: &RgbaImage) -> Result<(), String> {
    let png_bytes = encode_rgba_to_png(img)?;
    let dib_bytes = encode_rgba_to_cf_dib(img);

    let _clip = clipboard_win::Clipboard::new_attempts(10)
        .map_err(|e| format!("Could not open Windows clipboard: {}", e))?;
    let _ = clipboard_win::raw::empty();

    if let Some(png_fmt) = clipboard_win::register_format("PNG") {
        let _ = clipboard_win::raw::set_without_clear(png_fmt.get(), &png_bytes);
    }
    clipboard_win::raw::set_without_clear(clipboard_win::formats::CF_DIB, &dib_bytes)
        .map_err(|e| format!("Failed to write image to clipboard: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stint_a() -> StintData {
        let mut s = StintData::default();
        s.stint_number = 1;
        s.laps_completed = 32;
        s.lap_times = vec![
            19.120, 19.045, 19.088, 19.110, 19.142,
            19.165, 19.190, 19.210, 19.225, 19.240,
            19.255, 19.270, 19.285, 19.298, 19.310,
            19.322, 19.335, 19.348, 19.360, 19.375,
            19.388, 19.401, 19.415, 19.428, 19.440,
            19.452, 19.465, 19.474, 19.243, 19.361,
            19.323, 19.302,
        ];
        s.fast_lap = 19.045;
        s.last_lap = 19.302;
        s.fuel_start = 45.0;
        s.fuel_used = 18.2;
        s.latched_fuel_burn_rate = 0.568;
        s.setup_name = "Bristol_Race_SetA".to_string();
        s.track_temp_at_start = "85.1 F".to_string();
        s.air_temp_at_start = "72.4 F".to_string();
        s.tires = StintTireSet {
            has_data: true,
            lf: TireCornerData {
                temp_l: 78.3,
                temp_m: 71.7,
                temp_r: 58.9,
                wear_l: 92.0,
                wear_m: 94.0,
                wear_r: 100.0,
                pressure: 137.9,
            },
            rf: TireCornerData {
                temp_l: 112.8,
                temp_m: 108.9,
                temp_r: 110.0,
                wear_l: 82.0,
                wear_m: 81.0,
                wear_r: 85.0,
                pressure: 206.8,
            },
            lr: TireCornerData {
                temp_l: 74.0,
                temp_m: 70.0,
                temp_r: 65.0,
                wear_l: 95.0,
                wear_m: 96.0,
                wear_r: 98.0,
                pressure: 131.0,
            },
            rr: TireCornerData {
                temp_l: 108.0,
                temp_m: 104.5,
                temp_r: 102.0,
                wear_l: 84.0,
                wear_m: 85.0,
                wear_r: 88.0,
                pressure: 199.9,
            },
        };
        s
    }

    fn sample_stint_b() -> StintData {
        let mut s = sample_stint_a();
        s.stint_number = 2;
        s.fast_lap = 19.012;
        s.last_lap = 19.340;
        for (i, t) in s.lap_times.iter_mut().enumerate() {
            if i < 15 {
                *t -= 0.032;
            } else {
                *t += 0.018;
            }
        }
        s.setup_name = "Bristol_Race_SetB_LowerRR".to_string();
        s.tires.lf.wear_l = 94.0;
        s.tires.lf.wear_m = 95.0;
        s.tires.lf.wear_r = 100.0;
        s.tires.rf.wear_l = 84.0;
        s.tires.rf.wear_m = 84.0;
        s.tires.rf.wear_r = 87.0;
        s.tires.rr.wear_l = 86.0;
        s.tires.rr.wear_m = 87.0;
        s.tires.rr.wear_r = 89.0;
        s
    }

    #[test]
    fn test_render_discord_cards() {
        let mut state = WarRoomState::default();
        state.track_name = "Bristol Motor Speedway".to_string();
        state.car_name = "NASCAR Cup Series Next Gen".to_string();
        state.session_type = "Practice".to_string();
        state.track_state = "Moderate Rubber".to_string();
        state.weather_type = "Realistic".to_string();
        state.track_temp_raw = "86.0 F".to_string();
        state.air_temp_raw = "73.0 F".to_string();
        state.wind_speed_raw = "6.0 mph".to_string();
        state.wind_dir_raw = "NW".to_string();
        state.relative_humidity_raw = "48%".to_string();

        let mut settings = AppSettings::default();
        settings.use_metric = false;
        settings.active_avg_buckets = vec![5, 10, 15, 20, 25, 30];

        let stint_a = sample_stint_a();
        let stint_b = sample_stint_b();

        let single_entries = [DiscordStintEntry {
            slot_label: None,
            driver_name: "James Grantham",
            car_num: "9",
            setup_name: &stint_a.setup_name,
            is_player: true,
            stint: &stint_a,
            accent_rgb: [245, 184, 46],
        }];
        let single_img = render_discord_stint_card(&single_entries, &state, &settings).expect("render single card");
        assert!(single_img.width() > 500 && single_img.height() > 500);

        let compare_entries = [
            DiscordStintEntry {
                slot_label: Some("STINT A"),
                driver_name: "James Grantham",
                car_num: "9",
                setup_name: &stint_a.setup_name,
                is_player: true,
                stint: &stint_a,
                accent_rgb: [0, 210, 255],
            },
            DiscordStintEntry {
                slot_label: Some("STINT B"),
                driver_name: "James Grantham",
                car_num: "9",
                setup_name: &stint_b.setup_name,
                is_player: true,
                stint: &stint_b,
                accent_rgb: [255, 175, 40],
            },
        ];
        let compare_img = render_discord_stint_card(&compare_entries, &state, &settings).expect("render compare card");
        assert!(compare_img.width() > 900 && compare_img.height() > 500);

        let png = encode_rgba_to_png(&single_img).expect("encode png");
        assert!(png.len() > 1000);
        let dib = encode_rgba_to_cf_dib(&single_img);
        assert_eq!(dib.len(), 40 + (single_img.width() * single_img.height() * 4) as usize);

        if let Ok(out_dir) = std::env::var("DISCORD_CARD_PREVIEW_DIR") {
            let dir = std::path::PathBuf::from(out_dir);
            let _ = std::fs::create_dir_all(&dir);
            single_img.save(dir.join("discord_single_stint_preview.png")).unwrap();
            compare_img.save(dir.join("discord_compare_stints_preview.png")).unwrap();
        }
    }
}

