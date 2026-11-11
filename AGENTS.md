# AGENTS.md - SearchForSpeedAnalyzer

Guidelines, architectural context, and development workflows for AI agents working in this repository.

---

## 1. Project Overview & Tech Stack

**SearchForSpeedAnalyzer** is a high-performance standalone Windows desktop application providing real-time telemetry analysis, stint tracking, and post-session review for **iRacing**.

* **Language**: Rust (2021 edition)
* **GUI Framework**: `eframe` / `egui` (0.27.2) with `egui_extras`
* **Telemetry & Data Sources**:
  * `iracing` (0.4.1) for live memory-mapped shared memory polling
  * Custom binary parser for iRacing `.ibt` telemetry files
  * Custom `.sto` car setup file parser
* **Distribution & Packaging**: Windows binary compiled with `winres` icon embedding, packaged into a per-user installer via Inno Setup (`installer.iss`).

---

## 2. Codebase Architecture & File Guide

Keep modifications modular and respect the separation between raw telemetry parsing, state models, and UI rendering:

### Core Telemetry & Parsing
* `src/telemetry.rs`: Live iRacing SDK connection and thread-safe memory map polling.
* `src/ibt.rs`: Binary parser for iRacing telemetry log files (`.ibt`). Handles headers, channel definitions, and sub-sampled data streams.
* `src/setup_file.rs`: Parsing of iRacing `.sto` setup files.

### Domain Models & Persistence
* `src/models.rs`: Core data structures (`Stint`, `Lap`, `TelemetryFrame`, session states, tire temps/wear).
* `src/stint_db.rs`: Stint logging, historical comparisons, and SQLite/JSON persistence.
* `src/settings.rs`: Application configuration saved to `settings.json` (view preferences, overlay toggles, custom bucket sizes).

### UI & Visualization (`egui`)
* `src/main.rs`: App entry point (`eframe::run_native`), top-level window frame, menu bar, and global event loop.
* `src/ibt_ui.rs`: Telemetry graph rendering, trace comparison, cursor scrubber, and lap overlay charts.
* `src/views/cockpit.rs`: Live dashboard view showing real-time inputs, delta times, and track conditions.
* `src/views/grid.rs`: Field standings, competitor relative times, and live gap monitoring.
* `src/views/overlay.rs`: Borderless, semi-transparent HUD overlay mode for in-sim driving.
* `src/views/stint_modal.rs`: Deep-dive modal inspecting stint averages, fuel burn, and tire degradation.
* `src/views/modals.rs`: Settings dialogs, session selector, and miscellaneous popups.

### Utilities
* `src/formatters.rs`: Display formatting for lap times (`mm:ss.ms`), speeds (mph / kph), pressures (psi / kPa), and fuel.
* `src/discord_card.rs`: Markdown/Discord-ready stint summaries for team communication.
* `src/updater.rs`: Background GitHub release checking and update prompting.

---

## 3. Development Guidelines & Rules

### A. Large File Management
* Files like `main.rs`, `ibt.rs`, and `ibt_ui.rs` are substantial (>1,000 lines).
* **Do not rewrite entire files.** Always make surgical, targeted edits.
* When adding significant new UI components or features, prefer creating a dedicated module under `src/views/` or `src/` rather than expanding already large files.

### B. Egui / UI Guidelines
* **Egui version**: Target `egui 0.27.2` API conventions.
* **Palette**: Dark theme with soft matte sage-green and gold accents (see `S4SGoldBlue` theme). High-contrast glaring colors must be avoided so drivers in dark sim rigs are not blinded.
* **Responsive Layouts**: Support window resizing and zoom offsets (`font_size_offset`). Never hardcode rigid pixel bounds when `ui.available_width()` or proportional sizing is appropriate.

### C. Telemetry Handling & Safety
* **Defensive Decoding**: iRacing channel definitions and car telemetry variables vary by vehicle and track. Never blindly `.unwrap()` telemetry lookups or file parsing offsets.
* **Thread Safety**: Live telemetry polling runs on a background thread and exchanges state with the UI loop through thread-safe primitives (`Arc<Mutex<...>>` or channels). Never perform disk I/O, network requests, or heavy computations directly inside the egui `update()` pass.

---

## 4. Verification & Build Commands

Before marking any task as complete, always verify your Rust changes:

```powershell
# 1. Fast compile & type check (MANDATORY after any code edit)
cargo check

# 2. Run unit tests
cargo test

# 3. Check for common lints
cargo clippy -- -D warnings

# 4. Release build
cargo build --release
```

### Packaging Windows Releases:
* Inno Setup script is at `installer.iss`.
* Executable output resides in `target/release/SearchForSpeedAnalyzer.exe`.
