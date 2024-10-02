# heliport
A language identification tool that aims to be both fast and accurate.
Originally started as a [HeLI-OTS](https://aclanthology.org/2022.lrec-1.416/) port to Rust.

## Installation
### From PyPi
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

## Differences with HeLI-OTS
Although `heliport` currently uses the same models as HeLI-OTS 2.0 and the 
identification algorithm is almost the same, there are a few differences
(mainly during pre-processing) that may cause different results.
However, in most case, these should not deacrease accuracy and should not happen frequently.

**Note**: Both tools have a pre-processing step for each identified text to
remove all non-alphabetic characters.

The implementation differences that can change results are:
 - `HeLI` during preprocessing removes urls and words beginning with `@`, while `heliport` does not.
 - Since 1.5, during preprocessing, HeLI repeats every word that does not start with capital letter, This is probably to penalize proper nouns. However, in our tests, we have not find a significant improvement with this. Therefore,to avoid multiplying the cost of prediction by almost x2, this has not been implemented. In the future it might end up being implemented if there is need for it and can be implemented efficiently.
 - Rust and Java sometimes have small differences on the smallest decimals in a float, so the stored n-gram probabilities are not exactly the same. But this is very unlikely to affect predicted labels.

## Benchmarks
Speed benchmarks with 100k random sentences from [OpenLID](https://github.com/laurieburchell/open-lid-dataset), all the tools running single-threaded:
| tool | time (s) |
| :--------- | ---------: |
| CLD2 | 1.12 |
| HeLI-OTS | 60.37 |
| lingua all high preloaded | 56.29 |
| lingua all low preloaded | 23.34
| fasttext openlid193 | 8.44 |
| heliport | 2.33 |
