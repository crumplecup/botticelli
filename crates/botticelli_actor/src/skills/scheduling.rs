//! Content scheduling skill.

use crate::{Skill, SkillContext, SkillOutput, SkillResult};
use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, Utc};
use serde_json::json;

/// Skill for scheduling content posts based on time windows.
pub struct ContentSchedulingSkill {
    name: String,
}

impl ContentSchedulingSkill {
    /// Create a new content scheduling skill.
    pub fn new() -> Self {
        Self {
            name: "content_scheduling".to_string(),
        }
    }
}

impl Default for ContentSchedulingSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for ContentSchedulingSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Schedule content posts based on configured time windows"
    }

    #[tracing::instrument(skip(self, context), fields(skill = %self.name))]
    async fn execute(&self, context: &SkillContext) -> SkillResult<SkillOutput> {
        tracing::debug!("Executing content scheduling skill");

        let window_start = context
            .config
            .get("schedule_window_start")
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
            .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap());

        let window_end = context
            .config
            .get("schedule_window_end")
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
            .unwrap_or_else(|| NaiveTime::from_hms_opt(17, 0, 0).unwrap());

        let randomize = context
            .config
            .get("randomize_within_window")
            .map(|s| s.parse::<bool>().unwrap_or(true))
            .unwrap_or(true);

        tracing::info!(
            window_start = %window_start,
            window_end = %window_end,
            randomize,
            "Scheduling configuration loaded"
        );

        let mut total_items = 0;
        for (table_name, rows) in &context.knowledge {
            tracing::debug!(table = %table_name, count = rows.len(), "Processing knowledge table");
            total_items += rows.len();
        }

        let now = Utc::now();
        let next_slot = calculate_next_slot(now, window_start, window_end, randomize);

        tracing::info!(
            items = total_items,
            next_slot = %next_slot,
            "Content scheduling completed"
        );

        Ok(SkillOutput {
            skill_name: self.name.clone(),
            data: json!({
                "total_items": total_items,
                "next_slot": next_slot.to_rfc3339(),
                "window_start": window_start.format("%H:%M").to_string(),
                "window_end": window_end.format("%H:%M").to_string(),
                "randomized": randomize,
            }),
        })
    }
}

fn calculate_next_slot(
    now: DateTime<Utc>,
    window_start: NaiveTime,
    window_end: NaiveTime,
    randomize: bool,
) -> DateTime<Utc> {
    let current_time = now.time();

    if current_time >= window_start && current_time <= window_end {
        if randomize {
            let offset_secs = now.timestamp() % 900;
            return now + chrono::Duration::seconds(offset_secs);
        }
        return now;
    }

    if current_time < window_start {
        let today = now.date_naive();
        let scheduled_naive = today.and_time(window_start);
        return DateTime::from_naive_utc_and_offset(scheduled_naive, Utc);
    }

    let tomorrow = now.date_naive() + chrono::Days::new(1);
    let scheduled_naive = tomorrow.and_time(window_start);
    DateTime::from_naive_utc_and_offset(scheduled_naive, Utc)
}
