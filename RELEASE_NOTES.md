# The Search For Speed Analyzer — v0.2.3

### 🌡️ Air/Track Temperature & Wind Direction Overlay:
* **Air & Track Surface Temp Readout:** Integrated live environmental telemetry into the floating overlay HUD (`Air/Track Temp: 72/82` in Fahrenheit or `22/28` in Celsius) so drivers can monitor track grip loss and ambient conditions without leaving their driving line.
* **Wind Direction Compass Gauge:** Custom-drawn circular vector compass widget with the live wind speed number centered inside and a sharp pointer arrowhead indicating exact wind heading.
* **Modular HUD Toggle:** Added `"Track Temp & Wind"` toggle checkbox in Settings under *Visible HUD Sections*.
* **Integrated HUD & Dashboard Polish:** Seamlessly fits into the in-car overlay status bar with generous hover tooltips detailing exact temperatures and angles (`Wind: 6 mph NE (45°)`), plus a live mini compass in the main application header.
* **Cockpit Engineer Best / Fastest Lap Readout:** Added "Best lap" right beneath "Last lap" in Column 1's Fuel & Stint card, styled in high-visibility status green with live stint and session fallback.

---

# The Search For Speed Analyzer — v0.2.2

### 🛞 Definitive Tire Telemetry & Garage Sync:
* **Direct iRacing Garage Matching:** Integrated direct extraction of iRacing's definitive garage tire data (`CarSetup`) from the session string, guaranteeing exact 1-to-1 matches with the in-game iRacing garage screen for last hot pressures, carcass temperatures, and tread wear percentages (LF: `12.1 psi` · `194° · 187° · 164°F` · `86% · 86% · 100%`).
* **Preserved Native Tire Ordering:** Maintained the canonical telemetry reading order (`temp_l · temp_m · temp_r` and `wear_l · wear_m · wear_r`), which naturally maps to `O·M·I` for Left tires and `I·M·O` for Right tires exactly as shown in the iRacing garage.
* **Zero Estimation Policy:** Completely eliminated ideal gas law math and pressure estimation formulas. Stint tires now exclusively report ground-truth telemetry values directly measured by iRacing.
* **Accurate Pit Stall Latching:** Prevented premature tire latching at the pit entry line while moving at speed. Stint tires are now latched when stationary in the pit stall following crew service or upon returning to the garage.
* **Isolated Stint Data:** Removed unserviced stint fallback to previous runs. Stints without pit tire service (such as 1-lap runs or drive-throughs) now cleanly display empty dashes (`"—% · —% · —%"`, `"— psi"`) with clear `"NO PIT SERVICE"` indicators instead of showing wear from another stint.
* **Excel TSV Export Alignment:** Updated all clipboard export functions ("Copy Temps", "Copy Wear", "Copy 4-Tire Table") to match the garage screen layout for seamless analysis in spreadsheets.

---

# The Search For Speed Analyzer — v0.2.1

### 🛠️ UI Refinements & Styling Fixes:
* **Tire Wear & Temps Line Wrapping Fix:** Widened Column 3 (`345px` min), reduced button padding, and enforced non-breaking single-line layout (`.wrap(false)`) so 3-digit tire temperatures and wear percentages (`100% · 100% · 100%`) never wrap onto multiple rows.
* **15.5pt In-Car Overlay Sub-Labels:** Increased overlay secondary font size to bold 15.5pt for effortless glanceability from your sim cockpit.
* **Trimmed Bottom HUD Space:** Eliminated ~40px of bottom void in the in-car overlay, creating clean, symmetrical 8px borders around all cards without any empty space.

---

# The Search For Speed Analyzer — v0.2.0

### 🏁 Customizable In-Car Overlay & Fuel Sizing:
* **Primary Fuel Stat Focus:** You can now choose what takes center stage in the HUD fuel box! Set your prominent hero figure to **Fuel in Tank (Gallons / Liters)** with large display typography in vibrant accent color, or keep **Laps to Empty** as primary with raw fuel underneath.
* **Modular Section Visibility:** Customize your HUD footprint with independent show/hide toggles in Settings for:
  * **Pace & Stint:** Stint/lap counters, your last/best lap, leader/P2 lap, and live delta.
  * **Fuel & Strategy:** Tank fuel level, laps of fuel, target pit lap, burn rate, and fuel to add on F4.
  * **Race Control & Flags:** Incidents (`X/17x`), caution laps, and session flag indicators.
