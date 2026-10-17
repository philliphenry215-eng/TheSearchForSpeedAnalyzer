# Project: iRacing War Room (Antigravity Edition)

## **Project Vision**
An autonomous, agent-led standalone application designed to provide high-performance, real-time analytics for iRacing. The application focuses on translating raw telemetry into actionable racing intelligence with a clean, distraction-free interface.

---

## **Functional Requirements**

### **1. Intelligent Stint Analysis**
* **Dynamic Lap Buckets:** * Calculate rolling averages based on custom-defined segments.
    * Presets for "Qualifying Sim," "Full Tank Stint," or "Last 10 Laps."
    * Ability to manually flag and bucket laps during a live session.
* **Input Correlation:** * Cross-reference lap time variance with telemetry traces.
    * Identify specific time-loss events (e.g., "Lap 5 was 0.3s slower due to early braking at T1").
    * Highlight inconsistencies in braking points or throttle application across a stint.

### **2. Pit & Tire Intelligence**
* **Post-Stop Retrieval:** * Automatically trigger a data pull for tire wear percentages and heat maps the moment the car is "on the jacks."
    * Log historical tire data to compare wear across different compounds or setups.
* **Strategy Prediction:** * Predict optimal pit windows based on live tire degradation and fuel consumption.
    * Offer "Stay Out" vs. "Pit Now" recommendations based on pace decay.

### **3. Environmental Monitoring**
* **Session Weather:** * Live tracking of ambient air temperature, track temperature, humidity, and wind speed/direction.
* **Grip Evolution:** * Correlation engine to show how fluctuating track temps affect tire pressures and lap time decay.
    * Visual alerts for significant shifts in wind direction (e.g., "Headwind shift at T1").

### **4. High-Fidelity Telemetry Traces**
* **Standard Inputs:** * Real-time, high-refresh-rate graphing for **Speed**, **Throttle**, **Brake**, and **Gear**.
* **Chassis Dynamics:** * Track **Wheel Input (Steering Angle)** to identify "scrubbing" or excessive steering lock.
    * Monitor steering-to-grip ratios to help the driver minimize tire overheating.

---

## **Technical Implementation Guidelines**
* **Language & Stack:** Agent-selected. The Antigravity agent is authorized to choose the most efficient language (e.g., C++, Rust, or Python) to ensure low-latency interaction with the iRacing Memory Map.
* **Interface Type:** Standalone desktop application (no web-browser overhead).
* **UI Aesthetic:** Minimalist "Dark Mode" with matte sage-green accents. Lighting should be soft and diffused to prevent glare while driving in a sim rig.

---

## **Development Roadmap**
1.  **Phase 1:** Establish iRacing SDK connection and data polling.
2.  **Phase 2:** Build the Telemetry & Weather UI modules.
3.  **Phase 3:** Implement the Stint Bucketing and Strategy Prediction logic.
4.  **Phase 4:** Performance optimization and telemetry overlay features.