# The Search For Speed Analyzer — v0.5.11

### Seamless 100% Corner & Short-Stint Tire Capture:
* **Unworn `100%` Corners Now Update Immediately on Pit Stops:**
  * Fixed an issue (notably on Dirt Late Models where `LF` often stays at `100%` while `RF`/`LR`/`RR` wear to `99%`–`95%`) where individual `100%` corners waited until you opened the Garage before appearing.
  * Pit-stall detection now evaluates service across all 4 corners as one set, attaching `100%` wear, temperatures, and pressures right alongside worn corners immediately in the pit stall.
* **Short-Run `100%` Stints with Hot Temperatures:**
  * Short runs where all 4 corners remain at `100%` tread—whether captured in the pit stall via carcass temperature jump or in the Garage with real thermal spread—now attach cleanly and survive local Stint DB sync, while uniform cold setup resets continue to be ignored.

---

# The Search For Speed Analyzer — v0.5.10

### Instant Garage & Pit-Exit Tire Latching + Clean Release History:
* **Immediate Garage Tire Capture on Exit-Car (`Esc`):**
  * Eliminated a frame-ordering delay where exiting the car on the same tick iRacing published garage tire measurements could skip attaching those tires until the next YAML refresh.
  * Exiting the car now forces an immediate YAML check and UI refresh so wear, hot pressures, and pyrometer temperatures attach right away.
* **4-Corner Garage Tire Validity & Doc Cleanup:**
  * Ensured garage tire detection checks all 4 corners consistently, and streamlined `RELEASE_NOTES.md` and `CHANGELOG.md` into clean driver-focused highlights.

---

# The Search For Speed Analyzer — v0.5.9

### Tire Accuracy, Cockpit Display & .IBT Comparison Polish:
* **Post-Stint Tire Tracking:**
  * Completed stint tire wear is now locked in as soon as you head back out on track or check the garage, so aborted out-laps never overwrite an earlier stint's tire numbers.
  * Fuel-only and damage-repair pit stops now attach your updated black-box tire wear to the stint that just ended while keeping your final hot on-track tire pressures.
* **Cockpit Tires Panel:**
  * Always stays locked on your own latest post-stint pit or garage tire readout—clicking around in the Stint Log or importing reference stints no longer changes the Cockpit panel.
  * Starts clean (`TIRES · AWAITING PIT`) at the start of every new session until you complete laps and pit or return to the garage.
* **4-Corner Wear, Corded Tires & Setup Exports:**
  * Full 4-corner detection and display across UI cards, overlay popups, Discord share cards, Excel exports, and Saved Stints (`LR`/`RR` corners and `0%` corded bands now display properly).
  * Fixed Celsius garage tire temperatures above 120°C being misconverted, and hid local `.sto` setup buttons when inspecting competitor stints.
* **.IBT Telemetry Lab:**
  * Fixed the analysis scroll area height and moved the Lap B G-G readout inline in the header so Lap A vs. Lap B comparisons stay aligned without shifting the layout.
