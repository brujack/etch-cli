"""Source check: EnvGuard users must be serialised `#[test]` bodies, bound to a name.

`scan` masks Rust string, char and comment contents, then applies four rules to the
masked text (spec Design 6, R16). The brace counter works on masked text, so a literal
such as "not: valid: yaml: {{{" cannot unbalance it.
"""

import re
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parent.parent
HELPERS_PATH = "lib/src/test_helpers.rs"

_IDENT = re.compile(r"[A-Za-z0-9_]")
_RAW_START = re.compile(r'b?r(#*)"')
_ENV_GUARD = re.compile(r"\bEnvGuard\b")
_ALIAS = re.compile(r"\bEnvGuard\s+as\b")
_LET_UNDERSCORE = re.compile(r"\blet\s+_\s*(?::[^=;]*)?=\s*(?:\w+\s*::\s*)*EnvGuard\b")
_DROP_CALL = re.compile(r"\bdrop\s*\(\s*(?:\w+\s*::\s*)*EnvGuard\b")
_FN = re.compile(r"\bfn\s+(\w+)")
_IMPL = re.compile(r"\bimpl\s+(?:Drop\s+for\s+)?EnvGuard\s*\{")
_SERIAL = re.compile(r"#\s*\[\s*(?:serial_test\s*::\s*)?serial\b")
_TEST_ATTR = re.compile(r"#\s*\[\s*test\s*\]")


def _blank(text: str) -> str:
    """Replace every non-newline character with a space."""
    return "".join(c if c == "\n" else " " for c in text)


def _line_comment_end(src: str, i: int) -> int:
    end = src.find("\n", i)
    return len(src) if end == -1 else end


def _block_comment_end(src: str, i: int) -> int:
    """Return the index after a (nestable) block comment starting at `i`."""
    depth = 0
    while i < len(src):
        if src.startswith("/*", i):
            depth += 1
            i += 2
        elif src.startswith("*/", i):
            depth -= 1
            i += 2
            if depth == 0:
                return i
        else:
            i += 1
    return i


def _string_end(src: str, i: int) -> int:
    """Return the index after a "..." string whose opening quote is at `i`."""
    i += 1
    while i < len(src) and src[i] != '"':
        i += 2 if src[i] == "\\" else 1
    return min(i + 1, len(src))


def _raw_string_end(src: str, hashes: str, quote: int) -> int:
    end = src.find('"' + hashes, quote + 1)
    return len(src) if end == -1 else end + 1 + len(hashes)


def _char_end(src: str, i: int) -> int | None:
    """Return the end of a char literal at `i`, or None for a lifetime."""
    if src.startswith("\\", i + 1):
        end = src.find("'", i + 3)
        return None if end == -1 else end + 1
    if src.startswith("'", i + 2):
        return i + 3
    return None


def _literal_end(src: str, i: int) -> int | None:
    """Return the end of a string/char/comment starting at `i`, else None."""
    if src.startswith("//", i):
        return _line_comment_end(src, i)
    if src.startswith("/*", i):
        return _block_comment_end(src, i)
    if src[i] == '"':
        return _string_end(src, i)
    if src[i] == "'":
        return _char_end(src, i)
    raw = _RAW_START.match(src, i)
    if raw and (i == 0 or not _IDENT.match(src[i - 1])):
        return _raw_string_end(src, raw.group(1), raw.end() - 1)
    return None


def mask(src: str) -> str:
    """Blank out string, char and comment contents, preserving offsets and newlines."""
    out: list[str] = []
    i = 0
    while i < len(src):
        end = _literal_end(src, i)
        if end is None:
            out.append(src[i])
            i += 1
        else:
            out.append(_blank(src[i:end]))
            i = end
    return "".join(out)


def _match_brace(masked: str, open_idx: int) -> int:
    """Return the index of the `}` closing the `{` at `open_idx` (or len on imbalance)."""
    depth = 0
    for i in range(open_idx, len(masked)):
        if masked[i] == "{":
            depth += 1
        elif masked[i] == "}":
            depth -= 1
            if depth == 0:
                return i
    return len(masked)


def _attributes(masked: str, fn_start: int) -> str:
    """Return the text between the previous item boundary and the `fn` keyword."""
    boundary = max(masked.rfind(c, 0, fn_start) for c in ";{}")
    return masked[boundary + 1 : fn_start]


def _functions(masked: str) -> list[tuple[str, int, str, str, int]]:
    """Return (name, line, attributes, body, start offset) for each fn that has a body."""
    found = []
    for m in _FN.finditer(masked):
        brace = masked.find("{", m.end())
        semi = masked.find(";", m.end())
        if brace == -1 or (semi != -1 and semi < brace):
            continue
        body = masked[brace : _match_brace(masked, brace) + 1]
        line = masked.count("\n", 0, m.start()) + 1
        found.append(
            (m.group(1), line, _attributes(masked, m.start()), body, m.start())
        )
    return found


def _exempt_ranges(masked: str, path: str) -> list[tuple[int, int]]:
    """Return the `impl EnvGuard` / `impl Drop for EnvGuard` spans, for the helper file only."""
    if path != HELPERS_PATH:
        return []
    return [
        (m.start(), _match_brace(masked, m.end() - 1)) for m in _IMPL.finditer(masked)
    ]


def _function_violations(masked: str, path: str) -> list[str]:
    exempt = _exempt_ranges(masked, path)
    out = []
    for name, line, attrs, body, start in _functions(masked):
        if not _ENV_GUARD.search(body) or any(a <= start <= b for a, b in exempt):
            continue
        where = f"{path}:{line}: fn {name}"
        if not _TEST_ATTR.search(attrs):
            out.append(f"{where} names EnvGuard but is not a #[test]")
        elif not _SERIAL.search(attrs):
            out.append(f"{where} names EnvGuard without #[serial]")
    return out


