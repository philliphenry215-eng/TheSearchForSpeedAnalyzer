# Changelog — The Search For Speed Analyzer

All notable changes to **The Search For Speed Analyzer** will be documented in this file.
The project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (`MAJOR.MINOR.PATCH`).

---

## [0.3.7] — 2026-10-25

### Added & Improved
* **Documents Installation Default & Frictionless Setup:**
  * Default installer directory moved to `{userdocs}\The Search For Speed Analyzer` (`C:\Users\<Username>\Documents\SearchForSpeedAnalyzer`).
  * Eliminated initial "Install for all users vs me only" prompt on startup so setup launches straight into the install wizard.
  * Added `ArchitecturesInstallIn64BitMode=x64compatible` to ensure 64-bit architecture and eliminate `Program Files (x86)` redirection.
  * Completely eliminates Windows UAC administrator elevation requirements and permission errors.
* **Visible User Stints & Data in Documents:**
  * Stint files (`stints/*.s4s.json`), the stint database (`stint_database.json`), and settings (`settings.json`) now default to `C:\Users\<Username>\Documents\SearchForSpeedAnalyzer`.
  * Sits right beside `Documents\iRacing\telemetry` and `Documents\iRacing\setups`—highly visible and easy to share.
  * Added automatic migration from legacy `%APPDATA%\SearchForSpeedAnalyzer` on first launch.

---

## [0.3.6] — 2026-10-25

