from __future__ import annotations

import ast
import re
from pathlib import Path
from typing import Any

from .models import Edge

IMPORT_PATTERNS = {
    "javascript": [
        re.compile(r"(?:import\s+.*?\s+from\s+|import\s*\(|require\s*\()\s*['\"]([^'\"]+)['\"]"),
    ],
    "typescript": [
        re.compile(r"(?:import\s+.*?\s+from\s+|import\s*\(|require\s*\()\s*['\"]([^'\"]+)['\"]"),
    ],
    "powershell": [
        re.compile(r"(?:Import-Module|\.\s+)\s*['\"]?([^'\"\s;]+)", re.IGNORECASE),
    ],
    "shell": [
        re.compile(r"(?:source|\.)\s+['\"]?([^'\"\s;]+)"),
    ],
    "go": [re.compile(r'^\s*import\s+(?:\([^)]*?"([^"]+)"|"([^"]+)")', re.MULTILINE | re.DOTALL)],
    "rust": [re.compile(r"^\s*(?:use|mod)\s+([A-Za-z0-9_:]+)", re.MULTILINE)],
    "java": [re.compile(r"^\s*import\s+([A-Za-z0-9_.*]+)", re.MULTILINE)],
    "kotlin": [re.compile(r"^\s*import\s+([A-Za-z0-9_.]*)", re.MULTILINE)],
    "csharp": [re.compile(r"^\s*using\s+([A-Za-z0-9_.]+)", re.MULTILINE)],
}

SYMBOL_PATTERNS = {
    "javascript": re.compile(r"^\s*(?:export\s+)?(?:async\s+)?(?:function|class)\s+([A-Za-z_$][\w$]*)", re.MULTILINE),
    "typescript": re.compile(r"^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function|class|interface|type|enum)\s+([A-Za-z_$][\w$]*)", re.MULTILINE),
    "powershell": re.compile(r"^\s*function\s+([A-Za-z0-9_-]+)", re.MULTILINE | re.IGNORECASE),
    "shell": re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(\)\s*\{", re.MULTILINE),
    "go": re.compile(r"^\s*(?:func|type)\s+(?:\([^)]*\)\s*)?([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE),
    "rust": re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|trait)\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE),
    "java": re.compile(r"^\s*(?:public|private|protected|static|final|abstract|\s)+\s*(?:class|interface|enum|[\w<>\[\]]+)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:\(|\{|extends|implements)", re.MULTILINE),
    "kotlin": re.compile(r"^\s*(?:internal\s+|private\s+|public\s+|protected\s+|open\s+|abstract\s+|sealed\s+|data\s+|inline\s+|companion\s+)*?(?:class|object|interface|enum\s+class|fun|val|var)\s+([A-Za-z_][A-Za-z0-9_<>*]*)", re.MULTILINE),
    "csharp": re.compile(r"^\s*(?:public|private|protected|internal|static|sealed|abstract|partial|\s)+\s*(?:class|interface|enum|record|[\w<>\[\]]+)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:\(|\{|:)", re.MULTILINE),
}

MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)#]+)(?:#[^)]+)?\)")
PATH_REFERENCE = re.compile(
    r"(?<![A-Za-z0-9_])((?:[A-Za-z0-9_.-]+/)+[A-Za-z0-9_.-]+\.(?:py|js|ts|tsx|jsx|md|json|yaml|yml|toml|ps1|sh|rs|go|java|kt|kts|cs))(?![A-Za-z0-9_])"
)


def language_for(path: Path) -> str:
    name = path.name.lower()
    suffix = path.suffix.lower()
    if suffix in {".py", ".pyi"}:
        return "python"
    if suffix in {".js", ".jsx", ".mjs", ".cjs"}:
        return "javascript"
    if suffix in {".ts", ".tsx"}:
        return "typescript"
    if suffix in {".ps1", ".psm1", ".psd1"}:
        return "powershell"
    if suffix in {".sh", ".bash", ".zsh", ".fish"}:
        return "shell"
    if suffix == ".go":
        return "go"
    if suffix == ".rs":
        return "rust"
    if suffix in {".java"}:
        return "java"
    if suffix in {".kt", ".kts"}:
        return "kotlin"
    if suffix in {".cs", ".fs", ".fsx"}:
        return "csharp"
    if suffix in {".md", ".mdx", ".rst", ".adoc", ".txt"}:
        return "documentation"
    if suffix in {".json", ".jsonc", ".yaml", ".yml", ".toml", ".ini", ".cfg", ".conf", ".xml"}:
        return "configuration"
    if name == "dockerfile" or suffix == ".dockerfile":
        return "docker"
    if name in {"makefile", "justfile"}:
        return "build"
    return suffix.lstrip(".") or "text"


