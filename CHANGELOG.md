# Changelog — The Search For Speed Analyzer

All notable changes to **The Search For Speed Analyzer** will be documented in this file.
The project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (`MAJOR.MINOR.PATCH`).

---

## [0.2.3] — 2026-10-21

### Added
* **Live In-Car Air/Track Temperature & Wind Direction HUD Overlay:**
  * **Compact Dual-Temp Readout:** Added `Air/Track Temp: 72/82` (or `22/28` in Celsius) directly into the overlay status bar, styled in high-contrast gold label and cyan temperature figures with rich hover tooltips.
  * **Dynamic Wind Direction Compass:** Added a circular vector gauge widget featuring an outer ring, live wind velocity integer in the center (e.g. `6 mph` / `10 km/h`), and a directional pointer arrowhead oriented along the true telemetry heading.
  * **Modular HUD Customization:** Configured an independent `"Track Temp & Wind"` visibility checkbox in Settings (`OverlaySettings.show_weather`) so racers can easily toggle the module.
  * **Main Header Dashboard Compass:** Added the mini circular wind indicator to the main app header conditions pill for consistent environmental awareness across both desktop and overlay views.
* **Cockpit Engineer Best / Fastest Lap Metric:** Added "Best lap" right beneath "Last lap" in Column 1's Fuel & Stint card, styled in high-visibility status green with live stint and session fallback.

### Fixed & Improved
* **Tire Data Garage Synchronization:** Integrated direct extraction of iRacing's definitive garage tire data (`CarSetup`) from the session string, guaranteeing exact 1-to-1 matches with the in-game iRacing garage screen for last hot pressures, carcass temperatures, and tread wear percentages (LF: `12.1 psi` · `194° · 187° · 164°F` · `86% · 86% · 100%`).
* **Preserved Native Tire Ordering:** Maintained the canonical telemetry reading order (`temp_l · temp_m · temp_r` and `wear_l · wear_m · wear_r`), which naturally maps to `O·M·I` for Left tires and `I·M·O` for Right tires exactly as shown in the iRacing garage.
* **Zero Estimation Policy:** Completely eliminated ideal gas law math and pressure estimation formulas. Stint tires now exclusively report ground-truth telemetry values directly measured by iRacing.
* **Accurate Pit Stall Latching:** Prevented premature tire latching at the pit entry line while moving at speed. Stint tires are now latched when stationary in the pit stall following crew service or upon returning to the garage.
* **Isolated Stint Data:** Removed unserviced stint fallback to previous runs. Stints without pit tire service (such as 1-lap runs or drive-throughs) now cleanly display empty dashes (`"—% · —% · —%"`, `"— psi"`) with clear `"NO PIT SERVICE"` indicators instead of showing wear from another stint.
* **Excel TSV Export Alignment:** Updated all clipboard export functions ("Copy Temps", "Copy Wear", "Copy 4-Tire Table") to match the garage screen layout for seamless analysis in spreadsheets.

---

## [0.2.1] — 2026-10-20

### Fixed & Improved
* **Tire Temps & Wear Single-Line Fix:** Widened Column 3 (`345px` min), reduced button padding, and enforced non-breaking single-line layout (`.wrap(false)`) so tire temperatures and wear percentages (`100% · 100% · 100%`) never wrap onto multiple rows.
* **15.5pt In-Car Overlay Sub-Labels:** Increased overlay secondary font size to bold 15.5pt for effortless glanceability from your sim cockpit.
* **Trimmed Bottom HUD Space:** Eliminated ~40px of bottom dead space in the in-car overlay, creating clean, uniform 8px borders around all cards without any empty space.

---

## [0.2.0] — 2026-10-20

