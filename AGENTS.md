# Repository instructions

- This is a local Tauri desktop application, not a hosted website. Browser mode is explicitly read-only preview data.
- Keep contracts free of business dependencies. Rust feature packages live under modules; UI feature components live under src/modules.
- Never query another module's storage namespace. Use its public API with a shared UnitOfWork when atomic coordination is needed.
- Scheduler and impact analysis remain pure: no database, network, notifications, or LLM calls.
- Never infer task completion from timer expiry. Never silently relax deadlines, locked blocks, or protected life periods.
- Every new constraint must be interpreted and validated; reject unknown constraints and unknown task references.
- Any plan or AI write must validate its input version. Preserve command idempotency and restore-time version monotonicity.
- Keep modules/<id>/manifest.json and modules.json synchronized. Optional modules must degrade without disabling local planning.
- Validate with npm run build, npm test, npm run check:rust, Rust formatting, and clippy. Use the isolated database directory for UI tests.
