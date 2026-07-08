---
name: RC validation report
about: Report from building a real app on a release candidate (helps close WS-RC6)
title: "RC validation: <what you built>"
labels: rc-validation
---

Thanks for kicking the tires on a Gize release candidate! This report helps us reach the RC
acceptance criterion "a team outside the core builds an app on the RC" (see `docs/roadmap.md`).
Follow `docs/validation.md` in the repo if you want a scripted walkthrough.

## Context

- **Are you independent of the Gize core team?** (yes / no)
- `gize --version`:
- `rustc --version` / OS / database:

## What you built

<!-- One or two sentences: the app, its resources, which features you turned on. -->

## Surface exercised

- [ ] `gize new` (flags used: `--database` / `--openapi` / `--websocket` / `--api-version` / `--no-user`)
- [ ] `gize make crud` with at least one `belongs_to` relationship
- [ ] `gize createadmin` + login
- [ ] `gize migrate` (and `--status`)
- [ ] `gize serve` and called routes over HTTP
- [ ] Auth behaved as documented (401 without token, 403 non-admin on `users`, 201 with token)
- [ ] OpenAPI at `/docs` / `/openapi.json` (if enabled)
- [ ] `gize make admin` / admin UI (if used)
- [ ] `gize sync` after a hand edit (drift preserved)
- [ ] `gize check` + `gize fmt` clean on the generated project

## Acceptance criteria (roadmap RC)

- [ ] Generated project **compiles** and is **clippy/rustfmt-clean**
- [ ] The documented routes and auth behavior work end to end
- [ ] The docs (`docs/getting-started.md` / `docs/cookbook.md` / `docs/faq.md`) were enough to
      get unstuck

## Friction & papercuts

<!-- Anything confusing, missing from the docs, or that surprised you. Small things count. -->

## Verdict

<!-- Would you build a real service on this today? What would you need first? -->
