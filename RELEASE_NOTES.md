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

---

# The Search For Speed Analyzer — v0.5.8

### Static Top Telemetry HUD: 2D Track Map & Motorsport G-G Circle:
* **Pinned Top Telemetry HUD:**
  * Moved the 2D Track Map and G-G Friction Circle into a dedicated Static Top HUD pinned above all analysis tabs (`Inputs & Speed`, `Ride Height & Rake`, `Shock Histograms`, `Tires & Pressures`, and `Channels & Export`) with a 1-click `[▲ Collapse HUD]` / `[▼ Expand Top HUD]` toggle.
* **Aspect-Preserving 2D Circuit Map:**
  * True 1:1 isometric track proportions keep both ovals and road courses accurately shaped, complete with an S/F marker, `↺ 90°` rotate / `Reset` controls, and click-and-drag track scrubbing.
* **Motorsport G-G Traction Circle:**
  * Aligned to standard motorsport conventions (`◀ Turn Left` / `Turn Right ▶`) with phase-colored grip points (braking, throttle, pure lateral cornering, transition), 1.0G/2.0G reference rings, a peak traction boundary hull, and live cornering/braking G readouts.
