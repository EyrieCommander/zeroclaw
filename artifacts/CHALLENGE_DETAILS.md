# Committed agent-rename recovery

Upstream report: [zeroclaw-labs/zeroclaw#10373](https://github.com/zeroclaw-labs/zeroclaw/issues/10373).

Original issue pin: `c1e79a774b8d0a8539481b08838a3c818c97a6d0`.

Branch base: `fb116d612` (`origin/master`), one commit after that pin. Both patches still apply on this base.

## Contract

One runtime-owned recovery record is shared by the CLI, the gateway map-key handlers, and RPC. It is armed when a rename commits (and when a committed rename is discovered with residue or an unreadable follower). It is cleared only after workspace, memory, cron, ACP, and session followers have been read and converged. While it is open, the retired alias cannot be created again, including after residue is deleted out of band.

Knowledge-graph ownership from unmerged work is out of scope. Followers are the ones on this pin: default per-alias workspace, memory, cron, ACP sessions, and session attribution.

## Why a naive patch fails

Copying the existing gateway residue scan into the CLI is not enough:

- That scan fails open. An unreadable cron database is treated as "no residue", so the rename is reported as not configured and recovery is not armed. The tests require a retryable failure and a refused recreate of the old alias.
- Residue alone cannot tell a stranded rename from an alias that was recreated on purpose. After an out-of-band wipe, a residue scan allows `agents create` of the retired alias. The tests require that create to keep failing until a later rename converges and clears recovery.
- A gateway-only journal is invisible to the CLI, and a CLI-only journal is invisible to the gateway. The cross-surface test arms recovery through the gateway handler and then requires the CLI to refuse the old alias and later finish the same recovery.
- Clearing recovery on the first follower error, or moving a custom workspace path, fails the blocked-workspace and custom-workspace cases.
- HTTP and RPC still return a committed rename when a follower lags, matching the existing gateway tests. The CLI must exit non-zero for that same lag, without the "is not configured" wording.

The hidden tests call the `zeroclaw` binary, the public gateway handlers, and the public RPC dispatcher. They do not import the recovery module, so a solution that only adds an unwired helper still fails.

## Patches

`solution.patch` changes 7 production files:

- `crates/zeroclaw-runtime/src/agent_rename_recovery.rs`
- `crates/zeroclaw-runtime/src/lib.rs`
- `crates/zeroclaw-runtime/src/rpc/dispatch.rs`
- `crates/zeroclaw-runtime/locales/en/cli.ftl`
- `crates/zeroclaw-gateway/src/api_config.rs`
- `crates/zeroclaw-gateway/src/agent_owned_state.rs`
- `src/alias_cli/mod.rs`

`test.patch` changes 5 files and does not overlap those paths:

- `Cargo.toml`
- `test.sh`
- `tests/committed_agent_rename.rs`
- `crates/zeroclaw-gateway/src/lib.rs`
- `crates/zeroclaw-gateway/src/committed_rename_recovery_tests.rs`

## Harness

`test.sh --output_path FILE {base|new}` runs, without `set -e`:

- `cargo test --offline --test committed_agent_rename -- --test-threads=1`
- `cargo test --offline -p zeroclaw-gateway --lib committed_rename -- --test-threads=1`

It writes JUnit from the cargo `test ... ok|FAILED` lines. Exit status is non-zero when any cargo invocation fails. `base` (tests only) is expected to be non-zero. `new` (tests plus solution) is expected to be zero.

Build the image with the repository root as context after `test.patch` is applied and before `solution.patch`:

```sh
docker build -f artifacts/Dockerfile .
```

The image is `rust:1.96-bookworm`, `WORKDIR /app`, fetches the locked crates, compiles the harness tests with `--offline --no-run`, and ends with `CMD ["bash"]`.