def classify_file(path: Path, relative: str, runtime_hints: set[str]) -> str:
    parts = {part.lower() for part in Path(relative).parts}
    name = path.name.lower()
    if parts & runtime_hints:
        return "runtime_evidence"
    if "test" in parts or "tests" in parts or name.startswith("test_") or name.endswith((".test.js", ".spec.ts", ".spec.js")):
        return "test"
    language = language_for(path)
    if language == "documentation":
        return "documentation"
    if language in {"configuration", "docker", "build"}:
        return "configuration"
    return "source"


def _python_parse(text: str, path: str) -> tuple[list[dict[str, Any]], list[Edge]]:
    symbols: list[dict[str, Any]] = []
    edges: list[Edge] = []
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return symbols, edges

    class Visitor(ast.NodeVisitor):
        def __init__(self) -> None:
            self.scope: list[str] = []

        def _symbol(self, node: ast.AST, name: str, kind: str, signature: str = "") -> None:
            qualified = ".".join([*self.scope, name])
            symbols.append({
                "name": name,
                "qualified_name": qualified,
                "symbol_kind": kind,
                "start_line": getattr(node, "lineno", 1),
                "end_line": getattr(node, "end_lineno", getattr(node, "lineno", 1)),
                "signature": signature,
            })

        def visit_ClassDef(self, node: ast.ClassDef) -> None:
            self._symbol(node, node.name, "class")
            self.scope.append(node.name)
            self.generic_visit(node)
            self.scope.pop()

        def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
            args = [arg.arg for arg in node.args.args]
            self._symbol(node, node.name, "function", f"{node.name}({', '.join(args)})")
            self.scope.append(node.name)
            self.generic_visit(node)
            self.scope.pop()

        visit_AsyncFunctionDef = visit_FunctionDef

        def visit_Import(self, node: ast.Import) -> None:
            for alias in node.names:
                edges.append(Edge(path, alias.name, "imports", 0.95, f"line {node.lineno}"))

        def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
            module = "." * node.level + (node.module or "")
            edges.append(Edge(path, module, "imports", 0.95, f"line {node.lineno}"))

        def visit_Call(self, node: ast.Call) -> None:
            target = ""
            if isinstance(node.func, ast.Name):
                target = node.func.id
            elif isinstance(node.func, ast.Attribute):
                parts: list[str] = []
                current: ast.AST = node.func
                while isinstance(current, ast.Attribute):
                    parts.append(current.attr)
                    current = current.value
                if isinstance(current, ast.Name):
                    parts.append(current.id)
                target = ".".join(reversed(parts))
            if target:
                source = f"{path}::{'.'.join(self.scope)}" if self.scope else path
                edges.append(Edge(source, target, "calls", 0.55, f"line {node.lineno}"))
            self.generic_visit(node)

    Visitor().visit(tree)
    return symbols, edges


def parse_structure(text: str, path: str, language: str) -> tuple[list[dict[str, Any]], list[Edge]]:
    if language == "python":
        return _python_parse(text, path)

    symbols: list[dict[str, Any]] = []
    edges: list[Edge] = []
    pattern = SYMBOL_PATTERNS.get(language)
    if pattern:
        for match in pattern.finditer(text):
            name = match.group(1)
            line = text.count("\n", 0, match.start()) + 1
            symbols.append({
                "name": name,
                "qualified_name": name,
                "symbol_kind": "symbol",
                "start_line": line,
                "end_line": line,
                "signature": match.group(0).strip()[:240],
            })

    for import_pattern in IMPORT_PATTERNS.get(language, []):
        for match in import_pattern.finditer(text):
            target = next((group for group in match.groups() if group), "")
            if target:
                line = text.count("\n", 0, match.start()) + 1
                edges.append(Edge(path, target, "imports", 0.75, f"line {line}"))

    if language == "documentation":
        for match in MARKDOWN_LINK.finditer(text):
            target = match.group(1)
            if not target.startswith(("http://", "https://", "mailto:")):
                line = text.count("\n", 0, match.start()) + 1
                edges.append(Edge(path, target, "documents", 0.80, f"line {line}"))

    for match in PATH_REFERENCE.finditer(text):
        target = match.group(1)
        line = text.count("\n", 0, match.start()) + 1
        edges.append(Edge(path, target, "references", 0.65, f"line {line}"))

    return symbols, edges
