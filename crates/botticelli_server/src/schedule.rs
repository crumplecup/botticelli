//! Task scheduling abstractions for actor servers.
//!
//! This module provides trait-based scheduling with support for multiple
//! schedule types (interval, cron, one-time, immediate).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Result of checking if a task should run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleCheck {
    /// Whether the task should run now
    pub should_run: bool,
    /// When the task should run next (if applicable)
    pub next_run: Option<DateTime<Utc>>,
}

impl ScheduleCheck {
    /// Create a new schedule check result.
    pub fn new(should_run: bool, next_run: Option<DateTime<Utc>>) -> Self {
        Self {
            should_run,
            next_run,
        }
    }

    /// Task should run immediately with no future schedule.
    pub fn run_once() -> Self {
        Self {
            should_run: true,
            next_run: None,
        }
    }

    /// Task should not run yet, schedule for future time.
    pub fn wait_until(next_run: DateTime<Utc>) -> Self {
        Self {
            should_run: false,
            next_run: Some(next_run),
        }
    }

    /// Task should run now and schedule for future time.
    pub fn run_and_schedule(next_run: DateTime<Utc>) -> Self {
        Self {
            should_run: true,
            next_run: Some(next_run),
        }
    }
}

/// Trait for schedule types that can determine when tasks should run.
pub trait Schedule {
    /// Check if a task should run now based on last execution time.
    ///
    /// # Arguments
    ///
    /// * `last_run` - When the task last executed (None if never run)
    ///
    /// # Returns
    ///
    /// ScheduleCheck indicating whether to run and when to check next
    fn check(&self, last_run: Option<DateTime<Utc>>) -> ScheduleCheck;

    /// Calculate the next execution time after a given reference time.
    ///
    /// # Arguments
    ///
    /// * `after` - Reference time to calculate next execution from
    ///
    /// # Returns
    ///
    /// Next execution time, or None if schedule is exhausted
    fn next_execution(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>>;
}

/// Types of task schedules supported by the actor server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ScheduleType {
    /// Cron expression (7 fields: sec min hour day month weekday year)
    ///
    /// Example: "0 0 9 * * * *" = 9 AM daily
    Cron {
        /// Cron expression string
        expression: String,
    },

    /// Fixed interval in seconds
    Interval {
        /// Interval duration in seconds
        seconds: u64,
    },

    /// One-time execution at specific time
    Once {
        /// Execution timestamp
        at: DateTime<Utc>,
    },

    /// Execute immediately on startup
    Immediate,
}

impl Schedule for ScheduleType {
    fn check(&self, last_run: Option<DateTime<Utc>>) -> ScheduleCheck {
        let now = Utc::now();

        match self {
            ScheduleType::Immediate => {
                if last_run.is_none() {
                    ScheduleCheck::run_once()
                } else {
                    ScheduleCheck::wait_until(now + Duration::hours(24))
                }
            }
            ScheduleType::Once { at } => {
                if last_run.is_none() && now >= *at {
                    ScheduleCheck::run_once()
                } else if last_run.is_none() {
                    ScheduleCheck::wait_until(*at)
                } else {
                    ScheduleCheck::new(false, None)
                }
            }
            ScheduleType::Interval { seconds } => {
                let interval = Duration::seconds(*seconds as i64);
                match last_run {
                    None => {
                        let next = now + interval;
                        ScheduleCheck::run_and_schedule(next)
                    }
                    Some(last) => {
                        let next = last + interval;
                        if now >= next {
                            ScheduleCheck::run_and_schedule(next + interval)
                        } else {
                            ScheduleCheck::wait_until(next)
                        }
                    }
                }
            }
            ScheduleType::Cron { expression } => match cron::Schedule::from_str(expression) {
                Ok(schedule) => {
                    let after = last_run.unwrap_or(now);
                    if let Some(next) = schedule.after(&after).next() {
                        if now >= next {
                            if let Some(future) = schedule.after(&now).next() {
                                ScheduleCheck::run_and_schedule(future)
                            } else {
                                ScheduleCheck::run_once()
                            }
                        } else {
                            ScheduleCheck::wait_until(next)
                        }
                    } else {
                        ScheduleCheck::new(false, None)
                    }
                }
                Err(_) => ScheduleCheck::new(false, None),
            },
        }
    }

    fn next_execution(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            ScheduleType::Immediate => None,
            ScheduleType::Once { at } => {
                if after < *at {
                    Some(*at)
                } else {
                    None
                }
            }
            ScheduleType::Interval { seconds } => Some(after + Duration::seconds(*seconds as i64)),
            ScheduleType::Cron { expression } => {
                if let Ok(schedule) = cron::Schedule::from_str(expression) {
                    schedule.after(&after).next()
                } else {
                    None
                }
            }
        }
    }
}
