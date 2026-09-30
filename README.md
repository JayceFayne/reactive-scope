# reactive-scope &emsp; [![Action Badge]][actions] [![Version Badge]][crates.io] [![License Badge]][license] [![Docs Badge]][docs]

[Version Badge]: https://img.shields.io/crates/v/reactive-scope.svg
[crates.io]: https://crates.io/crates/reactive-scope
[Action Badge]: https://github.com/JayceFayne/reactive-scope/workflows/Rust/badge.svg
[actions]: https://github.com/JayceFayne/reactive-scope/actions
[License Badge]: https://img.shields.io/crates/l/reactive-scope.svg
[license]: https://github.com/JayceFayne/reactive-scope/blob/master/LICENSE.md
[Docs Badge]: https://docs.rs/reactive-scope/badge.svg
[docs]: https://docs.rs/reactive-scope

This crate provides a fine-grained reactive scope. A reactive scope provide a clear lifecycle for signals and effects. When a scope is dropped, all signals and effects created within it are freed.

## Examples

For examples of `reactive-scope` in use, see the [examples](https://github.com/JayceFayne/ratatui-reactive/tree/master/examples) directory of [ratatui-reactive](https://github.com/JayceFayne/ratatui-reactive).

## Contributing

If you find any errors in reactive-scope or just want to add a new feature feel free to [submit a PR](https://github.com/jaycefayne/reactive-scope/pulls).
