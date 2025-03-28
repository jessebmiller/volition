- The multi-turn conversation/state recovery feature (f06828b) is
  interesting. Where is this state persisted? What's the strategy for
  handling potential errors during state loading/saving (e.g.,
  corrupted state file)?

- For the configurable iteration limit (7ce8a04), where is this
  configuration value expected to be stored and read from? How does
  the user prompt mechanism work if it's missing?

- The verbose tracing (bfd9277) is great for debugging.  Is this
  controlled via the standard RUST_LOG environment variable? Are there
  any concerns about potentially sensitive information appearing in
  trace logs?

- The decoupling of config loading (04b32a5, b995b61) is good. Does
  the core library now define traits or structs that the consuming
  application needs to implement or provide for configuration?
  - is it documented?