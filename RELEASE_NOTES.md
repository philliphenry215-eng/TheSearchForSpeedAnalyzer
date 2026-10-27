# The Search For Speed Analyzer — v0.3.9

### 🛡 Windows Defender & Antivirus False-Positive Hardening:
* **Embedded Application Manifest & Metadata:**
  * Embedded a formal Windows application manifest (`asInvoker`) explicitly declaring to Windows Defender and SmartScreen that the application runs safely with standard user privileges and requires zero elevation.
  * Added complete Windows VersionInfo metadata (Company, Product Name, Description, Copyright, Numeric Version) to both the compiled executable (`build.rs`) and the setup installer (`installer.iss`).

### ⚙ Direct Win32 Updater Pipeline (Eliminated Hidden PowerShell):
* **Native Process Execution:**
  * Fully replaced hidden background PowerShell execution (`powershell.exe -WindowStyle Hidden -EncodedCommand`) in the auto-updater with native Win32 `ShellExecuteW` and direct process spawning.
  * Eliminates the single largest behavioral trigger for antivirus heuristic scanners.
  * Seamlessly integrates with Windows Restart Manager via Inno Setup flags (`/CLOSEAPPLICATIONS` and `/RELAUNCH`) so updating in place requires no external script wrappers.
  * Hardened URL opening logic to strictly validate HTTP/HTTPS links before passing to the system shell.

---

# The Search For Speed Analyzer — v0.3.8

### 📊 Cross-File Telemetry Comparison (.IBT Lab):
* **Dual-Session File Browser & Tagging:**
  * The left `.ibt` browser allows loading files as either **Session A (Primary)** or **Reference B (Secondary)**.
  * Files in the list display distinctive status tags: `[A]` (Gold) and `[B]` (Electric Blue).
  * Direct `Ref B` button on each file row, right-click context menu options (*Load as Primary Session A*, *Load as Reference Session B*), and an active Reference Session status pill with 1-click `[Swap A / B]` and `[× Unload Ref]`.
* **Cross-Session Lap Slicing & Overlays:**
  * Card 3 lap selector enables selecting any lap from Reference Session B to compare directly against Session A.
  * Distance-normalized overlay charts display $\Delta t$ time delta, speed comparisons, longitudinal rake, and side rake side-by-side.

### 🛡 Track & Vehicle Platform Compatibility with NASCAR Spec Series Exemptions:
* **Circuit & Platform Validation:**
  * Compares track display names and vehicle models between files to safeguard telemetry overlays.
  * Telemetry comparison requires the **same circuit** and the **same vehicle platform**, *with built-in spec-series exemptions* for:
    * **NASCAR Cup NextGen**: Chevrolet Camaro ZL1, Ford Mustang Dark Horse, Toyota Camry XSE.
    * **NASCAR Xfinity Series**: Chevrolet Camaro, Ford Mustang, Toyota Supra.
    * **NASCAR Craftsman Trucks**: Chevrolet Silverado, Ford F-150, Toyota Tundra.
    * **NASCAR ARCA Menards**: Chevrolet Impala, Toyota Camry.
  * If files have different tracks or non-spec car platforms (e.g. Porsche 963 GTP vs Ferrari 296 GT3), an amber alert banner displays the exact mismatch and disables comparison overlay while keeping primary Session A fully accessible.

### ⏱ 30-Minute Max Lap Time Threshold:
* **Large Circuit & Wet Stint Support:**
  * Replaced the hardcoded `360.0s` limit with `MAX_VALID_LAP_TIME_SECS = 1800.0s` (30 minutes) across stint rolling averages (`valid_laps`, `numbered_valid_laps`) and live lap crossings.
  * Accommodates full-length laps at Circuit de la Sarthe (Le Mans), Nürburgring Nordschleife, historic Hockenheim, wet stints, and incident recovery limps without dropping valid laps from rolling averages or fuel calculations.

### ⚙ Repository Hygiene & Test Suite:
* Removed legacy `backup_pre_redesign` archive from repository root.
* Expanded test coverage to 46 automated unit tests verifying spec series exemptions, compatibility checkers, cross-file lap normalization, and font glyph resilience.

---

# The Search For Speed Analyzer — v0.3.7

