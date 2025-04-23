# Changelog

## [0.2.0](https://github.com/jessebmiller/volition/compare/volition-core-v0.1.4...volition-core-v0.2.0) - 2025-04-23

### Added

- add recursive option to list_directory_contents
- enhance ApiResponse with token usage and content fields

### Fixed

- improve Gemini provider implementation
- resolve lifetime issues in list_directory_contents
- use relative paths in list_directory_contents output
- make list_directory_contents recursive
- make DEFAULT_ENDPOINT public for testing
- corrects tool format for openai API

### Other

- update provider implementation documentation to reflect direct integration approach
- integrate API functionality directly into provider implementations
- remove api module as functionality is now integrated into providers
- remove api module
- remove API module implementation files
- clean up api module
- simplify provider architecture by removing ChatApiProvider trait
- update OpenAI provider to use new ChatApiProvider interface
- update Ollama provider to use new ChatApiProvider interface
- update mock provider to match new ApiResponse structure
- improve schema to tool parameters conversion
- improve filesystem tool implementation with better error handling and FileInfo struct
- improve Ollama provider implementation
- improve OpenAI provider implementation and error handling
- simplify ChatApiProvider trait and improve error handling
- update Gemini model configuration documentation
- *(gemini)* improve endpoint construction with model name
- refactors api providers code for simpler provider creation

## [0.1.4](https://github.com/jessebmiller/volition/compare/volition-core-v0.1.3...volition-core-v0.1.4) - 2025-04-08

### Added

- adds the OpenAI provider

## [0.1.3](https://github.com/jessebmiller/volition/compare/volition-core-v0.1.2...volition-core-v0.1.3) - 2025-04-04

### Fixed

- correct the google api request

## [0.1.2](https://github.com/jessebmiller/volition/compare/volition-core-v0.1.1...volition-core-v0.1.2) - 2025-04-04

### Other

- no explicit \n in summaries

## [0.1.1](https://github.com/jessebmiller/volition/compare/volition-core-v0.1.0...volition-core-v0.1.1) - 2025-04-04

### Fixed

- *(ui)* nicer output spacing

### Other

- *(ui)* more nice spacing

## 0.1.0 (2025-04-03)


### Features

* user facing feedback ([4d5e175](https://github.com/jessebmiller/volition/commit/4d5e175e6d709eb0cea26504dee1d3dadb2dbeb0))
