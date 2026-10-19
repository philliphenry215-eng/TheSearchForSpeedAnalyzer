# The Search For Speed Analyzer — v0.1.1

### 🛠️ Bug Fixes & Improvements:

#### 1. Session Phase Transitions & Lap Reset (Practice ➔ Qualify ➔ Race)
* **Clean Session Isolation:** Moving from Practice to Qualifying to Race now resets active driver lap counters, stints, caution states, and fast laps cleanly. Qualifying flyer laps no longer bleed into your race session fast lap.
* **Historical Baseline Fuel Retention:** Fuel burn rates calculated in previous sessions are preserved as an instant strategy baseline until active race laps are logged.
* **Session History Archiving:** Completed session stints are archived in memory for historical reference.

#### 2. Stint 1 Setup Name Fix
* **Setup Display Across Stints:** Fixed an issue where Stint 1 erroneously displayed the car name rather than the loaded setup name. Setup names are now correctly captured and back-filled from garage YAML data.
* **Universal Neutral Fallback:** Cleaned `.sto` file extensions and directory paths across all stints and grid table rows, defaulting cleanly to `"Baseline"` rather than the car model name.

#### 3. Real-Time Track State in Header
* **Session Track State Display:** The top header bar's session details pill now includes the live track condition (e.g. `Clean Track`, `Moderate Rubber`, `Heavy Rubber`) along with live wetness states (`Light Wet`, `Moderate Wet`, `Very Wet`).

---

# The Search For Speed Analyzer — v0.1.0 (Official Production Baseline)

### 🏁 What is The Search For Speed Analyzer?
The Search For Speed Analyzer is a lightweight, local-first telemetry war room and floating live HUD engineered for competitive iRacing drivers. It runs directly on your sim rig with near-zero CPU overhead, providing real-time stint pace analysis, dynamic tire wear tracking, and race-critical fuel strategy.

---

### 🚀 Highlights in v0.1.0:

#### 1. Real-Time Fuel Mileage & Pit Strategy (Accurate from Lap 1)
* **Out-Lap Fuel Fix:** Solved the SDK pit exit calculation bug. Out-lap crossings at start/finish now cleanly latch crossing fuel without recording partial out-lap burn into racing averages.
* **Responsive 70/30 Rolling Average:** Calculates fuel-per-lap using 70% weight on recent laps and 30% stint base, adapting immediately to race pace and fuel-saving.
* **In-Car Pit Strategy:** Real-time "Laps to Empty", "Pit Window Target Lap", and "Fuel To Add (F4)" calculations factoring your custom safety margin.

#### 2. Floating Overlay HUD with Leader Differential (Δ)
* **Live Pace Comparison:** Section 1 displays your last lap vs. the race leader's last lap with a high-visibility differential (`-0.335s` green if faster, `+0.420s` amber if slower).
* **Adaptive P1 / P2 Switch:** When leading the race, the HUD automatically switches to monitor P2 (`DELTA VS P2 · Pulling away` / `P2 closing`).
* **Customizable Overlay:** Supports Square, Horizontal, and Vertical shapes with drag-and-drop screen positioning, click-through toggles, and opacity control.

#### 3. Custom Themes & High-Visibility UI
* **S4S Gold & Blue (Dark):** Signature dark theme with deep navy card fills and vibrant gold accents.
* **S4S Daylight (Light Mode):** Clean, high-contrast light theme engineered for daytime racing in bright rooms.
* **Stealth Monochrome:** High-performance high-contrast theme for focused sim cockpits.

#### 4. Native Windows Integration & In-App Updater
* **Inno Setup Installer:** Automatic Windows Start Menu and Desktop shortcuts without requiring administrator prompts.
* **Embedded Icon:** Custom application branding embedded directly into the Windows executable and taskbar.
* **Local-First In-App Updater:** Automatic Semantic Versioning (`MAJOR.MINOR.PATCH`) checks GitHub Releases with zero telemetry and zero third-party accounts.