### 📂 Documents Folder Default & Frictionless Setup:
* **Frictionless Windows Setup Installer (`installer.iss`):**
  * Default installation directory moved to `{userdocs}\The Search For Speed Analyzer` (`C:\Users\<Username>\Documents\SearchForSpeedAnalyzer`).
  * Eliminated the confusing "Install for all users vs me only" prompt on startup so setup launches straight into the install wizard.
  * Added `ArchitecturesInstallIn64BitMode=x64compatible` to ensure native 64-bit architecture and eliminate incorrect `Program Files (x86)` paths if users choose to install to Program Files.
  * Requires **zero Windows administrator elevation or UAC prompts**—clean, instant installation without permission errors.
* **Visible User Stints & Data in Documents (`src/settings.rs` & `src/stint_db.rs`):**
  * Stint files (`stints/*.s4s.json`), the stint database (`stint_database.json`), and settings (`settings.json`) now default to `C:\Users\<Username>\Documents\SearchForSpeedAnalyzer`.
  * Sits right beside `Documents\iRacing\telemetry` and `Documents\iRacing\setups`—highly visible, easy to back up, and effortless to share or drop teammate stint files.
  * **Automatic Legacy Migration:** Seamlessly detects and migrates existing stints, database records, and settings from `%APPDATA%\SearchForSpeedAnalyzer` on first launch so no historical runs are lost.

---

# The Search For Speed Analyzer — v0.3.6

