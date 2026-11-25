"""Every valid yaml-test-suite case must load via load_all and load_all_value."""

from pathlib import Path

import ozzyyaml

SUITE_ROOT = Path(__file__).resolve().parents[2] / "vendor" / "yaml-test-suite"


def valid_suite_ids(root: Path) -> list[str]:
    ids = []
    for d in root.iterdir():
        if not d.is_dir():
            continue
        if (
            (d / "in.yaml").is_file()
            and (d / "test.event").is_file()
            and not (d / "error").exists()
        ):
            ids.append(d.name)
    ids.sort()
    return ids


def test_all_valid_cases_load_all_and_load_all_value():
    assert (SUITE_ROOT / "229Q").is_dir(), (
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    )
    ids = valid_suite_ids(SUITE_ROOT)
    assert ids, "no valid suite cases found"

    failures: list[str] = []
    for case_id in ids:
        text = (SUITE_ROOT / case_id / "in.yaml").read_text(encoding="utf-8")
        try:
            ozzyyaml.load_all(text)
        except Exception as e:  # noqa: BLE001 — collect all suite failures
            failures.append(f"{case_id}: load_all: {e}")
            continue
        try:
            ozzyyaml.load_all_value(text)
        except Exception as e:  # noqa: BLE001
            failures.append(f"{case_id}: load_all_value: {e}")

    assert not failures, f"{len(failures)} failures:\n" + "\n".join(failures)
