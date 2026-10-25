# Fixtures

Recorded flights used as test data. Each file is JSON lines of the
messages a connector produced, written by `fd-recorder record`.

Name files `<sim>-<from>-<to>-<what>.jsonl`, for example
`msfs2024-ksea-kpdx-normal.jsonl` or `msfs2020-egll-egll-touch-and-go.jsonl`,
and replay them with `fd-recorder replay <file>`.

Large recordings can be compressed with zstd before committing.
