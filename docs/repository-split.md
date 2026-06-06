# Repository Split

Nomad Timer should be split into two repositories when the API grows beyond the current MVP.

## nomad-timer

Public client repository.

- Windows app
- landing site
- pixel cat assets
- API contract structs
- client tests

The app defaults to:

```text
https://nomad-timer.hyeon.space/api
```

## nomad-timer-api

API server repository.

- schedule endpoint
- preset broadcast endpoint
- recent messages storage
- rate limiting
- moderation
- deployment config

This repo should own production deployment. The client repo should only know the public API contract and base URL.
