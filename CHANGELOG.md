# Changelog — The Search For Speed Analyzer

User-facing milestone history for **The Search For Speed Analyzer** ([Semantic Versioning](https://semver.org/spec/v2.0.0.html)).  
For detailed notes on the latest builds, see [RELEASE_NOTES.md](RELEASE_NOTES.md) or the GitHub Releases page.

---

## [0.5.x] — Tire Telemetry Precision, Interactive .IBT Lab HUD & Zoom Controls *(v0.5.0 – v0.5.12)*

* **Exact iRacing Garage & Black-Box Tire Matching:** Locked post-stint tire wear, hot pressures, and carcass/pyrometer temperatures cleanly to the stint that just finished—including fuel-only pit stops, post-exit live SDK / fast-poll capture, unworn `100%` corners (such as LF on Dirt Late Models), and short runs with real hot temperatures—while protecting saved stints from cold setup resets or aborted out-laps.
* **Locked Cockpit Tire Panel:** Fixed Cockpit `TIRES` to always show your latest live post-pit or garage tire readout cleanly (and stay blank on fresh sessions before your first stop).
* **Pinned Top Telemetry HUD in `.IBT Lab`:** Added an interactive, aspect-preserving 2D Circuit Track Map (with `↺ 90°` rotation and click/drag cursor scrubbing) paired with a motorsport-standard G-G Traction Friction Circle.
* **MoTeC / Pi Persistent Channel Stats:** Added live `Cur`, `Min`, `Max`, `Avg`, and Lap B `Δ` delta banners across all synchronized 60Hz `.ibt` trace lanes with jitter-free monospace formatting.
* **Drag-to-Zoom on Stint Lap Graphs:** Added interactive box-select zoom and double-click reset on both Single Stint and Compare Stints lap-progression charts.
* **Immediate `.sto` Setup Sync:** Renaming or saving a stint's `.sto` setup file immediately updates the active session, Stint Analyzer, Discord share cards, and local database records.
* **4-Corner Edge Cases & Non-Slip HUD Polish:** Supported `0%` corded tire bands, `LR`/`RR` rear-only readouts, Celsius temps over `120°C`, and locked `LeftPillar` HUD fuel pills cleanly without clipping.

---

## [0.4.x] — Daylight Theme, Custom HUD Layouts, Setup Sharing & Scaling *(v0.4.0 – v0.4.5)*

* **5 Selectable In-Car HUD Overlay Layouts:** Added `CompactSingle`, `LeftPillar`, `CompactVertical`, `TwoRowCard`, and `PitStopFocus` with a locked anti-jitter single-line option and live customization.
* **Daylight / Light Mode Overhaul:** Rebuilt Light Mode palette and contrast ratios (including theme-aware Discord card exports) for high visibility on bright monitors.
* **Native `.sto` Setup Resolution & Explorer Export:** Automatically links your iRacing `.sto` setup file to each stint, embeds it into shared `.s4s.json` stint files, and lets you save/rename `.sto` setups right in Windows File Explorer.
* **1-Click Teammate Stint File Sharing:** Export, share (`Ctrl+V` in Discord), and import native `.s4s.json` stint packages alongside full teammate setup matching.
* **Text Size Presets & Responsive Scaling:** Added adjustable UI text presets (`Small`, `Normal`, `Large`, `Extra Large`) that scale panels, overlays, and modals cleanly without overlapping controls.
* **Pit-Exit Tire Report & Session Detection:** Fixed out-lap S/F crossing capture, arming the post-pit 4-corner tire report reliably on pit exit while automatically hiding HUD previews when stepping out of the car.

---

## [0.3.x] — Offline `.IBT Telemetry Lab`, Stint Database & Discord Cards *(v0.3.0 – v0.3.9)*

* **`.IBT Telemetry Lab` Engine:** Built a full offline binary `.ibt` analyzer covering `Inputs & Speed`, `Ride Height & Rake` (including NASCAR NextGen calibrations), `Shock Histograms`, `Tires & Pressures`, and CSV/channel inspection.
* **Cross-File Lap Comparison:** Overlay Lap A vs. Lap B from different `.ibt` sessions with track-and-car compatibility checks and live time/telemetry deltas.
* **Persistent Stint Database (`Documents\SearchForSpeedAnalyzer`):** Auto-saves player stints locally with track-matched re-import (`IMP`) so you can benchmark live runs against your best historical or teammate stints.
* **1-Click Discord Stint Cards:** Render crisp PNG stint summary cards—single stint or side-by-side comparison—directly to your Windows clipboard (`Ctrl+V` ready).
* **Post-Pit Tire Popup Overlay:** Displays an automatic 4-corner tire wear, temperature, and hot pressure popup on pit exit with configurable dual-layer HUD transparency.
* **Frictionless Windows Installer & Auto-Updater:** Added a native Win32 background updater and dual-mode installer (Documents / custom drives) supporting both portable EXEs and Setup installs.

---

## [0.2.x] — Multi-Stint Comparison, Relative Wind Dial & Weather HUD *(v0.2.0 – v0.2.6)*

* **Side-by-Side Stint Comparison Modal:** Compare up to 3 stints—player vs. rivals or historical imported stints—across lap pace, falloff, fuel burn, and 4-corner tire degradation.
* **Car-Relative Wind Vector Dial:** Live dynamic wind arrow and speed indicator rotated relative to your car's heading around the track, paired with air/track temperature readouts.
* **Customizable Floating Race Overlay:** Configurable live fuel, laps remaining, pace/leader delta (`Δ`), and stint progress overlay for in-car driving.
* **Excel Copy & TSV Exports:** 1-click clipboard copy of formatted lap tables and 4-corner tire wear/temp sheets ready to paste straight into spreadsheets.

---

## [0.1.x] — Live iRacing Telemetry Foundation *(v0.1.0 – v0.1.4)*

* **Live 60Hz iRacing Shared Memory Engine:** Real-time driver roster, standings grid, green/caution/pit stint segmentation, and lap-by-lap fuel burn strategy accurate from Lap 1.
* **Automatic Session Transitions:** Clean resets across Practice, Qualifying, and Race phases while archiving prior session stints and handling mid-run app launches seamlessly.
* **Cockpit, Standings Grid & In-App Updater:** Established the dark low-glare UI theme for sim rigs alongside one-click GitHub Release update checks.