### 🛡 Dual-Mode Setup Installer & Custom Drive Flexibility (`C:\Program Files`):
* **Dual-Mode All-Users vs Per-User Installation (`installer.iss`):**
  * Configured Inno Setup with `PrivilegesRequiredOverridesAllowed=dialog commandline` and dynamic `{autopf}` path resolution.
  * Completely eliminated the **`Error 5: Access is denied`** when selecting `C:\Program Files\SearchForSpeedAnalyzer`.
  * The setup wizard now presents a clean choice on startup:
    * **Install for all users (recommended):** Requests standard Windows UAC administrator elevation and installs to `C:\Program Files\SearchForSpeedAnalyzer` (or any custom directory) with verified write permissions.
    * **Install for me only:** Runs without UAC prompts, defaulting to `%LOCALAPPDATA%\Programs\SearchForSpeedAnalyzer` for users without administrative credentials.
  * Custom folder support: Users can freely install to secondary drives (`D:\`, `E:\`, etc.) or custom paths.
* **Intelligent In-App Auto-Updater (`src/updater.rs`):**
  * Silent in-app updates now detect whether the application is running from a protected system directory (`C:\Program Files`) and automatically pass `/ALLUSERS` with administrative elevation (`-Verb RunAs`), or `/CURRENTUSER` when installed per-user.

### ⏱ Session Lap Number Tracking & Robust Stint Ingestion:
* **Session Lap Number Synchronization:**
  * Saved stints now store the exact iRacing session lap number (`CarIdxLapCompleted` at the start/finish crossing) alongside each lap time.
  * Spreadsheet TSV exports now output both the stint-relative lap number and the official session lap number.
* **Timing & Edge-Case Telemetry Handling:**
  * Added resilient ingestion for identical consecutive lap times, out-lap transitions, and timing frames where iRacing increments the lap counter slightly before emitting the final lap time.

### 🔤 UI Glyphs & Branding:
* **Font Fallback Architecture:**
  * Enhanced font definition stacks to ensure all UI icons and telemetry symbols render cleanly without missing `?` glyphs across Windows configurations.
* **Branding Consistency:**
  * Standardized window titles and export headers to **The Search For Speed Analyzer**.

---

# The Search For Speed Analyzer — v0.3.5

### 📂 Local Stint Database, Track-Matched Re-Import & Teammate Sharing:
* **💾 Automatic Live Stint Saving (`%APPDATA%\SearchForSpeedAnalyzer`):**
  * Completed player stints (`1+` green-flag laps) are automatically saved locally in real time and before session transitions (`Practice` $\rightarrow$ `Qualifying` $\rightarrow$ `Race`) so you never lose a Practice or Qualifying stint when iRacing advances the server.
  * Each saved stint is ultra-lightweight (~2 KB) and stores lap times, per-lap fuel burn, 4-corner post-pit tire wear/temps/pressures, setup name, weather snapshot, and engineer notes.
* **📂 Saved Stints Database Manager (`Re-Import` & `Compare`):**
  * Open **`📂 Saved Stints`** from the Cockpit `STINT LOG` header, Grid View top bar, Stint Analyzer / Compare window, or the `…` menu.
  * Automatically filters to **🏁 Current Track** when connected to iRacing (with a green `MATCH` badge) or **🌎 All Saved Stints**, with instant search across Track, Car, Setup, Driver, and Notes.
  * Click **`📥 Re-Import`** to load any saved stint into your active session as `IMP 1`, `IMP 2`, etc. (`Ref` badge) without interfering with live `S1`, `S2` lap counting or pit tire latching, or click **`⚖ Compare`** to open it directly in a Side-by-Side Stint Comparison slot.
* **⛽ Automatic Lap 0 Historical Fuel Burn Seeding:**
  * Loading into a Track + Car combo that exists in your local Stint Database automatically seeds your baseline fuel burn rate on Lap 0 so **Burn / lap**, **Laps Left**, and **Pit Call** are immediately populated before completing your first lap.
* **📋 1-Click Discord Stint Code & `.s4s.json` File Sharing:**
  * Click **`📋 Share`** in Saved Stints (or **`💾 Save / Share`** on any driver's stint in Stint Analyzer) to copy a compact `S4S-STINT-V1:...` share code to your clipboard for Discord, or click **`📥 Paste Stint from Clipboard`** / **`📂 Open Stints Folder`** to import a teammate's stint and compare side-by-side.

---

# The Search For Speed Analyzer — v0.3.4

### 🏁 Post-Pit 4-Corner Tire Popup Overlay & Dual-Layer HUD Transparency:
* **🏁 Post-Pit 4-Corner Tire Popup Overlay:**
  * Added an optional in-car floating overlay that automatically pops up for a configurable duration (`3s–60s`, default `15s`) whenever new post-pit or garage 4-corner tire wear and temperatures are latched.
  * Displays `LF`, `RF`, `LR`, and `RR` 3-zone tread wear (`O·M·I` / `I·M·O`), carcass temperatures, and hot pressures, with independent position, scale (`75%–150%`), and **`👁 Preview / Move Popup`** controls in Settings.
* **✨ True Transparent Outer HUD Background (`Outer Background Box` vs `Inner Card Opacity`):**
  * Added Win32 DWM per-pixel alpha compositing (`DwmEnableBlurBehindWindow` + `DWMWA_BORDER_COLOR`) and separate **Inner Card Opacity** and **Outer Background Box** sliders (`0%` fully transparent by default) for both the main HUD overlay and the Tire Popup overlay, eliminating the solid black bounding box and bottom dead space.
* **🏁 Default Startup View Setting:**
  * Added a **Default Startup View** selector in Settings (`Cockpit Stint & Fuel Analyzer` by default, `Multi-Driver Stint Grid`, or `Offline .IBT Telemetry Lab`).

---

# The Search For Speed Analyzer — v0.3.3

### 📸 1-Click Discord Share Cards & 2x2 Stint Comparison Tire Layout:
* **📸 1-Click Discord Image Card Export (`Copy Discord Image`):**
  * Added **`📸 Copy Discord Image`** buttons to both the **Single Stint** analyzer and **Side-by-Side Stint Comparison** views.
  * Generates a high-contrast telemetry summary graphic card in memory and copies it directly to the Windows clipboard (both lossless `PNG` and `CF_DIB` bitmap formats) so you can immediately press `Ctrl+V` in Discord to share or compare runs.
  * **Shared Session & Weather Header:** Top header bar displays Track, Car, Session Type, Track Rubber State (`Moderate Rubber`, `Clean Track`, etc.), Weather Type, Track Temp, Air Temp, Wind Speed/Direction, and Humidity without repeating session weather inside each stint column.
  * **Complete Stint Breakdown:** Displays Driver, Stint Number, Laps Completed, Setup Name, Fastest Lap (with `★ BEST PACE` and delta callouts in comparison mode), Last Lap, Full Average, Consistency ($\sigma$), Fall-off, Fuel/Lap, Rolling Averages (`Avg 5` through `Avg 30`), and 4-corner Post-Stint Tire Wear & Temperatures.
* **🏁 2x2 Side-by-Side Tire Layout on Stint Comparison:**
  * Re-arranged the **Post-Stint Tire Wear & Temperatures** cards on the Stint Comparison page into a 2x2 chassis grid within each stint column: **Fronts (`LF` | `RF`) side-by-side on the top row**, and **Rears (`LR` | `RR`) side-by-side on the bottom row**, cutting vertical scrolling in half.
* **🔄 Custom Drive (`C:`, `D:`, `E:`) & Protected Folder Updater Improvements:**
  * Standalone portable `.exe` installs placed on any drive (`C:`, `D:`, `E:`, Desktop) now update directly in place without creating a second installation in `%LOCALAPPDATA%`.
  * Setup installations in custom folders pass `/DIR=<current_folder>` during silent updates and automatically request Windows UAC administrator permission (`RunAs`) only if installed inside a protected system folder (such as `C:\Program Files`).

---

# The Search For Speed Analyzer — v0.3.2

### ⏱ In-Car HUD Overlay "Race Remain" & Lap Counter Calibration:
* **🏁 Modern `SessionLapsRemainEx` Telemetry Synchronization:**
  * Migrated live telemetry countdown ingestion from legacy `SessionLapsRemain` to `SessionLapsRemainEx`, eliminating iRacing's historical off-by-one telemetry calculation bug that caused the overlay to announce "1 to go" when 2 laps remained.
* **🎯 Scheduled Race Distance Tracking (`SessionLapsTotal - player_lap`):**
  * Added live ingestion of `SessionLapsTotal` from continuous telemetry frames and subsession YAML metadata.
  * Laps remaining now calculate against individual driver line crossings (`SessionLapsTotal - player_lap`), ensuring your HUD ticks down accurately when you cross start/finish rather than fluctuating when the race leader crosses ahead.
* **🏁 Clean Race Conclusion & Checkered Flag State:**
  * Fixed telemetry parsing to allow 0 laps remaining to properly latch into state, completely eliminating the issue where the HUD remained stuck on "1 to go" through the checkered flag and cool-down lap.
  * Transitions cleanly to **`Finished`** upon race conclusion instead of displaying `"Unlimited"` or staying stuck on `"1 to go"`.
* **🚫 Zero-Clamped Completed Laps (Eliminated "Lap -1 done"):**
  * Properly handled iRacing SDK's `-1` initialization for `CarIdxLapCompleted` during pre-race warmup, grid formation, and pace laps.
  * Clamped completed laps to $\ge 0$ across all telemetry models and replaced the awkward `"Lap -1 done"` indicator with `"0 laps done"` and `"1 lap done"`.

### 🔄 Updater & Install Fixes:
* **No more "Run as administrator" needed to update:** "Update Now" now downloads and silently runs the installer, which installs per-user (no admin prompt) and relaunches the app automatically.
* **Protected-folder fallback:** If a copy lives in a protected folder (e.g. Program Files), Windows will ask for permission (UAC) only for the file copy, and the app reopens normally afterwards.
* **Settings now live in `%APPDATA%\SearchForSpeedAnalyzer`:** Existing settings are carried over automatically.
* **Beta and stable now install side by side** without replacing each other in Windows' installed apps list.
* **Recommended:** If you've been running the plain `.exe`, install with `SearchForSpeedAnalyzer-Setup.exe` going forward.

---

# The Search For Speed Analyzer — v0.3.1

### 🏁 NASCAR NextGen Calibration & Visual Tire Telemetry Suite:
* **🏷 NASCAR NextGen Left Rear Ride Height Calibration ($-0.50^{\prime\prime}$ / $-12.7\text{ mm}$):**
  * Automatically detects NASCAR NextGen Cup cars (`Chevrolet Camaro ZL1`, `Ford Mustang`, `Toyota Camry`) from session metadata and setup schema.
  * Corrects iRacing's known $+0.50^{\prime\prime}$ sensor datum offset by subtracting $0.50^{\prime\prime}$ ($0.0127\text{ m}$) across all continuous samples, aligning LR ride height, dynamic longitudinal pitch rake, roll / side rake, and rear splits with physical garage setup sheets.
  * Prominent calibration banner in the **`📐 Ride Height & Rake`** tab confirms active calibration.
* **📈 Synchronized Color Telemetry Graphs for Tires & Pressures (0–100% Lap Distance):**
  * Added dedicated multi-trace telemetry chart with view toggles: **`4-Corners`** (LF, RF, LR, RR center temps), **`⚡ Front Axle`** (LF vs RF 3-zone), **`🛡 Rear Axle`** (LR vs RR 3-zone), and **`📊 Pressures (PSI / kPa)`**.
  * Synchronized cursor scrubbing displaying live temperature, pressure, speed, and track distance.
* **📍 Track Sector Reference Bar & Peak Location Callouts:**
  * Interactive track sector timeline directly below the chart identifies corner locations: Frontstretch ($0\text{–}15\%$), Turn 1/2 ($15\text{–}35\%$), Backstretch ($35\text{–}65\%$), Turn 3/4 ($65\text{–}85\%$), and Tri-Oval ($85\text{–}100\%$).
  * Prominent **Peak Thermal Badges** (`🔥 Peak: 233.6°F · 📍 Turn 3/4 Apex 74% Dist`) identify exactly where on track and on which tire corner maximum thermal stress occurred.
* **🏁 Top-Down 4-Wheel Chassis Heatmap Cards:**
  * Re-architected corner cards into a top-down car chassis layout (Front Axle $\rightarrow$ Chassis Divider $\rightarrow$ Rear Axle) with dynamic temperature heat-tinting (Cool Blue, Optimal Green, Warm Amber, Overheating Red).
* **🏁 Stint Run Progression Table:**
  * Full session overview table listing completed laps, lap times, status badges, 4-corner peak temperatures, hottest tire callouts, and hot pressures with 1-click lap selection.

---

# The Search For Speed Analyzer — v0.3.0

### 📊 Offline iRacing `.IBT` Telemetry Analysis Engine & Lab View:
* **Dedicated `.IBT` Telemetry Lab:** Added a full offline `.ibt` binary telemetry ingestion engine and interactive analysis view (`📊 .IBT Telemetry Lab`), automatically scanning `%USERPROFILE%\Documents\iRacing\telemetry\` on a background worker thread without requiring an active iRacing session.
* **Lap Slicing, Classification & Lap-to-Lap Overlay (`Lap A vs. Lap B`):** Automatically indexes all session laps, flags Out-Laps, In-Laps, Off-Track/Incomplete laps, and Valid flying laps (`PB ★`), and normalizes `LapDistPct` (`0–100%`) across a 1,000-point grid for direct `Lap A vs. Lap B` trace comparison and time delta ($\Delta t$) analysis.
* **🏁 Driver Inputs & Vehicle Dynamics Tab:** Synchronized 4-trace telemetry charts for **Speed**, **Throttle % & Brake %**, **Steering Wheel Angle (°)**, and **Gear / RPM**, complete with interactive cursor scrubbing and Lap A vs. Lap B comparison table.
* **📐 Ride Height & Rake Tab (5 Dedicated Scrollable Charts):**
  * **Front Ride Heights & Center Splitter:** Dedicated trace panel for `LF`, `RF`, and `CFSR` (Center Front Splitter) ride heights with bottoming/ground-strike warning lines.
  * **Rear Ride Heights:** Dedicated trace panel for `LR` and `RR` ride heights.
  * **Dynamic Longitudinal Pitch Rake:** Vectorized trace for $\text{Rear RH} - \text{Front RH}$ (nose-down compression vs. rebound).
  * **Dynamic Roll / Side Rake:** Vectorized trace for $\text{Left RH} - \text{Right RH}$ lateral roll attitude.
  * **Lateral Corner Splits:** Dedicated trace panel comparing Front Split ($\text{LF} - \text{RF}$) and Rear Split ($\text{LR} - \text{RR}$).
* **📉 Shock Velocity Histograms Tab:** 4-corner (`LF`, `RF`, `LR`, `RR`) damper velocity distribution histograms with native `*shockVel` extraction or Savitzky-Golay / moving-average smoothed numerical differentiation ($\Delta \text{Defl} / \Delta t$), low/high-speed compression & rebound breakdowns, and asymmetry diagnostics.
* **Tires & Pressures Tab:** 4-corner tire analysis cards pulling definitive hot tire surface/carcass temperatures (`O · M · I` for Left tires, `I · M · O` for Right tires), 3-zone tread wear percentages, and Cold $\rightarrow$ Hot pressure build from `CarSetup` YAML, pit-stall latches, and `.ibt` channels, plus inside-to-outside spread ($\Delta T_{\text{I-O}}$) and crown bias diagnostics.
* **Session YAML Header & 1-Click TSV Export:** Displays full session metadata (Track, Car, Driver, Weather, Air/Track Temp, Wind Speed/Direction, Setup Name) respecting Imperial (`°F`, `mph`, `mi`, `in`, `PSI`) and Metric (`°C`, `km/h`, `km`, `mm`, `kPa`) settings, plus 1-click clipboard TSV export for Excel/Google Sheets.

---

# The Search For Speed Analyzer — v0.2.6

### ⚖ Side-by-Side Stint Telemetry Comparison:
* **Compare Up to 3 Stints Side-by-Side:** Select and compare up to 3 individual stints simultaneously — analyze your own progression across tire runs and setup revisions, or benchmark directly against competitors in the session.
* **Multi-Stint Lap Time Progression Chart:** Interactive line chart overlay with color-coded curves, stint PB highlights, and instant cursor hover displaying exact lap times and deltas to the fastest car on that lap.
* **Dynamic Lap-by-Lap Comparison Log:** Side-by-side lap table with stint personal best highlights, delta times relative to baseline Stint A, and purple star callouts for the fastest driver on each lap.
* **4-Corner Post-Stint Tire Wear & Temperature Comparison:** Compare LF, RF, LR, and RR inner/middle/outer carcass temperatures, cold vs hot pressures, and wear degradation cards across stints.
* **Key Metrics & Pace Drop-off Analysis:** Side-by-side cards comparing completed laps, peak lap time, full stint average, consistency standard deviation ($\sigma$), fuel consumption rate per lap, and early-vs-late pace fall-off (laps 1–5 vs stint finish).
* **Dedicated Floating Window & Pop-Out Modal:** Access stint comparisons from Cockpit View, Grid View, the overflow menu (`…`), or via the `[ ↗ Pop Out ]` button in the Stint Telemetry Analyzer.
* **1-Click Spreadsheet Export:** Copy full side-by-side stint data and lap-by-lap comparison logs to clipboard formatted for Excel/Google Sheets.

### 💨 High-Legibility Relative Wind Dial & 14pt Speed Typography:
* **High-Contrast 14pt Speed Typography:** Enlarged wind speed digits to prominent 14pt bold text for both single and double-digit wind speeds (e.g. 6 to 35+ mph or km/h), with a clean, uncluttered dial face for instant readability at race pace.
* **Collision-Free Arrow Vectoring:** Vector pointer line originates cleanly outside the center text zone, ensuring the arrow shaft and chevron never cut through or obscure the digits.
* **Decoupled Car-Nose Heading Pip:** Re-engineered the 12 o'clock forward heading reference as a sleek gold triangular pip on the outer perimeter, completely eliminating visual polygon clipping or wonky overlap when wind blows forward.

---

# The Search For Speed Analyzer — v0.2.5

### 💨 Precision Wind Direction Vectoring & 1-to-1 Speed Match:
* **True Cockpit Wind Orientation Alignment:** Fixed the wind angle calculation across the app (`(WindDir - YawNorth + PI)`), aligning vehicle yaw orientation with iRacing's clockwise coordinate system. Tailwinds now point forward (12 o'clock / UP), headwinds point backward (6 o'clock / DOWN), and quartering tailwinds (e.g. 2 o'clock) match external tools like Stint Analyzer 1-to-1 without inversion.
* **Exact Wind Velocity Decimal Precision:** Eliminated the premature truncation of live wind speed (`format!("{:.2} m/s", wv)`), ensuring wind speed across all app views perfectly matches iRacing's Weather tab (e.g. 5 mph instead of rounding up to 6 or 7 mph).
* **Enhanced Wind Vector Visuals:** Polished the wind vector compass gauge with a prominent sharp chevron pointer, higher contrast gold reference pip at 12 o'clock, and bright teal shaft for instant cockpit readability.

---

# The Search For Speed Analyzer — v0.2.4

### 💨 Car-Relative Dynamic Wind Heading & Prominent Vector Gauge:
* **Cockpit-Relative Real-Time Wind Tracking:** Synchronized wind telemetry against vehicle yaw (`YawNorth` at 60 Hz). As you steer around the track, the wind arrow dynamically updates in conjunction with the direction the car is facing.
* **Intuitive Wind Vectoring:** Crosswinds from left to right point directly right (3 o'clock), headwinds point directly into the car nose (6 o'clock), and tailwinds point forward (12 o'clock).
* **High-Visibility Prominent Pointer:** Enlarged gauge diameter to `28px` with a bold cyan chevron, outlined vector shaft, and gold car-nose reference pip at 12 o'clock for instantaneous glanceability at race pace.
* **Aerodynamic Tooltip Feedback:** Hovering over the gauge provides relative wind classification (e.g. `Crosswind (Left → Right) · True Compass: NE (45° rel)`).

---

# The Search For Speed Analyzer — v0.2.3

### ☀ Air/Track Temperature & Wind Direction Overlay:
* **Air & Track Surface Temp Readout:** Integrated live environmental telemetry into the floating overlay HUD (`Air/Track Temp: 72/82` in Fahrenheit or `22/28` in Celsius) so drivers can monitor track grip loss and ambient conditions without leaving their driving line.
* **Wind Direction Compass Gauge:** Custom-drawn circular vector compass widget with the live wind speed number centered inside and a sharp pointer arrowhead indicating exact wind heading.
* **Modular HUD Toggle:** Added `"Track Temp & Wind"` toggle checkbox in Settings under *Visible HUD Sections*.
* **Integrated HUD & Dashboard Polish:** Seamlessly fits into the in-car overlay status bar with generous hover tooltips detailing exact temperatures and angles (`Wind: 6 mph NE (45°)`), plus a live mini compass in the main application header.
* **Cockpit Engineer Best / Fastest Lap Readout:** Added "Best lap" right beneath "Last lap" in Column 1's Fuel & Stint card, styled in high-visibility status green with live stint and session fallback.

---

# The Search For Speed Analyzer — v0.2.2

### 🏁 Definitive Tire Telemetry & Garage Sync:
* **Direct iRacing Garage Matching:** Integrated direct extraction of iRacing's definitive garage tire data (`CarSetup`) from the session string, guaranteeing exact 1-to-1 matches with the in-game iRacing garage screen for last hot pressures, carcass temperatures, and tread wear percentages (LF: `12.1 psi` · `194° · 187° · 164°F` · `86% · 86% · 100%`).
* **Preserved Native Tire Ordering:** Maintained the canonical telemetry reading order (`temp_l · temp_m · temp_r` and `wear_l · wear_m · wear_r`), which naturally maps to `O·M·I` for Left tires and `I·M·O` for Right tires exactly as shown in the iRacing garage.
* **Zero Estimation Policy:** Completely eliminated ideal gas law math and pressure estimation formulas. Stint tires now exclusively report ground-truth telemetry values directly measured by iRacing.
* **Accurate Pit Stall Latching:** Prevented premature tire latching at the pit entry line while moving at speed. Stint tires are now latched when stationary in the pit stall following crew service or upon returning to the garage.
* **Isolated Stint Data:** Removed unserviced stint fallback to previous runs. Stints without pit tire service (such as 1-lap runs or drive-throughs) now cleanly display empty dashes (`"—% · —% · —%"`, `"— psi"`) with clear `"NO PIT SERVICE"` indicators instead of showing wear from another stint.
* **Excel TSV Export Alignment:** Updated all clipboard export functions ("Copy Temps", "Copy Wear", "Copy 4-Tire Table") to match the garage screen layout for seamless analysis in spreadsheets.

---

# The Search For Speed Analyzer — v0.2.1

### ⚙ UI Refinements & Styling Fixes:
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

### 🏁 Instant First-Frame Telemetry & Mid-Run Attach:
* **Immediate Lap 0 Pit Exit Detection:** Fixed an issue where launching the app while sitting in the pit or garage on Lap 0 could clear the pit exit transition check, preventing Stint 1 from starting until the app was closed and reopened.
* **Instant Mid-Run Attach:** Launching the app while already driving out on track now immediately recognizes the active session and creates Stint 1 at your current lap, recording telemetry and fuel with zero missed laps.

### 📖 Release Notes & "What's New" Window Overhaul:
* **Compact, Centered Layout:** Redesigned the "What's New" dialog into a fixed, centered 580x620 modal that never stretches horizontally off-screen.
* **Smart Word Wrapping:** Added automatic text wrapping across all bullet points and descriptions, completely eliminating horizontal scrolling.
* **Always-Visible Controls:** The "Done" button is permanently pinned within the visible window at the bottom right.

---

# The Search For Speed Analyzer — v0.1.3

### 🏁 Driver Roster & Stint Grid Restoration:
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

### ⚙ Bug Fixes & Improvements:

#### 1. Session Phase Transitions & Lap Reset (Practice → Qualify → Race)
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
