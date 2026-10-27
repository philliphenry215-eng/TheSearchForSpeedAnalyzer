pub mod cockpit;
pub mod grid;
pub mod modals;
pub mod overlay;
pub mod stint_modal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridSortKey {
    Position,
    CarNumber,
    DriverName,
    StintNumber,
    LapsCompleted,
    LastLap,
    FastestLap,
    CurrentAverage,
    AvgBucket(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StintAnalyzerTab {
    SingleStint,
    Compare,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StintCompareSlot {
    pub car_idx: usize,
    pub stint_number: i32,
}