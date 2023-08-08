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

## Submitting a Pull Request

Some guidelines:

* All development is done on the `next` branch, so please target that for any Pull Request.
* Make sure to run `just fmt` on each commit so formatting is consistent
* Make sure to also use `just test` to ensure each commit passes the tests

## Making a release

Mostly so I don't forget if I come back to this project after a while.

1. use `just bump` to bump the version
2. create PR with version bump and merge
3. if merge commit has a successful CI build, then `git tag <version>` on `master`
4. CI will create a draft release and start uploading artifacts to it
5. edit it, and publish it as a release