* **Global HUD Scale Slider:** Scale the entire HUD seamlessly between **75% and 150%** to match single-screen, triple-monitor, ultrawide, or VR sim rigs.
* **Live HUD Preview Mode:** Added a "Show HUD Preview" toggle in Settings to position, scale, and preview your overlay anywhere on your monitors without needing to be in an active iRacing session.

### 🔍 Enhanced In-Car Legibility:
* **Enlarged Secondary & Header Fonts:** Sub-labels (laps left, averages, fuel burn rate, pit window) have been bumped to a crisp, high-visibility 13pt font for instant readability in milliseconds at race pace.
* **Motorsport-Grade Spacing:** Tightened internal column item spacing for an authentic, compact MoTeC/Bosch digital dash appearance with zero wasted vertical space.
* **Zero Cutoff Layout:** Window dimensions automatically resize based on visible modules and scale with generous bottom cushioning so no text, cards, or rounded borders ever clip.

---

# The Search For Speed Analyzer — v0.1.4

### 🏎️ Instant First-Frame Telemetry & Mid-Run Attach:
* **Immediate Lap 0 Pit Exit Detection:** Fixed an issue where launching the app while sitting in the pit or garage on Lap 0 could clear the pit exit transition check, preventing Stint 1 from starting until the app was closed and reopened.
* **Instant Mid-Run Attach:** Launching the app while already driving out on track now immediately recognizes the active session and creates Stint 1 at your current lap, recording telemetry and fuel with zero missed laps.

### 📖 Release Notes & "What's New" Window Overhaul:
* **Compact, Centered Layout:** Redesigned the "What's New" dialog into a fixed, centered 580x620 modal that never stretches horizontally off-screen.
* **Smart Word Wrapping:** Added automatic text wrapping across all bullet points and descriptions, completely eliminating horizontal scrolling.
* **Always-Visible Controls:** The "Done" button is permanently pinned within the visible window at the bottom right.

---

# The Search For Speed Analyzer — v0.1.3

### 🏎️ Driver Roster & Stint Grid Restoration:
* **Direct Shared-Memory YAML Parser:** Replaced the third-party SDK session parser with a custom direct memory-mapped ISO-8859-1 YAML reader. This eliminates crashes caused by unpopulated subsession result schemas (`ResultsPositions: ~`) and permanently restores full driver names, car numbers, and iRatings across all stint grids and HUD views.
* **Resilient Driver Name Population:** Grid rows no longer fall back to placeholder `Car #1, Car #2` names when active sessions load. Real competitor names and numbers populate immediately.

### ⚡ Seamless 1-Click In-App Updater:
* **Direct Background Downloading:** Clicking "Update Now ⚡" downloads updates directly inside the app with a real-time progress bar.
* **No More Setup Wizards or Manual Downloads:** Updates are applied in place automatically with zero Inno Setup wizard dialogs and without redirecting to GitHub in your browser.
* **Instant In-Place Restart:** A single click on "Restart to Apply 🔄" restarts the app immediately into the new version.
* **Update from Release Notes:** When reading "What's New 📖", you can now trigger the update directly from the release notes dialog.

---

# The Search For Speed Analyzer — v0.1.2

### ⚡ Seamless 1-Click In-App Updater:
* **Direct Background Downloading:** Clicking "Update Now ⚡" now downloads update packages directly inside the app with a real-time progress bar.
* **No More Setup Wizards or Manual Downloads:** Updates are applied in place automatically with zero Inno Setup wizard dialogs and without redirecting to GitHub in your browser.
* **Instant In-Place Restart:** A single click on "Restart to Apply 🔄" restarts the app immediately into the new version.
* **Update from Release Notes:** When reading "What's New 📖", you can now trigger the update directly from the release notes dialog.

---

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
