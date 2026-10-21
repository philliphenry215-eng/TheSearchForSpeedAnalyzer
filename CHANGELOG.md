# Changelog — The Search For Speed Analyzer

All notable changes to **The Search For Speed Analyzer** will be documented in this file.
The project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (`MAJOR.MINOR.PATCH`).

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
