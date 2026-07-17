# Perci Dark-Blood CLI

The interactive CLI is a presentation layer over the governed Perci runtime. It
does not hide uncertainty or convert visual confidence into capability claims.

After the runtime synchronizes, the launcher **clears the console** so PowerShell
copyright, cargo build lines, and prior noise fade out and the Dark-Blood banner
**snaps to the top** of the window. Chat also clears once on entry. Asset paths
are not printed under the banner (icons live under `assets/` for branding only).

```powershell
.\Start-Perci.cmd
.\Launch-Perci.ps1 -Mode intel
.\target\release\perci.exe classify "your prompt"
```

Palette roles are deep blood for structure, bright arterial red for the header
diamond / `PERCI` lettering, bone for content, iron for secondary detail, and
muted ash for labels. **Purple** is reserved for `◉ YOU  ›` and the words
`dark-blood` only. Redirected output and JSON stay uncolored. Override with
`PERCI_COLOR=always|never`; `NO_COLOR` is honored.

The `intel` / `/intel` probe reports the predicted and expected domain together
with top-two margin, chance-normalized overlap z-score, and Jaccard similarity.
It is a visible smoke test. The sealed held-out evaluation receipt remains the
stronger evidence artifact.

Interactive response headers include measured wall-clock elapsed time. This is
runtime evidence, not an artificial thinking animation. `/learning` exposes the
adaptive profile and pending-evidence path.
