# Development Notes

When running with `debug_assertions` enabled `rgr` will write its log file to `rgr.log` in the current working directory.

Thus, it's fairly straightforward to use a development flow with two terminals:

```bash
# Terminal 1
# This follows and displays `rgr`'s logs
tail -f ./rgr.log
```

```bash
# Terminal 2
# This builds and runs `rgr` in debug mode with logging enabled
RUST_LOG=trace cargo run -- <rg args>
```
