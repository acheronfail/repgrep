# Development Notes

All common commands should be found in the [`justfile`](./justfile).

You can run these with [`just`](https://github.com/casey/just).

## Simple local dev loop

When running with `debug_assertions` enabled `rgr` will write its log file to `rgr.log` in the current working directory.

Thus, it's fairly straightforward to use a development flow with two terminals:

```bash
# Terminal 1
# This follows and displays `rgr`'s logs
just dev-logs
```

```bash
# Terminal 2
# This builds and runs `rgr` in debug mode with logging enabled
just dev <rg args>
```

## Updating the README

The README in this repository is generated from the doc comments in `src/main.rs`.

Once the doc comments have been updated, run `just readme` to apply the changes to the README.
