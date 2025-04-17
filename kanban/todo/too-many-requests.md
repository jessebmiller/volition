when we get API error: 429 Too Many Requests we shouldn't error log the whole thing

we should also respect the retry delay