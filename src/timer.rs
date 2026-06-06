pub const WORK_MINUTES: u32 = 50;
pub const BREAK_MINUTES: u32 = 10;
pub const CYCLE_MS: u64 = 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkPhase {
    Work,
    Break,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerState {
    pub phase: WorkPhase,
    pub remaining_secs: u32,
}

impl TimerState {
    pub fn remaining_label(self) -> String {
        let minutes = self.remaining_secs / 60;
        let seconds = self.remaining_secs % 60;
        format!("{minutes:02}:{seconds:02}")
    }

    pub fn title(self) -> &'static str {
        match self.phase {
            WorkPhase::Work => "일하는 시간",
            WorkPhase::Break => "쉬는시간",
        }
    }

    pub fn character(self) -> &'static str {
        match self.phase {
            WorkPhase::Work => "^_^",
            WorkPhase::Break => "-_- z",
        }
    }
}

pub fn state_from_time(minute: u32, second: u32) -> TimerState {
    debug_assert!(minute < 60);
    debug_assert!(second < 60);

    if minute < 50 {
        TimerState {
            phase: WorkPhase::Work,
            remaining_secs: ((49 - minute) * 60) + (60 - second),
        }
    } else {
        TimerState {
            phase: WorkPhase::Break,
            remaining_secs: ((59 - minute) * 60) + (60 - second),
        }
    }
}

pub fn state_from_epoch_ms(now_ms: u64) -> TimerState {
    state_from_schedule(now_ms, now_ms - (now_ms % CYCLE_MS))
}

pub fn state_from_schedule(now_ms: u64, cycle_start_ms: u64) -> TimerState {
    let elapsed_secs = now_ms.saturating_sub(cycle_start_ms) / 1000;
    let elapsed_secs = elapsed_secs.min((CYCLE_MS / 1000).saturating_sub(1));
    let work_secs = u64::from(WORK_MINUTES) * 60;
    let cycle_secs = u64::from(WORK_MINUTES + BREAK_MINUTES) * 60;
    if elapsed_secs < work_secs {
        TimerState {
            phase: WorkPhase::Work,
            remaining_secs: u32::try_from(work_secs - elapsed_secs).unwrap_or(u32::MAX),
        }
    } else {
        TimerState {
            phase: WorkPhase::Break,
            remaining_secs: u32::try_from(cycle_secs - elapsed_secs).unwrap_or(u32::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_phase_counts_down_to_break() {
        assert_eq!(
            state_from_time(49, 59),
            TimerState {
                phase: WorkPhase::Work,
                remaining_secs: 1
            }
        );
        assert_eq!(state_from_time(0, 0).remaining_label(), "50:00");
    }

    #[test]
    fn break_phase_counts_down_to_next_hour() {
        assert_eq!(
            state_from_time(50, 0),
            TimerState {
                phase: WorkPhase::Break,
                remaining_secs: 600
            }
        );
        assert_eq!(
            state_from_time(59, 59),
            TimerState {
                phase: WorkPhase::Break,
                remaining_secs: 1
            }
        );
    }

    #[test]
    fn phase_boundary_switches_at_minute_fifty() {
        assert_eq!(state_from_time(49, 59).phase, WorkPhase::Work);
        assert_eq!(state_from_time(50, 0).phase, WorkPhase::Break);
        assert_eq!(state_from_time(0, 0).phase, WorkPhase::Work);
    }

    #[test]
    fn state_from_epoch_ms_uses_utc_hour_boundary() {
        // Given: UTC timestamps around the shared global break boundary.
        let minute_49_second_59 = (49 * 60 * 1000) + (59 * 1000);
        let minute_50_second_00 = 50 * 60 * 1000;

        // When: timer state is computed from UTC epoch milliseconds.
        let work = state_from_epoch_ms(minute_49_second_59);
        let break_time = state_from_epoch_ms(minute_50_second_00);

        // Then: every client sees the same absolute work/break switch.
        assert_eq!(work.phase, WorkPhase::Work);
        assert_eq!(work.remaining_secs, 1);
        assert_eq!(break_time.phase, WorkPhase::Break);
        assert_eq!(break_time.remaining_secs, 600);
    }
}
