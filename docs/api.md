# API Contract

Default base URL:

```text
https://nomad-timer.hyeon.space/api
```

## GET /schedule

Returns the server-clock rhythm anchor.

```json
{
  "serverTimeMs": 1780655673927,
  "cycleStartMs": 1780653600000,
  "workMinutes": 50,
  "breakMinutes": 10
}
```

## GET /messages

Returns recent preset broadcasts, newest first.

```json
{
  "messages": [
    {
      "presetId": "water",
      "label": "물 마시기",
      "message": "물 한 잔 마시고 돌아와요.",
      "sender": "windows-widget",
      "sentAtMs": 1780655673927
    }
  ]
}
```

## POST /broadcast

Only prepared preset IDs are accepted.
Production should rate-limit this endpoint because it intentionally accepts unauthenticated preset reactions.

```json
{
  "presetId": "water",
  "sender": "windows-widget"
}
```

Accepted preset IDs are defined in `src/presets.rs`.
