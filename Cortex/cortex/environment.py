from __future__ import annotations

from collections import Counter
from hashlib import sha256
import json
import os
from pathlib import Path
import platform
import re
from typing import Any


KNOWN_MANIFESTS: dict[str, str] = {
    "pyproject.toml": "python",
    "requirements.txt": "python",
    "requirements-dev.txt": "python",
    "setup.py": "python",
    "setup.cfg": "python",
    "Pipfile": "python",
    "poetry.lock": "python",
    "package.json": "node",
    "package-lock.json": "node",
    "pnpm-lock.yaml": "node",
    "yarn.lock": "node",
    "bun.lockb": "node",
    "Cargo.toml": "rust",
    "go.mod": "go",
    "pom.xml": "java",
    "build.gradle": "java",
    "build.gradle.kts": "java",
    "gradlew": "java",
    "Makefile": "build",
    "Justfile": "build",
    "Dockerfile": "container",
    "docker-compose.yml": "container",
    "docker-compose.yaml": "container",
    "compose.yml": "container",
    "compose.yaml": "container",
}


def _read_text(path: Path, limit: int = 250_000) -> str:
    try:
        if path.stat().st_size > limit:
            return ""
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def _package_json_profile(path: Path) -> dict[str, Any]:
    text = _read_text(path)
    if not text:
        return {}
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return {"parse_error": True}
    scripts = data.get("scripts") if isinstance(data.get("scripts"), dict) else {}
    dependencies: set[str] = set()
    for key in ("dependencies", "devDependencies", "peerDependencies"):
        value = data.get(key)
        if isinstance(value, dict):
            dependencies.update(str(name) for name in value)
    return {
        "name": data.get("name"),
        "main": data.get("main"),
        "type": data.get("type"),
        "scripts": {str(key): str(value) for key, value in scripts.items()},
        "dependencies": sorted(dependencies),
    }


def _python_profile(root: Path) -> dict[str, Any]:
    pyproject = root / "pyproject.toml"
    text = _read_text(pyproject)
    scripts: dict[str, str] = {}
    dependencies: list[str] = []
    if text:
        section = ""
        for raw_line in text.splitlines():
            line = raw_line.strip()
            if line.startswith("[") and line.endswith("]"):
                section = line.strip("[]").strip()
                continue
            if section == "project.scripts" and "=" in line and not line.startswith("#"):
                key, value = line.split("=", 1)
                scripts[key.strip()] = value.strip().strip('"\'')
        dependency_match = re.search(r"dependencies\s*=\s*\[(.*?)\]", text, re.DOTALL)
        if dependency_match:
            dependencies = re.findall(r"['\"]([^'\"]+)['\"]", dependency_match.group(1))
    return {"scripts": scripts, "dependencies": sorted(set(dependencies))}


def _detect_frameworks(root: Path, manifests: list[dict[str, str]]) -> list[str]:
    frameworks: set[str] = set()
    package_path = root / "package.json"
    if package_path.exists():
        package = _package_json_profile(package_path)
        deps = set(package.get("dependencies", []))
        for dependency, framework in {
            "react": "React",
            "next": "Next.js",
            "vue": "Vue",
            "nuxt": "Nuxt",
            "svelte": "Svelte",
            "@angular/core": "Angular",
            "express": "Express",
            "fastify": "Fastify",
            "nestjs": "NestJS",
            "vite": "Vite",
            "electron": "Electron",
        }.items():
            if dependency in deps:
                frameworks.add(framework)
    python_text = "\n".join(
        _read_text(root / name)
        for name in ("pyproject.toml", "requirements.txt", "requirements-dev.txt")
        if (root / name).exists()
    ).lower()
    for needle, framework in {
        "django": "Django",
        "fastapi": "FastAPI",
        "flask": "Flask",
        "pytest": "pytest",
        "pydantic": "Pydantic",
        "sqlalchemy": "SQLAlchemy",
        "streamlit": "Streamlit",
    }.items():
        if needle in python_text:
            frameworks.add(framework)
    if any(item["ecosystem"] == "rust" for item in manifests):
        cargo = _read_text(root / "Cargo.toml").lower()
        for needle, framework in {"tokio": "Tokio", "axum": "Axum", "actix": "Actix"}.items():
            if needle in cargo:
                frameworks.add(framework)
    return sorted(frameworks)