def _line_of(masked: str, offset: int) -> int:
    return masked.count("\n", 0, offset) + 1


def _text_violations(masked: str, path: str) -> list[str]:
    rules = (
        (_ALIAS, "EnvGuard imported under another name"),
        (_LET_UNDERSCORE, "`let _ =` drops EnvGuard at once"),
        (_DROP_CALL, "drop() of EnvGuard"),
    )
    return [
        f"{path}:{_line_of(masked, m.start())}: {message}"
        for pattern, message in rules
        for m in pattern.finditer(masked)
    ]


def scan(source: str, path: str) -> list[str]:
    """Return the EnvGuard violations found in `source`, as `path:line: message`."""
    masked = mask(source)
    return _function_violations(masked, path) + _text_violations(masked, path)


MUST_FLAG = {
    "alias": "use crate::test_helpers::EnvGuard as G;\n",
    "non_test_helper": 'fn with_path() {\n    let _g = EnvGuard::set("A", "b");\n}\n',
    "test_without_serial": (
        '#[test]\nfn t() {\n    let _g = EnvGuard::set("A", "b");\n}\n'
    ),
    "let_underscore": (
        '#[test]\n#[serial]\nfn t() {\n    let _ = EnvGuard::set("A", "b");\n}\n'
    ),
    "let_underscore_qualified": (
        "#[test]\n#[serial]\nfn t() {\n"
        '    let _ = crate::test_helpers::EnvGuard::set("A", "b");\n}\n'
    ),
    "drop_call": (
        '#[test]\n#[serial]\nfn t() {\n    drop(EnvGuard::set("A", "b"));\n}\n'
    ),
}

MUST_NOT_FLAG = {
    "serial_before_test": (
        '#[serial]\n#[test]\nfn t() {\n    let _g = EnvGuard::set("A", "b");\n}\n'
    ),
    "serial_with_key": (
        '#[test]\n#[serial(path)]\nfn t() {\n    let _g = EnvGuard::set("A", "b");\n}\n'
    ),
    "serial_qualified": (
        "#[test]\n#[serial_test::serial]\nfn t() {\n"
        '    let _g = EnvGuard::set("A", "b");\n}\n'
    ),
    "module_use": "mod tests {\n    use crate::test_helpers::EnvGuard;\n}\n",
    "closure_in_test": (
        '#[test]\n#[serial]\nfn t() {\n    let _g = EnvGuard::remove("A");\n'
        "    let f = || { 1 + 1 };\n    f();\n}\n"
    ),
    "unbalanced_brace_string": (
        'fn data() -> &\'static str { "not: valid: yaml: {{{" }\n'
        '#[test]\n#[serial]\nfn t() {\n    let _g = EnvGuard::set("A", "b");\n}\n'
    ),
    "raw_string_and_comments": (
        'fn data() { let _s = r#"}}} "EnvGuard""#; /* } */ // {\n let _c = \'{\'; }\n'
        '#[test]\n#[serial]\nfn t() {\n    let _g = EnvGuard::set("A", "b");\n}\n'
    ),
    "lifetime_not_char": (
        "fn id<'a>(x: &'a str) -> &'a str { x }\n"
        '#[test]\n#[serial]\nfn t() {\n    let _g = EnvGuard::set("A", "b");\n}\n'
    ),
    "name_in_string_or_comment": ('fn helper() { let _s = "EnvGuard"; } // EnvGuard\n'),
}

HELPER_IMPLS = """\
impl EnvGuard {
    pub fn set(key: &str) -> Self {
        let previous = None;
        EnvGuard { key: key.into(), previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        apply(&self.key, self.previous.as_deref());
    }
}
"""


@pytest.mark.parametrize("name", sorted(MUST_FLAG))
def test_must_flag(name: str) -> None:
    assert scan(MUST_FLAG[name], "lib/src/x.rs"), f"{name} was not flagged"


@pytest.mark.parametrize("name", sorted(MUST_NOT_FLAG))
def test_must_not_flag(name: str) -> None:
    assert scan(MUST_NOT_FLAG[name], "lib/src/x.rs") == []


def test_each_must_flag_fixture_fires_its_own_rule() -> None:
    expected = {
        "alias": "another name",
        "non_test_helper": "is not a #[test]",
        "test_without_serial": "without #[serial]",
        "let_underscore": "`let _ =`",
        "let_underscore_qualified": "`let _ =`",
        "drop_call": "drop()",
    }
    for name, fragment in expected.items():
        hits = scan(MUST_FLAG[name], "lib/src/x.rs")
        assert any(fragment in h for h in hits), (name, hits)


def test_impls_exempt_only_in_helper_file() -> None:
    assert scan(HELPER_IMPLS, HELPERS_PATH) == []
    assert scan(HELPER_IMPLS, "lib/src/other.rs")


def test_helper_file_still_checks_other_functions() -> None:
    source = HELPER_IMPLS + 'fn sneaky() { let _g = EnvGuard::remove("A"); }\n'
    assert scan(source, HELPERS_PATH)


def test_mask_preserves_length_and_newlines() -> None:
    src = 'let a = "x\\"{";\n// c {\nlet b = \'{\';\n'
    masked = mask(src)
    assert len(masked) == len(src)
    assert masked.count("\n") == src.count("\n")
    assert "{" not in masked


def test_real_tree_is_clean() -> None:
    files = sorted((REPO / "lib/src").rglob("*.rs"))
    assert files, "no Rust sources found; the scan would pass by matching nothing"
    violations = []
    for f in files:
        rel = f.relative_to(REPO).as_posix()
        violations += scan(f.read_text(encoding="utf-8"), rel)
    assert violations == []
