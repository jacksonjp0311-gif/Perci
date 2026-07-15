# Troubleshooting

## PowerShell script execution is blocked

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
```

Then run the script again in the same PowerShell window.

## Python cannot import Cortex

Run the all-one installer or reinstall from the Cortex root:

```powershell
.\.venv\Scripts\python.exe -m pip install -e .
```

```bash
./.venv/bin/python -m pip install -e .
```

## Repository wrapper points to an old engine location

Re-run bootstrap from the current engine. The wrapper bindings in `.cortex/config.json` will be refreshed.

## Repository wrapper opens the wrong Cortex database

Current wrappers are bound to the Cortex home and Python engine recorded during bootstrap. Re-run bootstrap when intentionally moving the database or engine. Unrelated inherited `CORTEX_HOME` or `CORTEX_PYTHON` values will not override an already bound repository wrapper.

## Activation is read-only

Inspect:

```bash
python -m cortex verify --repo MyProject --json
python -m cortex doctor --repo MyProject --json
```

Common causes are manifest drift with refresh disabled, a missing or degraded certificate, database integrity failure, incomplete integration files, or a broken neural ledger.

## Neural graph has zero synapses

Small repositories can verify with nodes and no relationships. For a larger repository, inspect the structural graph:

```bash
python -m cortex graph --repo MyProject --json
```

Re-run bootstrap with `--force` if parsers or source relationships changed.

## Embedded Cortex folder appears in host memory

Re-run bootstrap from the embedded engine. Current bootstrap adds the engine-relative path to the host repository exclusion list. Confirm the path in `.cortex/config.json` under `exclude`.

## FTS5 unavailable

Use a normal CPython distribution with SQLite FTS5. `doctor` reports availability.

## PowerShell 5.1 compatibility

The included scripts avoid PowerShell 7-only syntax. Use the provided launchers rather than copying Bash command forms into Windows PowerShell.