### Added & Fixed
* **Dual-Mode Setup Installer & Custom Drive Support (`installer.iss`):**
  * Added `PrivilegesRequiredOverridesAllowed=dialog commandline` and auto-constant `{autopf}` support to Inno Setup.
  * Fixed Windows `Error 5: Access is denied` when installing into `C:\Program Files\SearchForSpeedAnalyzer`.
  * Setup now offers both **Install for all users (recommended)** with UAC administrator elevation into `C:\Program Files`, and **Install for me only** into `%LOCALAPPDATA%\Programs`.
  * Full support for installing to custom drives (`D:\`, `E:\`, etc.).
* **Smart In-App Auto-Updater (`src/updater.rs`):**
  * Silent update process passes `/ALLUSERS` with `-Verb RunAs` when installed in `C:\Program Files`, and `/CURRENTUSER` when installed per-user.
* **Session Lap Number Synchronization:**
  * Stint data now tracks exact iRacing session lap numbers (`CarIdxLapCompleted` at line crossing) alongside lap times and outputs them in TSV exports.
  * Added robust handling for identical consecutive lap times and out-lap transitions.
* **UI Font Stack & Branding:**
  * Added font fallback definitions preventing missing `?` glyphs across system configurations.
  * Standardized branding to **The Search For Speed Analyzer**.

---

## [0.3.5] — 2026-10-24

### Added & Improved
* **Local Stint Database & Real-Time Auto-Save:**
  * Completed stints (`1+` green-flag laps) automatically persist to `%APPDATA%\SearchForSpeedAnalyzer\stint_database.json` and individual `.s4s.json` files.
  * Automatic preservation of Practice and Qualifying stints before server transitions.
* **Track-Matched Re-Import & Comparison:**
  * Saved Stints manager with instant filtering to Current Track, fuzzy search, and re-importing into live sessions as reference stints.
* **Lap 0 Historical Fuel Burn Seeding:**
  * Pre-populates fuel burn rate, laps remaining, and pit call on Lap 0 from past runs on the same track and car combo.
* **Discord Stint Code Sharing:**
  * Compact `S4S-STINT-V1:...` share strings and clipboard import/export for teammate sharing.

---

## [0.3.4] — 2026-10-24

### Added & Improved
* **Post-Pit 4-Corner Tire Popup Overlay:**
  * Floating in-car overlay that automatically appears for a configurable duration (`3s–60s`) when new post-pit tire wear, temperatures, and pressures latch.
* **True Transparent Outer HUD Background:**
  * DWM per-pixel alpha compositing with separate Inner Card Opacity and Outer Background Box sliders.
* **Default Startup View Setting:**
  * Added startup view selection in Settings (Cockpit Stint & Fuel Analyzer, Multi-Driver Stint Grid, or Offline .IBT Telemetry Lab).

---

## [0.3.3] — 2026-10-24

### Added & Improved
* **1-Click Discord Stint & Comparison Image Card Export (`📸 Copy Discord Image`):**
  * Added native offscreen graphic card rendering (`discord_card.rs`) for both **Single Stint** and **Side-by-Side Stint Comparison** (1–3 stints).
  * Shared top session banner displays Track, Car, Session Type, Track Rubber State, Weather, Track/Air Temps, Wind Speed & Direction, and Relative Humidity.
  * Per-stint columns display Driver, Car #, Stint #, Laps Completed, Setup Name, Fastest Lap, Last Lap, Full Average, Consistency ($\sigma$), Pace Fall-off, Fuel/Lap, Rolling Averages (`Avg 5`–`Avg 30`), and 2x2 Post-Stint Tire Wear & Temps.
  * Copies both `PNG` and `CF_DIB` clipboard formats directly to the Windows clipboard for instant `Ctrl+V` pasting into Discord.
* **2x2 Side-by-Side Tire Orientation on Stint Comparison:**
  * Updated Stint Comparison tire cards so Fronts (`LF`, `RF`) sit side-by-side on the top row and Rears (`LR`, `RR`) sit side-by-side on the bottom row inside each stint column.
* **Custom Drive (`C:`, `D:`, `E:`) & Admin Elevation Auto-Updater Improvements:**
  * Standalone `.exe` installs on custom drives update directly in place via binary swap, while Setup installs preserve custom target folders via `/DIR=` and request UAC elevation (`-Verb RunAs`) if the folder is write-protected.

---

## [0.3.2] — 2026-10-23

### Fixed & Improved
* **Overlay HUD "RACE REMAIN" Countdown & Race Conclusion Fix:**
  * **Upgraded to `SessionLapsRemainEx` Telemetry:** Switched live telemetry remaining-laps channel from legacy/deprecated `SessionLapsRemain` to `SessionLapsRemainEx`, eliminating iRacing's known off-by-one telemetry calculation bug that caused the display to prematurely announce "1 to go" with 2 laps to go.
  * **Live `SessionLapsTotal` Integration:** Ingested scheduled total race distance (`SessionLapsTotal`) from 60 Hz telemetry and subsession YAML metadata to calculate car-specific remaining laps (`SessionLapsTotal - player_lap`), ensuring individual line crossings accurately tick down remaining laps for the player rather than jumping whenever the leader crosses.
  * **Zero Laps Remaining & Race Finish Propagation:** Corrected `remain >= 0` guard in live telemetry parser so that 0 laps remaining properly updates `session_laps_remain`. Eliminated stuck "1 to go" condition across checkered flag and finish states, transitioning cleanly to `"Finished"`.
  * **Singular/Plural & Finish State Label Formatting:** Formatted remaining lap strings cleanly (`"X laps"`, `"1 lap"`, `"Finished"`), replacing awkward `"1 laps"` or showing `"Unlimited"` after completing a scheduled race.
* **Eliminated Negative Completed Laps ("Lap -1 done"):**
  * Clamped `CarIdxLapCompleted` -1 initialization (emitted by iRacing prior to lap 1 completion during pre-race, grid, and pace laps) to `0` across all driver telemetry models and UI representations.
  * Replaced `"Lap -1 done"` with `"0 laps done"` during warmup and lap 1, ensuring no negative lap counts ever appear on the in-car HUD overlay or header bars.
* **Updater no longer requires administrator rights:**
  * In-app updates now download `*-Setup.exe` to `%TEMP%` and run it silently (`/SILENT /RELAUNCH`), installing per-user to `%LOCALAPPDATA%` and relaunching the app.
  * Legacy raw-exe fallback swaps the binary in place, requesting UAC elevation only when the install folder is not writable, then relaunches unelevated.
  * Update errors are now surfaced in the UI instead of being silently ignored.
* **Settings moved to `%APPDATA%\SearchForSpeedAnalyzer`** with automatic migration of legacy `settings.json` / `settings-beta.json`.
* **Installer:** Beta builds now use a separate AppId and install folder (`SearchForSpeedAnalyzer-Beta`), so beta and stable no longer overwrite each other.

---

## [0.2.5] — 2026-10-22

### Fixed & Improved
* **Relative Wind Direction Vector Math:** Corrected vehicle relative wind angle calculation (`(WindDir - YawNorth + PI)`) to properly honor iRacing's clockwise yaw rotation. Wind arrow orientation now mirrors real vehicle air flow across all angles and turns:
  * Tailwinds point forward at 12 o'clock, headwinds point backward at 6 o'clock.
  * Crosswinds and quartering tailwinds (e.g. 2 o'clock) align seamlessly with Stint Analyzer and telemetry references without vertical reflection or sign flips.
* **1-to-1 Wind Speed Precision Match:** Fixed wind speed telemetry storage to preserve decimal precision (`format!("{:.2} m/s", wv)`), eliminating premature integer rounding that caused the app to read 1 mph higher than iRacing's in-sim Weather tab.
* **Refined Vector Compass Gauge:** Polished the circular wind gauge with an enlarged chevron pointer and crisp border for rapid glanceability at race pace.

---

## [0.2.4] — 2026-10-21

### Added & Improved
* **Dynamic Car-Relative Wind Direction:**
  * **Real-time Vehicle Heading Alignment:** Synchronized wind direction vector math directly against `YawNorth` (60 Hz telemetry orientation) so the gauge tracks wind relative to the car's cockpit nose in real-time as you steer through corners and straights.
  * **Accurate Relative Wind Vectors:** Correctly resolves crosswinds (Left-to-Right = 3 o'clock arrow, Right-to-Left = 9 o'clock arrow), direct headwinds (6 o'clock arrow), and tailwinds (12 o'clock arrow).
  * **Prominent Motorsport Vector Gauge:** Enlarged gauge diameter (`28px`), added a bold chevron pointer with vector shaft and white border, and integrated a gold car-nose reference pip at 12 o'clock.
  * **Descriptive Wind Tooltips:** Hovering over the wind gauge now details relative aerodynamic wind behavior (e.g. `Crosswind (Left → Right) · True Compass: NE (45° rel)`).

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
