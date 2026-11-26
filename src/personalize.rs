//! Personalized first-touch email. The hook comes from the freshest, highest-priority signal.
//! The draft must pass lint: no `{{var}}` left, under the word limit, no spam words, one CTA.
use crate::util::*;
use serde_json::{json, Map, Value};

const SPAM: &[&str] = &["free!!!", "guarantee", "act now", "risk-free", "100% free", "no obligation", "click here", "limited time", "$$$"];

pub fn fill(tpl: &str, vars: &Map<String, Value>) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut rest = tpl;
    let mut missing = vec![];
    while let Some(a) = rest.find("{{") {
        out.push_str(&rest[..a]);
        let after = &rest[a + 2..];
        match after.find("}}") {
            None => {
                out.push_str(&rest[a..]);
                rest = "";
                break;
            }
            Some(b) => {
                let k = after[..b].trim();
                match vars.get(k) {
                    Some(Value::String(s)) if !s.is_empty() => out.push_str(s),
                    Some(v) if !v.is_null() && !v.is_string() => out.push_str(&v.to_string()),
                    _ => {
                        missing.push(k.to_string());
                        out.push_str(&format!("{{{{{k}}}}}"));
                    }
                }
                rest = &after[b + 2..];
            }
        }
    }
    out.push_str(rest);
    (out, missing)
}

pub fn email_op(i: &Value) -> R<Value> {
    let p = i.get("prospect").ok_or("`prospect` is required")?;
    let mut vars = p.as_object().cloned().ok_or("`prospect` must be an object")?;
    let hooks = obj(i, "hook_library")?;
    let order: Vec<&str> = arr(i, "hook_priority")?.iter().filter_map(Value::as_str).collect();
    let max_age = f_or(i, "max_signal_age_days", 60.0);
    let mut chosen = None;
    for kind in &order {
        if let Some(sig) = arr(p, "signals").ok().and_then(|a| a.iter().find(|x| x.get("type").and_then(Value::as_str) == Some(*kind) && f_or(x, "age_days", 1e9) <= max_age)) {
            chosen = Some((kind.to_string(), sig.clone()));
            break;
        }
    }
    let (hook_type, hook) = match &chosen {
        Some((k, sig)) => {
            let tpl = hooks.get(k).and_then(Value::as_str).ok_or(format!("no hook text for `{k}`"))?;
            let mut v2 = vars.clone();
            if let Some(o) = sig.as_object() {
                for (a, b) in o {
                    v2.insert(a.clone(), b.clone());
                }
            }
            (k.clone(), fill(tpl, &v2).0)
        }
        None => ("generic".to_string(), hooks.get("generic").and_then(Value::as_str).unwrap_or("").to_string()),
    };
    vars.insert("hook".into(), json!(hook));
    let (subject, m1) = fill(s(i, "subject_template")?, &vars);
    let (body, m2) = fill(s(i, "body_template")?, &vars);
    let words = body.split_whitespace().count();
    let max_words = f_or(i, "max_words", 120.0) as usize;
    let low = body.to_lowercase();
    let spam: Vec<&str> = SPAM.iter().copied().filter(|w| low.contains(w)).collect();
    let ctas = body.matches('?').count();
    let mut issues = vec![];
    let mut missing: Vec<String> = m1.into_iter().chain(m2).collect();
    missing.dedup();
    if !missing.is_empty() {
        issues.push(format!("unresolved variables: {}", missing.join(", ")));
    }
    if words > max_words {
        issues.push(format!("{words} words > limit {max_words}"));
    }
    if !spam.is_empty() {
        issues.push(format!("spam words: {}", spam.join(", ")));
    }
    if ctas != 1 {
        issues.push(format!("expected exactly 1 question / CTA, found {ctas}"));
    }
    Ok(json!({"hook_type": hook_type, "subject": subject, "body": body, "words": words, "ready_to_queue": issues.is_empty(), "lint_issues": issues}))
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn fill_reports_missing() {
        let v = json!({"a":"x","n":3}).as_object().cloned().unwrap();
        assert_eq!(fill("{{a}}-{{ n }}-{{b}}", &v), ("x-3-{{b}}".into(), vec!["b".to_string()]));
    }
    #[test]
    fn picks_priority_hook_and_lints() {
        let i = json!({"hook_priority":["funding","hiring"],"hook_library":{"funding":"Congrats on the {{round}}.","hiring":"Saw you're hiring."},
            "prospect":{"first_name":"Dana","signals":[{"type":"hiring","age_days":3},{"type":"funding","round":"Series B","age_days":90}]},
            "max_signal_age_days":60,"subject_template":"{{first_name}}, quick idea","body_template":"Hi {{first_name}}, {{hook}} Worth a chat?"});
        let r = email_op(&i).unwrap();
        assert_eq!(r["hook_type"], "hiring");
        assert_eq!(r["ready_to_queue"], true);
        let mut j = i.clone();
        j["body_template"] = json!("Hi {{nick}}! Act now? Click here?");
        assert_eq!(email_op(&j).unwrap()["lint_issues"].as_array().unwrap().len(), 3);
    }
}
