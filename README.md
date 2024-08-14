# heliport
A language identification tool that aims to be both fast and accurate.
Originally started as a [HeLI-OTS](https://aclanthology.org/2022.lrec-1.416/) port to Rust.

## Installation
### From PyPi (not available yet)
Install it in your environment
```
pip install heliport
```
then download the model
```
heliport-download
```

### From source
Install the requirements:
 - Python
 - PIP
 - [Rust](https://rustup.rs)
 - [OpenSSL](https://docs.rs/openssl/latest/openssl/#automatic)

Clone the repo, build the package and compile the model
```
git clone https://github.com/ZJaume/heliport
cd heliport
pip install .
heliport-convert
```

## Usage
### CLI
Just run the `heliport` command that reads lines from stdin
```
cat sentences.txt | heliport
```
```
eng_latn
cat_latn
rus_cyrl
...
```

### Python package
```python
>>> from heliport import Identifier
>>> i = Identifier()
>>> i.identify("L'aigua clara")
'cat_latn'
```

### Rust crate
```rust
use std::sync::Arc;
use heliport::identifier::Identifier;
use heliport::lang::Lang;
use heliport::load_models;

let (charmodel, wordmodel) = load_models("/dir/to/models")
let identifier = Identifier::new(
    Arc::new(charmodel),
    Arc::new(wordmodel),
    );
let lang, score = identifier.identify("L'aigua clara");
assert_eq!(lang, Lang::cat_Latn);
```

## Benchmarks
Speed benchmarks with 100k random sentences from [OpenLID](https://github.com/laurieburchell/open-lid-dataset), all the tools running single-threaded:
| tool | time (s) |
| :--------- | ---------: |
| CLD2 | 1.12 |
| HeLI-OTS | 60.37 |
| lingua all high preloaded | 56.29 |
| lingua all low preloaded | 23.34
| fasttext openlid193 | 8.44 |
| heliport | 4.72 |
