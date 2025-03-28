# TODO: Logging Improvements

## Issues: Current Logging Configuration

The current logging setup using `tracing` and `FmtSubscriber` works but has several areas for improvement for production readiness:

1.  **Format:** The default human-readable text format is not ideal for log aggregation and analysis systems.
2.  **Default Level:** The default level is `WARN`, which might hide important `INFO`-level events in production.
3.  **Configuration:** Configuration is primarily via command-line flags (`--verbose`, `--debug`). Environment variable configuration is more standard for deployed applications.
4.  **Output Destination:** Logs go to stdout/stderr. While suitable for container orchestration, direct file logging might be needed in other scenarios.
5.  **Inconsistent Logging:** A `println!` call exists in `src/tools/shell.rs` for operational output, which should use the `tracing` framework.

## Recommendations

1.  **Structured Logging:**
    *   Modify `src/main.rs` to configure `tracing_subscriber` to output logs in JSON format (e.g., using `.json()` layer or `tracing-bunyan-formatter`).
2.  **Default Level:**
    *   Change the default log level in `src/main.rs` to `INFO` instead of `WARN`.
3.  **Environment Variable Configuration:**
    *   Integrate `tracing_subscriber::EnvFilter` to allow log level configuration via the `RUST_LOG` environment variable (e.g., `RUST_LOG=info`, `RUST_LOG=volition=debug,warn`). This can work alongside the command-line flags.
4.  **Output Management:**
    *   Ensure the deployment strategy correctly handles stdout/stderr logs (e.g., container logging drivers). If file logging is required, configure `tracing_appender` or similar.
5.  **Refactor `println!`:**
    *   In `src/tools/shell.rs`, replace the `println!("Running: ...")` call with `tracing::info!("Running command: {}", command)` or `tracing::debug!(...)` as appropriate.
