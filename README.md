# dtc
Datetime converter.

## Examples

```
dtc "2023-10-01 11:20:00 cest" utc
2023-10-01 09:20:00 UTC
```

When no date is provided, now is assumed:
```
dtc "15:27:32 jst" cest
2023-10-22 08:27:32 CEST
```

When no timezone is provided UTC is assumed:
```
dtc "2023-05-07 09:13:03" utc
2023-05-07 09:13:03 UTC
```
