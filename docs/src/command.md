# Commands

- command.run

## command.run

This is the most basic, but yet powerful action as it can be used to run anything needed that is not directly supported in etch by an action. An alias exists, allowing you to abbreviate `command` as `cmd`.

| Key        | Type                    | Optional | Description                                     |
|:-----------|:------------------------|:---------|:------------------------------------------------|
| action     | string                  | no       | command.run                                     |
| command    | string                  | no       | command to run                                  |
| args       | string                  | yes      | argument passed                                 |
| dir        | string                  | yes      | actual working directory                        |
| privileged | bool                    | yes      | elevate privileges when executing               |
| env        | HashMap<string, string> | yes      | key and values for scoped environment variables |

### Scoped environment variables

`env:` sets environment variables for a single `command.run` action. It is a hash map of keys and values; multiple variables are supported. Three facts apply:

- The variables are passed to the command itself, including privileged commands: under `privileged: true` they reach the command through an `env K=V` prefix after the privilege provider (`sudo`, `doas`, `run0`).
- An unprivileged program is resolved on the step's own `PATH` from `env:`, so `env: {PATH: ...}` can locate a program that is not on etch's `PATH`.
- `env:` does not modify etch's own process environment. Later actions never see these variables, even when the command fails.

### Example

```yaml
- action: command.run
  dir: .
  command: echo
  args:
  - Hello world

# we should see the GOBIN set in the go env output now
- action: command.run
  command: go
  args:
    - env
  env:
    GOBIN: /Users/test

# GOBIN from `env:` applied only to the previous command; this `go env` sees etch's own environment
- action: command.run
  command: go
  args:
    - env
```
