# TODO: Error Handling Improvements

## Issue: Shell Command Non-Zero Exit Status

The `run_shell_command` function in `src/tools/shell.rs` currently captures the exit status of the executed command but does not treat a non-zero status as an error. It returns an `Ok(String)` result containing the status code, stdout, and stderr, regardless of whether the command succeeded or failed.

This can mask command failures from the calling process (the LLM), potentially leading to incorrect assumptions or actions.

## Recommendation

Modify `src/tools/shell.rs`:

*   Check the `output.status.code()` after executing the command.
*   If the status code is non-zero, return an `Err(anyhow!(...))` result instead of `Ok(...)`.
*   The error message should include the exit status, stdout, and stderr to provide context for the failure.
*   Consider if there are specific non-zero codes that *shouldn't* be treated as errors, although generally, non-zero indicates failure.
