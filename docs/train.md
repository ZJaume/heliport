## Adding support for more languages
HeLI is a generative classifier and each language has its own model, so adding support for more languages does not require training data for all the languages, just for the new ones.
To do so, follow this guide.

### Requirements
To add support for a new language you will need training data for that language.
Ideally a few hundred thousand sentences, but small or very small amounts data can sometimes work.
For example, some of the languages supported had only 5000 sentences for training.
A validation set is also needed, for the tool to compute confidence score.
1000 is usually enough for validation.

The format of the trainin data needs to be plain text and there's no additional preprocessing (which will be taken care by the tool).
However, it is advised to clean the data, removing sentences that may contaminate data with other languages.

When choosing the language or set of languages to add, it is recommended to look for similar languages that may already have support by the tool or may not be included in the set.
In both cases, all the languages that are similar should be taken into account and diagnose performance on each one.
As similar languages are the more probable ones to cause alterations in the tool accuracy.
A good methodology for adding languages can be found in [Jauhiainen et. al.](https://aclanthology.org/2024.sigul-1.15/).

### Step-by-step Guide
Clone the source code.
```
git clone https://github.com/ZJaume/heliport
cd heliport
```
Install [Rust](https://rustup.rs/).

Install Maturin for development in your Python environment.
```
pip install maturin
```

For speed optimizations, the language codes supported by heliport are embedded in the source code.
So, before training a new language, the code has to support them.
Edit the `Lang` enum in `heliport/heliport-model/src/lang.rs` and add the new language codes, keeping the alphabetical order.
Add the language code to the language list `heliport/LanguageModels/languagelist`, keeping the alphabetical order.

Build and install the tool to add the language codes support to the binary.
```
maturin develop -r
```

Create the language model for each of the new languages added:
```
heliport create-model LanguageModels/ my-train-files/fra.train my-train-files/eng.train
```
where `LanguageModels` is the output directory, and the rest are the train files, one file per language.
Each file has to follow the pattern `lang_code.train`.

After the language model has been created, the tool needs it to be binarized, to do so, you can build the package again
```
maturin develop -r
```
triggering the binarization of the model, or, since the source code has not changed, run the binarization command
```
heliport binarize -f -s
```

Finally, the confidence threshold can be calculated easily with this command:
```bash
cat fra.validation | heliport identify -s -n -p 16 | awk -F'\t' '$1=="fra"' | sort -t'\t' -nr | tail
```
will print the 10 lowest confidence values on correctly predicted sentences in the validation set.
Use the lowest value as a confidence threshold and write it in `LanguageModels/confidenceThresholds`.
The format of the file is language code + tab space + confidence value.
After that, run the binarization again.
But this time omit the `-s` option to check that all confidence values needed are present.
```
heliport binarize -f
```
In case you were adding more languages and they still have no confidence values computed, the command will fail. You can run it with `-s` option until you have all the new languages covered.
