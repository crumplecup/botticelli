use botticelli_server::{Schedule, ScheduleCheck, ScheduleType};
use chrono::{Duration, Utc};

#[test]
fn test_schedule_check_constructors() {
    let now = Utc::now();
    let future = now + Duration::hours(1);

    let run_once = ScheduleCheck::run_once();
    assert!(run_once.should_run);
    assert!(run_once.next_run.is_none());

    let wait = ScheduleCheck::wait_until(future);
    assert!(!wait.should_run);
    assert_eq!(wait.next_run, Some(future));

    let run_and_schedule = ScheduleCheck::run_and_schedule(future);
    assert!(run_and_schedule.should_run);
    assert_eq!(run_and_schedule.next_run, Some(future));
}

#[test]
fn test_immediate_schedule() {
    let schedule = ScheduleType::Immediate;

    let check = schedule.check(None);
    assert!(check.should_run);
    assert!(check.next_run.is_none());

    let check2 = schedule.check(Some(Utc::now()));
    assert!(!check2.should_run);
}

#[test]
fn test_interval_schedule() {
    let schedule = ScheduleType::Interval { seconds: 3600 };

    let check = schedule.check(None);
    assert!(check.should_run);
    assert!(check.next_run.is_some());

    let now = Utc::now();
    let past = now - Duration::hours(2);
    let check2 = schedule.check(Some(past));
    assert!(check2.should_run);

    let future = now + Duration::hours(2);
    let check3 = schedule.check(Some(future));
    assert!(!check3.should_run);
}

#[test]
fn test_once_schedule() {
    let now = Utc::now();
    let future = now + Duration::hours(1);
    let schedule = ScheduleType::Once { at: future };

    let check = schedule.check(None);
    assert!(!check.should_run);
    assert_eq!(check.next_run, Some(future));

    let past = now - Duration::hours(1);
    let past_schedule = ScheduleType::Once { at: past };
    let check2 = past_schedule.check(None);
    assert!(check2.should_run);
    assert!(check2.next_run.is_none());
}

#[test]
fn test_cron_schedule() {
    let schedule = ScheduleType::Cron {
        expression: "0 0 9 * * * *".to_string(),
    };

    let check = schedule.check(None);
    assert!(check.should_run || check.next_run.is_some());

    let next = schedule.next_execution(Utc::now());
    assert!(next.is_some());
}

#[test]
fn test_invalid_cron() {
    let schedule = ScheduleType::Cron {
        expression: "invalid cron".to_string(),
    };

    let check = schedule.check(None);
    assert!(!check.should_run);
    assert!(check.next_run.is_none());

    let next = schedule.next_execution(Utc::now());
    assert!(next.is_none());
}

#[test]
fn test_schedule_serialization() {
    let schedules = vec![
        ScheduleType::Immediate,
        ScheduleType::Interval { seconds: 3600 },
        ScheduleType::Once { at: Utc::now() },
        ScheduleType::Cron {
            expression: "0 9 * * *".to_string(),
        },
    ];

    for schedule in schedules {
        let json = serde_json::to_string(&schedule).unwrap();
        let deserialized: ScheduleType = serde_json::from_str(&json).unwrap();
        assert_eq!(schedule, deserialized);
    }
}
