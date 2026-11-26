//! Sequence scheduling. Day 0 is a Monday. Steps wait `wait_business_days` after the previous
//! step; weekends are skipped. Each mailbox has a daily cap; overflow moves to the next
//! business day. Prospects who replied, bounced or unsubscribed get no further steps.
use crate::util::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn is_weekend(day: i64) -> bool {
    day.rem_euclid(7) >= 5
}
fn next_business(mut d: i64) -> i64 {
    while is_weekend(d) {
        d += 1;
    }
    d
}
fn add_business(mut d: i64, n: i64) -> i64 {
    let mut k = 0;
    while k < n {
        d += 1;
        if !is_weekend(d) {
            k += 1;
        }
    }
    next_business(d)
}
const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

pub fn sequence_op(i: &Value) -> R<Value> {
    let steps = arr(i, "steps")?;
    let cap = f_or(i, "mailbox_daily_cap", 50.0) as usize;
    if cap == 0 {
        return Err("mailbox_daily_cap must be > 0".into());
    }
    let start = f(i, "start_day")? as i64;
    let send_hour = f_or(i, "local_send_hour", 9.0);
    let mut load: BTreeMap<(String, i64), usize> = BTreeMap::new();
    let mut out = vec![];
    let mut stopped = vec![];
    for p in arr(i, "prospects")? {
        let id = s(p, "id")?;
        let status = s_or(p, "status", "active");
        if ["replied", "bounced", "unsubscribed"].contains(&status) {
            stopped.push(json!({"id": id, "reason": status}));
            continue;
        }
        let tz = f_or(p, "utc_offset_hours", 0.0);
        let mbox = s_or(p, "mailbox", "default").to_string();
        let mut day = next_business(start);
        let mut plan = vec![];
        for (n, st) in steps.iter().enumerate() {
            if n > 0 {
                day = add_business(day, f(st, "wait_business_days")? as i64);
            }
            let ch = s_or(st, "channel", "email");
            if ch == "email" {
                while *load.get(&(mbox.clone(), day)).unwrap_or(&0) >= cap {
                    day = add_business(day, 1);
                }
                *load.entry((mbox.clone(), day)).or_default() += 1;
            }
            let utc = (send_hour - tz).rem_euclid(24.0);
            plan.push(json!({"step": n + 1, "channel": ch, "day": day, "weekday": DAYS[day.rem_euclid(7) as usize], "local_time": format!("{:02}:00", send_hour as i64), "utc_hour": rd(utc, 1)}));
        }
        out.push(json!({"id": id, "mailbox": mbox, "plan": plan}));
    }
    let peak = load.values().max().copied().unwrap_or(0);
    Ok(json!({"schedules": out, "stopped": stopped, "peak_mailbox_load": peak, "cap": cap}))
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn skips_weekends_caps_and_stops() {
        assert_eq!(add_business(4, 1), 7); // Fri + 1 = Mon
        let i = json!({"start_day":5,"mailbox_daily_cap":1,"steps":[{"channel":"email"},{"channel":"email","wait_business_days":2}],
            "prospects":[{"id":"a","mailbox":"m"},{"id":"b","mailbox":"m"},{"id":"c","status":"replied"}]});
        let r = sequence_op(&i).unwrap();
        assert_eq!(r["schedules"][0]["plan"][0]["day"], 7);
        assert_eq!(r["schedules"][1]["plan"][0]["day"], 8);
        assert_eq!(r["stopped"][0]["id"], "c");
        assert_eq!(r["peak_mailbox_load"], 1);
    }
}
