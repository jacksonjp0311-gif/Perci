# Archived candidate receipts (2026-10-17)

These files were in `models/candidates/` but nothing in the README, docs,
`VALIDATION.md`, `WEIGHTS.md`, the model manifests/ledger, `src/`, `scripts/`
or `training/` refers to them. They were moved here by
`scripts/archive_candidates.py --apply` (with `git mv`, so history is kept and
the five `.bdlg` files stay in Git LFS). Nothing was deleted.

To restore one: `git mv archive/candidates-2026-10-17/<file> models/candidates/<file>`.
`python scripts/archive_candidates.py --check` exits 1 if unreferenced files
appear in `models/candidates/` again.