def _commands(root: Path, ecosystems: set[str]) -> dict[str, list[str]]:
    test: list[str] = []
    build: list[str] = []
    run: list[str] = []
    if "python" in ecosystems:
        if (root / "pytest.ini").exists() or (root / "tests").exists():
            test.append("python -m pytest")
            test.append("python -m unittest discover -s tests -v")
        build.append("python -m build")
        python_profile = _python_profile(root)
        for name in python_profile.get("scripts", {}):
            run.append(name)
    if "node" in ecosystems and (root / "package.json").exists():
        package = _package_json_profile(root / "package.json")
        scripts = package.get("scripts", {})
        package_manager = "npm"
        if (root / "pnpm-lock.yaml").exists():
            package_manager = "pnpm"
        elif (root / "yarn.lock").exists():
            package_manager = "yarn"
        for name in scripts:
            command = f"{package_manager} run {name}"
            if name == "test" or name.startswith("test:"):
                test.append(command)
            elif name == "build" or name.startswith("build:"):
                build.append(command)
            elif name in {"start", "dev", "serve"}:
                run.append(command)
    if "rust" in ecosystems:
        test.append("cargo test")
        build.append("cargo build")
        run.append("cargo run")
    if "go" in ecosystems:
        test.append("go test ./...")
        build.append("go build ./...")
        run.append("go run .")
    if "java" in ecosystems:
        if (root / "mvnw").exists() or (root / "pom.xml").exists():
            test.append("./mvnw test" if (root / "mvnw").exists() else "mvn test")
            build.append("./mvnw package" if (root / "mvnw").exists() else "mvn package")
        if (root / "gradlew").exists() or (root / "build.gradle").exists() or (root / "build.gradle.kts").exists():
            test.append("./gradlew test")
            build.append("./gradlew build")
    if (root / "Makefile").exists():
        build.append("make")
    return {
        "test": list(dict.fromkeys(test)),
        "build": list(dict.fromkeys(build)),
        "run": list(dict.fromkeys(run)),
    }


def learn_environment(root: Path, store: Any, repo: str) -> dict[str, Any]:
    """Learn a bounded, deterministic repository/runtime profile during bootstrap."""

    file_rows = store.files(repo)
    languages = Counter(
        row["language"] for row in file_rows if row["status"] == "indexed" and row["language"]
    )
    kinds = Counter(row["kind"] for row in file_rows if row["status"] == "indexed")
    manifests: list[dict[str, str]] = []
    for name, ecosystem in KNOWN_MANIFESTS.items():
        if (root / name).exists():
            manifests.append({"path": name, "ecosystem": ecosystem})
    for row in file_rows:
        path = row["path"].replace("\\", "/")
        if path.startswith(".github/workflows/") and path.endswith((".yml", ".yaml")):
            manifests.append({"path": path, "ecosystem": "ci:github-actions"})
    manifests = sorted({(item["path"], item["ecosystem"]) for item in manifests})
    manifest_records = [{"path": path, "ecosystem": ecosystem} for path, ecosystem in manifests]
    ecosystems = {item["ecosystem"].split(":", 1)[0] for item in manifest_records}
    language_ecosystems = {
        "python": "python",
        "javascript": "node",
        "typescript": "node",
        "rust": "rust",
        "go": "go",
        "java": "java",
        "csharp": "dotnet",
    }
    for language in languages:
        ecosystem = language_ecosystems.get(language)
        if ecosystem:
            ecosystems.add(ecosystem)
    commands = _commands(root, ecosystems)
    profile: dict[str, Any] = {
        "schema_version": "1.0",
        "repository": repo,
        "repository_root_name": root.name,
        "runtime": {
            "os": platform.system(),
            "os_release": platform.release(),
            "architecture": platform.machine(),
            "python": platform.python_version(),
            "python_implementation": platform.python_implementation(),
            "path_separator": os.sep,
        },
        "inventory": {
            "indexed_files": sum(row["status"] == "indexed" for row in file_rows),
            "languages": [
                {"name": language, "file_count": count}
                for language, count in sorted(languages.items(), key=lambda item: (-item[1], item[0]))
            ],
            "kinds": dict(sorted(kinds.items())),
        },
        "manifests": manifest_records,
        "ecosystems": sorted(ecosystems),
        "frameworks": _detect_frameworks(root, manifest_records),
        "commands": commands,
        "entrypoint_candidates": sorted(
            row["path"]
            for row in file_rows
            if row["status"] == "indexed"
            and Path(row["path"]).name.lower()
            in {
                "main.py", "app.py", "server.py", "manage.py", "index.js", "index.ts",
                "main.rs", "main.go", "program.cs", "dockerfile", "makefile",
            }
        )[:50],
        "capabilities": {
            "git_available": bool(store.commits(repo, 1)),
            "fts5_available": bool(
                store.db.execute(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='memories_fts'"
                ).fetchone()
            ),
            "offline_core": True,
            "powershell_launcher": True,
            "bash_launcher": True,
        },
    }
    canonical = json.dumps(profile, sort_keys=True, separators=(",", ":"))
    profile["profile_hash"] = sha256(canonical.encode("utf-8")).hexdigest()
    store.set_environment_profile(repo, profile)
    runtime_path = root / ".cortex" / "runtime" / "environment_latest.json"
    runtime_path.parent.mkdir(parents=True, exist_ok=True)
    runtime_path.write_text(json.dumps(profile, indent=2) + "\n", encoding="utf-8")
    profile["runtime_path"] = str(runtime_path)
    return profile


def environment_summary(profile: dict[str, Any] | None) -> dict[str, Any]:
    if not profile:
        return {"available": False}
    return {
        "available": True,
        "profile_hash": profile.get("profile_hash"),
        "ecosystems": profile.get("ecosystems", []),
        "frameworks": profile.get("frameworks", []),
        "languages": profile.get("inventory", {}).get("languages", [])[:8],
        "commands": profile.get("commands", {}),
        "entrypoint_candidates": profile.get("entrypoint_candidates", [])[:12],
        "runtime": profile.get("runtime", {}),
    }
