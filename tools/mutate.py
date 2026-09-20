#!/usr/bin/env python3
"""Mutate an approved Scenario document and check that the Scenario test notices.

A Scenario test is only worth its green if a wrong figure fails it. This changes one figure at a
time in `docs/scenarios/<family>.md` and runs that Family's tests: a mutation the tests still pass
is an escape, and an escape means the comparison is not comparing (ADR-0010 as amended, Q192).

Every mutation runs in a throwaway `git worktree` checked out at `HEAD`, so the approved document
in the working copy is never touched — and so what is reviewed is what was committed, which is the
commit-per-Scenario boundary the build brief sets. Uncommitted work is not reviewed; commit first.
Research tooling, not product code: `core` stays pure (ADR-0004).

    python tools/mutate.py ar              # every Scenario with a test
    python tools/mutate.py ar AR-S04       # one Scenario
"""

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# A figure worth changing: an amount, a date, or a bare count. A `Q` before a number marks a
# design question (`Q150`), a reference and not a figure.
AMOUNT = re.compile(r"\d{1,3}(?:,\d{3})*\.\d{2}")
DATE = re.compile(r"(?<!\d)(\d{2})-(\d{2})(?!\d)")
COUNT = re.compile(r"(?<![\d,.\-Q])(\d{1,3})(?![\d,.:\-])")


def sections(doc: str, family: str) -> dict[str, tuple[int, int]]:
    """Each `## <FAMILY>-S<NN>` section, by id, as (start, end) offsets into the document."""
    marks = [
        (m.group(1), m.start())
        for m in re.finditer(rf"^## ({family.upper()}-S\d\d)\b", doc, re.M)
    ]
    ends = [m.start() for m in re.finditer(r"^## ", doc, re.M)] + [len(doc)]
    found = {}
    for name, start in marks:
        found[name] = (start, next(e for e in ends if e > start))
    return found


def bump_amount(text: str) -> str:
    """5,250.00 -> 5,251.00, and 0.00 -> 1.00: a figure an owner would act on, changed by one."""
    whole, _, frac = text.rpartition(".")
    value = int(whole.replace(",", "")) + 1
    return f"{value:,}.{frac}"


def bump_date(text: str) -> str:
    """10-20 -> 10-21, and 10-28 -> 10-01: a day every month has."""
    month, day = text.split("-")
    return f"{month}-{(int(day) % 28) + 1:02d}"


FIGURES = ((AMOUNT, bump_amount), (DATE, bump_date), (COUNT, lambda n: str(int(n) + 1)))


def figures(cell: str) -> list[tuple[int, int, str]]:
    """Every figure in a cell, left to right, as (start, end, replacement). A cell such as
    `Open invoices 12,000.00 against control 12,350.00; difference 350.00` yields three, so no
    figure hides behind the first one in its cell."""
    found: list[tuple[int, int, str]] = []
    for pattern, bump in FIGURES:
        for m in pattern.finditer(cell):
            if not any(s <= m.start() < e for s, e, _ in found):
                found.append((m.start(), m.end(), bump(m.group())))
    return sorted(found)


def mutations(doc: str, span: tuple[int, int]) -> list[tuple[str, str, str, int]]:
    """(label, before, after, occurrence) for every figure in one section's table rows: a cell
    repeated in several rows is mutated once per row, so no row hides behind another."""
    start, end = span
    out: list[tuple[str, str, str, int]] = []
    seen: dict[str, int] = {}
    for line in doc[start:end].splitlines():
        if not line.lstrip().startswith("|") or set(line) <= set("|-: "):
            continue
        for cell in (c.strip() for c in line.strip().strip("|").split("|")):
            changes = [
                after
                for s, e, r in figures(cell)
                if (after := cell[:s] + r + cell[e:]) != cell
            ]
            if not changes:
                continue
            occurrence = seen.get(cell, 0)
            seen[cell] = occurrence + 1
            nth = f" (row {occurrence + 1})" if occurrence else ""
            for after in changes:
                label = f"{cell!r} -> {after!r}{nth}"
                out.append((label, f"| {cell} |", f"| {after} |", occurrence))
    return out


def replace_nth(text: str, before: str, after: str, n: int) -> str:
    """`text` with the `n`th (0-based) occurrence of `before` replaced, or unchanged."""
    at = -1
    for _ in range(n + 1):
        at = text.find(before, at + 1)
        if at < 0:
            return text
    return text[:at] + after + text[at + len(before) :]


def run_test(tree: Path, family: str, scenario: str) -> bool:
    """Whether one Scenario's test passes in this worktree. `AR-S03` names `ar_s03_…`."""
    done = subprocess.run(
        ["cargo", "test", "--test", family, scenario.lower().replace("-", "_")],
        cwd=tree,
        capture_output=True,
        text=True,
    )
    # A filter that matches nothing also exits 0, so an empty run is not a pass.
    return done.returncode == 0 and " 0 passed" not in done.stdout


def main() -> int:
    # The documents use a typographic minus, which a cp1252 console cannot print.
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    family = sys.argv[1].lower()
    only = sys.argv[2].upper() if len(sys.argv) > 2 else None

    doc_path = Path("docs/scenarios") / f"{family}.md"
    doc = (REPO / doc_path).read_text(encoding="utf-8")
    spans = sections(doc, family)
    if only:
        spans = {only: spans[only]} if only in spans else {}
        if not spans:
            print(f"no section {only} in {doc_path}")
            return 2

    head = subprocess.run(
        ["git", "log", "-1", "--oneline"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    print(f"reviewing HEAD: {head}\n")

    tmp = Path(tempfile.mkdtemp(prefix="mutate-"))
    tree = tmp / "tree"
    subprocess.run(
        ["git", "worktree", "add", "--detach", str(tree), "HEAD"],
        cwd=REPO,
        check=True,
        capture_output=True,
    )
    target = tree / doc_path
    try:
        escapes: list[str] = []
        total = 0
        for name, span in sorted(spans.items()):
            if not run_test(tree, family, name):
                print(f"{name}: no test, or it already fails unmutated — skipped")
                continue
            applicable = [
                m
                for m in mutations(doc, span)
                if doc[span[0] : span[1]].count(m[1]) > m[3]
            ]
            if not applicable:
                print(f"{name}: no figure to change")
                continue
            for label, before, after, nth in applicable:
                total += 1
                mutated = doc[: span[0]] + replace_nth(doc[span[0] : span[1]], before, after, nth)
                target.write_text(mutated + doc[span[1] :], encoding="utf-8")
                if run_test(tree, family, name):
                    escapes.append(f"{name}: {label}")
                    print(f"  ESCAPED  {name}: {label}")
                else:
                    print(f"  caught   {name}: {label}")
            target.write_text(doc, encoding="utf-8")

        print(f"\n{total} mutations, {len(escapes)} escaped")
        for escape in escapes:
            print(f"  ESCAPED  {escape}")
        return 1 if escapes else 0
    finally:
        target.write_text(doc, encoding="utf-8")
        subprocess.run(
            ["git", "worktree", "remove", "--force", str(tree)],
            cwd=REPO,
            capture_output=True,
        )
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
