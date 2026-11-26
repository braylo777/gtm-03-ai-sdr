//! Waterfall enrichment. For each lead and each wanted field, ask providers in order and
//! keep the first valid answer. A provider is charged once per lead when it is queried.
use crate::util::*;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub fn valid_email(e: &str) -> bool {
    let Some((l, d)) = e.split_once('@') else { return false };
    !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !e.contains(' ') && e.matches('@').count() == 1
}

pub fn waterfall_op(i: &Value) -> R<Value> {
    let want: Vec<&str> = arr(i, "fields")?.iter().filter_map(Value::as_str).collect();
    let providers = arr(i, "providers")?;
    let budget = f_or(i, "credit_budget", f64::INFINITY);
    let mut spent_by: BTreeMap<String, f64> = BTreeMap::new();
    let mut total = 0.0;
    let mut leads_out = vec![];
    let mut fill: BTreeMap<String, usize> = BTreeMap::new();
    for lead in arr(i, "leads")? {
        let key = s(lead, "domain")?;
        let mut rec: Map<String, Value> = lead.as_object().cloned().unwrap_or_default();
        let mut source = Map::new();
        let mut queried = vec![];
        for p in providers {
            let name = s(p, "name")?;
            let missing: Vec<&str> = want.iter().copied().filter(|f| rec.get(*f).map_or(true, |v| v.is_null() || v == "")).collect();
            if missing.is_empty() {
                break;
            }
            let cost = f(p, "cost_per_lookup")?;
            if total + cost > budget {
                break;
            }
            total += cost;
            *spent_by.entry(name.to_string()).or_default() += cost;
            queried.push(name.to_string());
            let data = p.get("data").and_then(|d| d.get(key));
            for fld in missing {
                if let Some(v) = data.and_then(|d| d.get(fld)) {
                    if v.is_null() || v == "" || (fld == "email" && !v.as_str().map_or(false, valid_email)) {
                        continue;
                    }
                    rec.insert(fld.to_string(), v.clone());
                    source.insert(fld.to_string(), json!(name));
                    *fill.entry(fld.to_string()).or_default() += 1;
                }
            }
        }
        let still: Vec<&str> = want.iter().copied().filter(|f| rec.get(*f).map_or(true, |v| v.is_null() || v == "")).collect();
        leads_out.push(json!({"lead": rec, "field_source": source, "providers_queried": queried, "missing": still}));
    }
    let n = leads_out.len().max(1) as f64;
    let rates: Map<String, Value> = want.iter().map(|f| (f.to_string(), json!(rd(*fill.get(*f).unwrap_or(&0) as f64 / n * 100.0, 1)))).collect();
    Ok(json!({"leads": leads_out, "credits_spent": rd(total, 2), "credits_by_provider": spent_by, "fill_rate_pct_from_providers": rates}))
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn waterfall_stops_when_filled_and_rejects_bad_email() {
        let i = json!({"fields":["email","headcount"],"leads":[{"domain":"a.io"}],"providers":[
            {"name":"p1","cost_per_lookup":1,"data":{"a.io":{"email":"bad email","headcount":50}}},
            {"name":"p2","cost_per_lookup":2,"data":{"a.io":{"email":"ceo@a.io"}}},
            {"name":"p3","cost_per_lookup":5,"data":{"a.io":{"email":"x@a.io"}}}]});
        let r = waterfall_op(&i).unwrap();
        assert_eq!(r["leads"][0]["lead"]["email"], "ceo@a.io");
        assert_eq!(r["credits_spent"], 3.0);
        assert_eq!(r["leads"][0]["providers_queried"], json!(["p1","p2"]));
    }
    #[test]
    fn budget_caps_spend() {
        let i = json!({"fields":["email"],"credit_budget":1,"leads":[{"domain":"a.io"}],"providers":[{"name":"p","cost_per_lookup":2,"data":{}}]});
        assert_eq!(waterfall_op(&i).unwrap()["credits_spent"], 0.0);
    }
}
