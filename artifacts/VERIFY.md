# Harness verification

Original issue pin: `c1e79a774b8d0a8539481b08838a3c818c97a6d0`.

Resynced onto `origin/master` at `fb116d612` (`feat(log): add entry-count rotation and multi-segment log queries (#10214)`). That commit edits `crates/zeroclaw-runtime/src/rpc/dispatch.rs` only around the log-query handlers, not the rename handlers. On this base:

```sh
git apply --check artifacts/test.patch
git apply --check artifacts/solution.patch
```

Both succeeded. The fail-to-pass runs below were executed on the original pin before this rebase.

Path intersection of `solution.patch` and `test.patch` is empty.

Solution files (7):

- `crates/zeroclaw-runtime/src/agent_rename_recovery.rs`
- `crates/zeroclaw-runtime/src/lib.rs`
- `crates/zeroclaw-runtime/src/rpc/dispatch.rs`
- `crates/zeroclaw-runtime/locales/en/cli.ftl`
- `crates/zeroclaw-gateway/src/api_config.rs`
- `crates/zeroclaw-gateway/src/agent_owned_state.rs`
- `src/alias_cli/mod.rs`

Test files (5):

- `Cargo.toml`
- `test.sh`
- `tests/committed_agent_rename.rs`
- `crates/zeroclaw-gateway/src/lib.rs`
- `crates/zeroclaw-gateway/src/committed_rename_recovery_tests.rs`

## Commands

Tests only (solution reverted, `test.patch` contents present):

```sh
./test.sh --output_path /tmp/junit-base.xml base
```

Exit code: `1`. JUnit: `tests="12" failures="12"`. Every CLI, gateway, and RPC case failed its assertion. Representative base results:

- CLI resume of a committed rename reported `invalid new alias: alias agent_b already exists` instead of converging followers.
- CLI create after an unreadable cron store printed `created agents.agent_a`.
- Gateway create after an unreadable cron store returned `{"created":true,...}`.
- An unrelated `agent_a -> agent_b` with no residue returned HTTP 400 `alias agent_b already exists` rather than 404 `is not configured`.

Tests plus solution (`git apply artifacts/solution.patch`):

```sh
./test.sh --output_path /tmp/junit-new.xml new
```

Exit code: `0`. JUnit: `tests="12" failures="0"`.

`base` is non-zero. `new` is zero.

## Other checks

```sh
cargo fmt -- --check <changed Rust files>
cargo clippy -p zeroclaw-runtime --lib -- -D warnings
cargo clippy -p zeroclaw-gateway --lib --tests --no-deps -- -D warnings
cargo clippy --bin zeroclaw --test committed_agent_rename --no-deps -- -D warnings
bash -n test.sh
```

All exited 0. A workspace-wide `cargo clippy -p zeroclaw-gateway --lib --tests -- -D warnings` stopped in pre-existing `zeroclaw-channels` `clippy::drop_non_drop` findings and did not report diagnostics in the recovery files.

`docker build` was not run. The image recipe is `artifacts/Dockerfile` (`rust:1.96-bookworm`, `WORKDIR /app`, `cargo fetch --locked`, offline `--no-run` for `--test committed_agent_rename` and `-p zeroclaw-gateway --lib`, `CMD ["bash"]`). The fail-to-pass commands above are the ones that image is meant to run.
