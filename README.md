# Subword Navigation for Zed

Subword Navigation adds cursor movement, selection, and deletion for camelCase,
snake_case, digit boundaries, and separator-delimited text. It is a Zed port of
the VS Code extension
[Subword Navigation](https://github.com/ow--/vscode-subword-navigation).

## Commands

- `subwordNavigation.cursorSubwordLeft`
- `subwordNavigation.cursorSubwordRight`
- `subwordNavigation.cursorSubwordLeftSelect`
- `subwordNavigation.cursorSubwordRightSelect`
- `subwordNavigation.deleteSubwordLeft`
- `subwordNavigation.deleteSubwordRight`

## Default Key Bindings

The extension declares default key bindings for editors and git diffs:

| Command | Linux/Windows | macOS |
| --- | --- | --- |
| Move right | `alt-right` | `ctrl-right` |
| Select right | `alt-shift-right` | `ctrl-shift-right` |
| Move left | `alt-left` | `ctrl-left` |
| Select left | `alt-shift-left` | `ctrl-shift-left` |
| Delete left | `alt-backspace` | `ctrl-backspace` |
| Delete right | `alt-delete` | `ctrl-delete` |

## Development

This extension currently depends on unreleased Zed editor-command extension
APIs. Until those APIs are published in `zed_extension_api`, build it from a
checkout where `zed-subword-navigation` and `zed` are sibling directories:

```sh
cargo test
cargo clippy --all-targets
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/zed_subword_navigation.wasm extension.wasm
```

When `zed_extension_api = "0.8.0"` is available on crates.io, replace the local
path dependency in `Cargo.toml` with the published crate version.

## License

MIT. This port is based on the MIT-licensed VS Code Subword Navigation extension
by Olle Westman.