### Added
* **Customizable In-Car Overlay System:**
  * Modular section visibility toggles in Settings: independently show or hide `Pace & Stint`, `Fuel & Strategy`, and `Race Control & Flags`.
  * **Primary Fuel Stat Focus:** Choose between **Fuel in Tank (Gallons / Liters)** as the primary large hero metric or **Laps to Empty**.
  * **HUD Scale Slider:** Smooth scaling from **75% to 150%** for single-screen, triple-monitor, ultrawide, and VR rigs.
  * **Live HUD Preview:** Toggle overlay preview in Settings to position and scale without needing to be on track.
* **Enhanced In-Car Legibility:**
  * Enlarged sub-labels to 13pt and header labels to 11.5pt with display hero font (21.5pt) for rapid glanceability at speed.
  * Tightened row spacing for authentic cockpit digital dash appearance.
  * Adaptive window height with generous buffer ensuring zero clipping across all screen resolutions and scales.

---

## [0.1.4] — 2026-10-20
* Instant first-frame telemetry attach on Lap 0 pit exit and mid-run session join.
* Compact, centered 580x620 "What's New" release notes modal with smart word wrapping.

## [0.1.3] — 2026-10-19
* Direct shared-memory YAML parser for resilient driver names, car numbers, and iRatings.
* Seamless 1-click in-app background auto-updater with in-place restart.

## [0.1.2] — 2026-10-19
* Background downloading with real-time progress bar.

## [0.1.1] — 2026-10-19
* Session phase isolation (Practice ➔ Qualify ➔ Race) with clean stint and lap resets.
* Stint 1 setup name detection and baseline fallbacks.

---

## [0.1.0] — 2026-10-18 (Official Production Baseline)

### 🏁 What is The Search For Speed Analyzer?
The Search For Speed Analyzer is a lightweight, local-first telemetry war room and floating live HUD engineered for competitive iRacing drivers. It runs directly on your sim rig with near-zero CPU overhead, providing real-time stint pace analysis, dynamic tire wear tracking, and race-critical fuel strategy.

### Added
* **Leader Last Lap & Live Pace Differential (Δ) in Overlay HUD:**
  * Compares your last completed lap against the current race leader's last lap.
  * Formats delta cleanly (`-0.335s` green if faster, `+0.420s` amber if slower).
  * Automatically switches to compare against P2 (`DELTA VS P2 · Pulling away` / `P2 closing`) when leading the race.
* **Responsive 70/30 Fuel Rolling Average:**
  * Blends 70% weight on recent 5 laps and 30% stint base for rapid adaptation to fuel saving or green-flag pace shifts.
* **Windows Application Icon Embedding:**
  * Embedded high-resolution `.ico` into executable and taskbar using `winres`.
* **Inno Setup Windows Installer:**
  * Clean non-admin installer with optional Desktop and Start Menu shortcuts.
* **Color Themes:**
  * Added **S4S Gold & Blue (Dark)**, **S4S Daylight (Light Mode)**, and **Stealth Monochrome**.
* **In-App Updater:**
  * Integrated background update checker that polls GitHub Releases without requiring any third-party accounts or external services.

### Fixed
* **Fuel Mileage Out-Lap Fix:**
  * Solved SDK pit exit calculation bug where partial out-lap fuel consumption (~0.8 L) was treated as a full race lap, corrupting stint averages for 10 laps. Fuel is now latched at start/finish line crossing and accurate from Lap 1.
* **Updater 404 & Pre-Release Comparison:**
  * Switched update endpoint to list releases instead of `/releases/latest` to cleanly handle pre-releases and compare version components.

---

## [0.1.0-beta.3] — 2026-10-18
* Fixed pre-release SemVer comparison in update checker (`beta.3` vs `beta.2`).
* Implemented Leader Last Lap & Delta HUD Section 1.
* Implemented start/finish out-lap fuel latching.

## [0.1.0-beta.2] — 2026-10-18
* Fixed GitHub Releases updater 404 endpoint.
* Added S4S Daylight theme and updated Settings interface.

## [0.1.0-beta.1] — 2026-10-18
* Initial beta release with Inno Setup installer and Windows executable icon embedding.
