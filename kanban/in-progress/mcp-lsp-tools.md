# Goal

Add the mpc-language-server as a tool in the cli

# References

For reference on how to implement a tool in this repo look at:
* volition-cli/src/tools/mod.rs
* volition-cli/src/tools/providers.rs
* volition-cli/src/tools/file.rs
* volition-core/src/mod.rs
* volition-core/src/fs.rs

I've also downloaded the README.md file from the mcp-language-server github repo for reference.

# Definition of done

* the CLI exposes all mcp-language-server tools to the LLMs
* there are no build errors or warnings
* all tests pass