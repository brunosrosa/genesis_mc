# This crate is a fork of saphyr-parser intended to work with serde-saphyr.

This crate is a fork of [saphyr-parser](https://crates.io/crates/saphyr-parser), for work with [serde-saphyr](https://crates.io/crates/serde-saphyr).

It includes a small set of targeted changes required for full YAML compliance, resolving all yaml-test-suite failures. These changes have been proposed upstream. If they are incorporated, this fork may be deprecated in the future.

# Changes made

`saphyr-parser-bw` has the same public API as `saphyr-parser`.  
It differs only in the following, narrowly scoped behaviors, all motivated by YAML compliance and interoperability.

## Test case 4H7K: extra closing bracket is an error
A sequence such as:

```yaml
[ a, b, c ] ]
```

is invalid YAML. This case is now correctly reported as an error.

## Test case BS4K: comment intercepts multiline content
A comment that intercepts multiline content is invalid YAML:

```
word1  # comment
word2
```

while this is valid, even if can only occur at the top level and not in the map:
```yaml
word1
word2
```

`saphyr-parser` version `0.0.6` accepted input with comment as valid and silently discarded the part of the text following the comment. This behavior has been corrected.

## Test case ZYU8: reserved directives must be ignored
Reserved directives must be ignored when they appear in a document.  
While `saphyr-parser` does not make use of such directives, version `0.0.6` raised an error instead of ignoring them.  
This has been fixed to match the YAML specification.

## Insufficiently indented closing bracket accepted as valid

This is the most controversial change, and we fully understand the argument that such documents should be rejected:

```yaml
key: [ 1, 2, 3,
       4, 5, 6
] # <-- this closing bracket is not sufficiently indented
```

However, we received multiple bug reports and user complaints about rejecting this input, likely because many other YAML parsers accept it.  
After careful consideration, the `serde-saphyr` team decided to support this case for compatibility reasons.

That said, we still strongly recommend placing the closing bracket further to the right to remain fully YAML-compliant.

## Fix unexpected end of plain scalar while parsing string, cherry picked #87 from saphyr-rs
The pull request [#87](https://github.com/saphyr-rs/saphyr/pull/87) fixes parsing of the followign YAML:

```yaml
hello:
  world: this is a string
    --- still a string
```
The expected string value is `this is a string --- still a string`.

## Report the root cause of mismatching brackets
When bracket (`[`,`{`) or quote lacks the matching closing bracket, unpatched `0.0.6` version currently reports then end of file as the location of the error. This may be very far from the actual cause of the error, and reporting the opening bracket is more helpful. Our version in such cases reports row and column of the opening bracket as the error location.

# saphyr-parser

[saphyr-parser](https://github.com/saphyr-rs/saphyr-parser) is a fully compliant YAML 1.2
parser implementation written in pure Rust.

**If you want to load to a YAML Rust structure or manipulate YAML objects, use
`saphyr` instead of `saphyr-parser`. This crate contains only the parser.**

This work is based on [`yaml-rust`](https://github.com/chyh1990/yaml-rust) with
fixes towards being compliant to the [YAML test
suite](https://github.com/yaml/yaml-test-suite/). `yaml-rust`'s parser is
heavily influenced by `libyaml` and `yaml-cpp`.

`saphyr-parser` is a pure Rust YAML 1.2 implementation that benefits from the
memory safety and other benefits from the Rust language.

## Security

This library does not try to interpret any type specifiers in a YAML document,
so there is no risk of, say, instantiating a socket with fields and
communicating with the outside world just by parsing a YAML document.

## Specification Compliance

This implementation is fully compatible with the YAML 1.2 specification. In
order to help with compliance, `yaml-rust2` tests against (and passes) the [YAML
test suite](https://github.com/yaml/yaml-test-suite/).

## License

Licensed under either of

 * Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license (http://opensource.org/licenses/MIT)

at your option.

Since this repository was originally maintained by
[chyh1990](https://github.com/chyh1990), there are 2 sets of licenses.
A license of each set must be included in redistributions. See the
[LICENSE](LICENSE) file for more details.

You can find licences in the [`.licenses`](.licenses) subfolder.

## Contribution

[Fork this repository](https://github.com/saphyr-rs/saphyr-parser/fork) and
[Create a Pull Request on Github](https://github.com/saphyr-rs/saphyr-parser/compare/master...saphyr-rs:saphyr-parser:master).
You may need to click on "compare across forks" and select your fork's branch.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

## Links

* [saphyr-parser source code repository](https://github.com/saphyr-rs/saphyr-parser)

* [saphyr-parser releases on crates.io](https://crates.io/crates/saphyr-parser)

* [saphyr-parser documentation on docs.rs](https://docs.rs/saphyr-parser/latest/saphyr-parser/)

* [yaml-test-suite](https://github.com/yaml/yaml-test-suite)
