# ruby.install compile_flags Implementation Plan

> **Status: DONE**

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `compile_flags: Vec<String>` field to `ruby.install` that appends a `--` separator followed by each flag to the `ruby-install` invocation, forwarding flags verbatim to `./configure`.

**Architecture:** Single field addition to `RubyInstall` struct with `#[serde(default)]`. Plan logic appends `--` + flags after `--rubies-dir` when non-empty. All struct construction sites updated to include `compile_flags: vec![]`.

**Tech Stack:** Rust, serde, `lib/src/atoms/command::Exec`

---

## Files

- Modify: `lib/src/actions/ruby/install.rs` — add field, plan logic, tests; fix all construction sites

---

### Task 1: Add `compile_flags` field and plan logic

**Files:**

- Modify: `lib/src/actions/ruby/install.rs`

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn plan_includes_compile_flags_after_separator() {
    let action = RubyInstall {
        version: String::from("99.99.99"),
        implementation: None,
        rubies_dir: Some(String::from("/tmp/test-rubies")),
        version_manager: None,
        compile_flags: vec![String::from("--with-openssl-dir=/opt/homebrew/opt/openssl@3")],
    };
    let steps = action.plan();
    assert_eq!(steps.len(), 1);
    let args = steps[0].atom.to_string();
    assert!(args.contains("--"), "expected -- separator");
    assert!(args.contains("--with-openssl-dir=/opt/homebrew/opt/openssl@3"));
}
```

- [x] **Step 2: Run test — confirm FAIL**

- [x] **Step 3: Add field to struct**

```rust
#[serde(default)]
pub compile_flags: Vec<String>,
```

Add after `version_manager` field. Fix all struct construction sites by adding `compile_flags: vec![]`.

- [x] **Step 4: Add plan logic**

After the `rubies_dir` block in `plan()`:

```rust
if !self.compile_flags.is_empty() {
    arguments.push(String::from("--"));
    arguments.extend(self.compile_flags.iter().cloned());
}
```

- [x] **Step 5: Run tests — confirm PASS**

- [x] **Step 6: Add remaining tests**

Add 7 tests total covering: deserialization with flags, defaults to empty, flags after separator, flags with default rubies_dir, multiple flags, omits separator when empty, rbenv + compile_flags emits 3 steps.

- [x] **Step 7: Fix pre-existing failing test**

`actions::directory::create::tests::it_can_be_deserialized` asserted `"/some-directory"` but `pop()` returns the last entry from `examples/directory/create.yaml` (which has 2 entries). Fix: update assertion to `"{{ user.home_dir }}/.config/myapp"`.

- [x] **Step 8: Commit**

```bash
git add lib/src/actions/ruby/install.rs lib/src/actions/directory/create.rs examples/ruby/
git commit -m "feat(ruby.install): add compile_flags field"
```
