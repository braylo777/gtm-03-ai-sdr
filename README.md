# gtm-03-ai-sdr

**GTM Engineering — Stage 3 of 12: AI SDRs & Hyper-Personalization**

The guardrails behind an AI SDR: waterfall enrichment with a credit budget, signal-based personal hooks with a draft linter, and multi-step sequences that respect time zones, weekends and mailbox caps.

**Live demo:** https://gtm-03-ai-sdr.braylo777.workers.dev — the Rust core runs in your browser as WebAssembly. No server, no tracking.

## Why this matters

An AI SDR is only as good as its data and its brakes. Bad enrichment means bounced emails and burned domains. Generic hooks get ignored. A sequence that keeps emailing someone who replied, or sends 400 emails from one mailbox on a Monday, gets the domain blocked. This repo covers the deterministic parts that keep an AI SDR safe and relevant, before any LLM writes a word.

## What it does

- Clay-style waterfall: providers in order, per-field first valid answer, stop when filled, credit budget, per-provider spend, fill rate
- Email syntax check before accepting an enriched email
- Hook selection by priority and freshness (funding, hiring, tech change, content), with a generic fallback
- Template fill with {{variables}} and a lint gate: unresolved variables, word limit, spam words, exactly one CTA
- Sequence builder: business-day waits, Monday-based weeks, local send hour to UTC, mailbox daily cap with overflow, stop on reply / bounce / unsubscribe

## What it does not do (yet)

- No Clay / Apollo / scraping connection: provider responses are synthetic JSON
- No LLM call: the hook library is the place an LLM draft would plug in, and the linter is the gate it must pass
- Does not send email and does not check deliverability (SPF, DKIM, warm-up)

## Operations

Every operation takes JSON in and gives JSON out. The CLI, the tests and the demo page all use the same entry point.

| Operation | What it does |
|---|---|
| `waterfall_enrich` | Fill email, title and headcount from cheap providers first. A bad email from one provider is skipped and the next provider is tried. |
| `personalize_email` | Pick the freshest high-priority signal as the hook, fill the template and lint the draft. Edit the body to see the linter fail. |
| `build_sequence` | Schedule a 4-step sequence. Day 0 is a Monday. Weekends are skipped, the mailbox cap pushes overflow, and replied prospects get no steps. |

## Run it

You need Rust (stable). No Node, no npm.

```bash
git clone https://github.com/braylo777/gtm-03-ai-sdr
cd gtm-03-ai-sdr
cargo test
cargo run -- ops
```

Try the first demo example from the command line:

```bash
python3 -c 'import json;print(json.dumps(json.load(open("web/examples.json"))["ops"][0]["input"]))' | cargo run -q -- waterfall_enrich
```

## Build and deploy the demo

```bash
./build.sh    # tests, then compiles the core to web/app.wasm
./deploy.sh   # serves web/ as a Cloudflare Workers static-assets site (needs a Cloudflare login)
```

`web/` is a plain static site (`index.html`, `app.js`, `app.wasm`). It has no build step, so any static host works.

## Layout

```
src/lib.rs        operation list and router
src/api.rs        JSON entry point + the 3 WASM exports (alloc, dealloc, run)
src/main.rs       CLI
src/*.rs          one module per capability, each with unit tests
tests/            integration tests (every demo example must pass)
web/              demo page; examples.json drives both the page and this README
```

## Design choices

- **Rust, few dependencies.** Only `serde_json`, pinned to an exact version with a committed `Cargo.lock`.
- **Deterministic.** Time, randomness and keys are inputs, so every result can be replayed in a test or an audit.
- **Fail closed.** Unknown input or missing fields give an error, never a silent default.

## Part of a series

This is one of 12 repos that work through a public Go-To-Market (GTM) Engineering roadmap: [12 stages by Suraj Sharma (@suraj_sharma14)](https://x.com/suraj_sharma14/status/2125552583452270692). The roadmap is the inspiration; the code here is original. All 12: [github.com/braylo777?q=gtm-](https://github.com/braylo777?tab=repositories&q=gtm-).

## Data and privacy

Every example uses synthetic data. Nothing calls a paid API, scrapes a site, identifies a real visitor or sends an email.

## License

MIT — see [LICENSE](LICENSE). Built by [Braylo SI](https://braylo.si).
