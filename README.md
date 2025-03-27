# heliport
![License](https://img.shields.io/github/license/zjaume/heliport?color=blue)
![PyPi-version](https://img.shields.io/pypi/v/heliport)
![Python-version](https://img.shields.io/python/required-version-toml?tomlFilePath=https%3A%2F%2Fgithub.com%2FZJaume%2Fheliport%2Fraw%2Frefs%2Fheads%2Fmain%2Fpyproject.toml)
![Supported-languages](https://img.shields.io/badge/supported_languages-220-green)


A language identification tool which aims for both speed and accuracy, with support for [220 languages](LANGS.md).

This tool is an efficient [HeLI-OTS](https://aclanthology.org/2022.lrec-1.416/) port to Rust,
achieving 25x speedups while having almost identical output.

## Installation
### From PyPi
Install it in your environment
```
pip install heliport
```

NOTE: Since version 0.8 models do not need to be downloaded anymore.

### From source
Install the requirements:
 - Python
 - PIP
 - [Rust](https://rustup.rs)

Clone the repo, build the package and binarize the model
```
git clone https://github.com/ZJaume/heliport
cd heliport
pip install .
```

## Usage
### CLI
Just run the `heliport identify` command that reads lines from stdin
```
cat sentences.txt | heliport identify
```
```
eng
cat
rus
...
```

```
Identify languages of input text

Usage: heliport identify [OPTIONS] [INPUT_FILE] [OUTPUT_FILE]

Arguments:
  [INPUT_FILE]   Input file, default: stdin
  [OUTPUT_FILE]  Output file, default: stdout

Options:
  -j, --threads <THREADS>                Number of parallel threads to use.
                                         0 means no multi-threading
                                         1 means running the identification in a separated thread
                                         >1 run multithreading [default: 0]
  -b, --batch-size <BATCH_SIZE>          Number of text segments to pre-load for parallel processing [default:
                                         100000]
  -c, --ignore-confidence                Ignore confidence thresholds. Predictions under the thresholds will
                                         not be labeled as 'und'
  -s, --print-scores                     Print confidence score (higher is better) or raw score (lower is
                                         better) in case '-c' is provided
  -m, --model-dir <MODEL_DIR>            Model directory containing binarized model or plain text model.
                                         Default is Python module path or './LanguageModels' if relevant
                                         languages are requested
  -l, --relevant-langs <RELEVANT_LANGS>  Load only relevant languages. Specify a comma-separated list of
                                         language codes. Needs plain text model directory
  -h, --help                             Print help
```

### Python package
```python
>>> from heliport import Identifier
>>> i = Identifier()
>>> i.identify("L'aigua clara")
'cat'
```

For further information of the avaliable functions and parameters, please take a look at the module docs:
```python
>>> import heliport
>>> help(heliport)
```

### Rust crate
```rust
use std::path::PathBuf;
use heliport::identifier::Identifier;
use heliport::lang::Lang;

let identifier = Identifier::load(
    PathBuf::from("/path/to/model_dir",
    None,
    );
let lang, score = identifier.identify("L'aigua clara");
assert_eq!(lang, Lang::cat);
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
 - Rust and Java implementations have small precision differences due to Rust accumulating probabilities with double precision floats.

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

___

![Connecting Europe Facility](https://www.paracrawl.eu/images/logo_en_cef273x39.png)

All documents and software contained in this repository reflect only the authors' view. The Innovation and Networks Executive Agency of the European Union is not responsible for any use that may be made of the information it contains.
