"""Pin the root ruff.toml: ruff's default rules stay on, our extras are explicit.

`extend-select` (not `select`) keeps ruff's default rule set active, so a ruff
pin bump that adds defaults is gated by that PR's `make lint`.
"""

import unittest
from pathlib import Path

import tomllib

CONFIG = Path(__file__).resolve().parent.parent / "ruff.toml"

EXPECTED_EXTEND_SELECT = [
    "E4", "E7", "E9", "F", "W", "I", "B", "C4", "UP", "PIE", "RUF",
    "PLW1510", "BLE001", "S110", "S112", "C901",
]  # fmt: skip


class TestRuffConfig(unittest.TestCase):
    def setUp(self) -> None:
        with CONFIG.open("rb") as fh:
            self.config = tomllib.load(fh)
        self.lint = self.config["lint"]

    def test_select_is_absent_so_defaults_stay_on(self) -> None:
        self.assertNotIn("select", self.lint)

    def test_whole_config_is_pinned(self) -> None:
        # Any other key can switch defaults off (e.g. `extend-ignore = ["F"]`
        # disabled all 86 default F rules with every other test green), so the
        # full parsed file is compared; a new key is a deliberate edit here.
        self.assertEqual(
            self.config,
            {
                "target-version": "py310",
                "lint": {
                    "extend-select": EXPECTED_EXTEND_SELECT,
                    "ignore": ["RUF001", "RUF002", "RUF003"],
                    "unfixable": ["RUF100"],
                    "per-file-ignores": {"tests/**": ["E402"]},
                },
            },
        )

    def test_no_deprecated_top_level_rule_selection(self) -> None:
        # ruff 0.16 still honours a top-level `select`, which would replace the
        # defaults again (measured: 467 enabled rules down to 233).
        self.assertNotIn("select", self.config)
        self.assertNotIn("extend-select", self.config)

    def test_extend_select_is_pinned(self) -> None:
        self.assertEqual(self.lint["extend-select"], EXPECTED_EXTEND_SELECT)

    def test_ignore_unchanged(self) -> None:
        self.assertEqual(self.lint["ignore"], ["RUF001", "RUF002", "RUF003"])

    def test_unfixable_unchanged(self) -> None:
        self.assertEqual(self.lint["unfixable"], ["RUF100"])

    def test_per_file_ignores_unchanged(self) -> None:
        self.assertEqual(self.lint["per-file-ignores"], {"tests/**": ["E402"]})


if __name__ == "__main__":
    unittest.main()
