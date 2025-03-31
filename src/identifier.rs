use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use ordered_float::OrderedFloat;
use strum::{IntoEnumIterator, EnumCount};
use shingles::AsShingles;
use anyhow::Result;
use log::{debug,warn};
use rayon::prelude::*;

#[cfg(feature = "python")]
use pyo3::pyclass;

use heliport_model::Model;
use heliport_model::{Lang, LangScores, LangBitmap};
use crate::utils::{is_cjk_block, RE_NON_ALPHA};


#[cfg_attr(feature = "python", pyclass)]
pub struct Identifier {
    model: Arc<Model>,
    lang_scored: LangBitmap,
    lang_points: LangScores,
    word_scores: LangScores,
    heli_score: BTreeMap<OrderedFloat<f32>, Vec<Lang>>,
    pub ignore_confidence: bool,
}

/// A clone of Identifier creates new instances for all the members
/// except the model, which is a pointer to avoid copying it.
impl Clone for Identifier {
    fn clone(&self) -> Self {
        Self::new(
            self.model.clone(),
            self.ignore_confidence,
        )
    }
}

impl Identifier {
    const PENALTY_VALUE : f32 = 7.0;
    const MAX_NGRAM : usize = 6;

    pub fn load(modelpath: &Path, langs: Option<Vec<Lang>>) -> Result<Self> {
        Ok(Self::new(
                Arc::new(Model::load(modelpath, false, langs)?),
                false,
            ))
    }

    pub fn new(model: Arc<Model>, ignore_confidence: bool) -> Self {
        Self {
            model: model,
            lang_scored: LangBitmap::new(),
            lang_points: LangScores::new(),
            word_scores: LangScores::new(),
            heli_score: BTreeMap::new(),
            ignore_confidence: ignore_confidence,
        }
    }
    /// Disable use of confidence thresholds
    pub fn disable_confidence(&mut self) {
        self.ignore_confidence = true;
    }

    /// Disable use of confidence thresholds
    pub fn without_confidence(&mut self) -> &mut Self {
        self.ignore_confidence = true;
        self
    }

    /// Enable use of confidence thresholds
    pub fn with_confidence(&mut self) -> &mut Self {
        self.ignore_confidence = false;
        self
    }

    /// Get the most probable language according to the current language scores
    fn pick_winner(&mut self) -> (Lang, f32) {
        // if only one lang is requested, just search for the minimum score (winner)
        let mut score = Self::PENALTY_VALUE + 1.0;
        let mut winner_lang = Lang::und;

        // Get the lang with minimum score
        for lang in Lang::iter() {
            let points = self.lang_points.get(lang);
            if points <= score {
                score = points;
                winner_lang = lang;
            }
        }
        winner_lang = winner_lang.collapse();
        debug!("Winner lang '{winner_lang}' with score '{score}'");

        // Compute confidence value
        // confidence is absolute difference with the second scoring language
        // only collapsed macrolangs can be taken into account
        if !self.ignore_confidence {
            let mut second = Self::PENALTY_VALUE + 1.0;
            for lang in Lang::iter() {
                let points = self.lang_points.get(lang);
                // compare only collapsed macrolangs
                if lang.collapse() != winner_lang && points <= second {
                    second = points;
                }
            }
            // Compute absolute difference
            score = second - score;
            // Get the threshold, thresholds are only for macrolangs, so collapse
            let threshold = self.model.confidence.get(winner_lang.collapse());
            if threshold > score {
                winner_lang = Lang::und;
            }
            debug!("Winner lang '{winner_lang}' with confidence '{score}' and threshold '{threshold}'");
        }

        (winner_lang, score)
    }

    /// Build a ranking of the top k scoring languages,
    /// according to the current language scores
    fn rank_langs(&mut self, k: usize) -> Vec<(Lang, f32)> {
        //TODO collapse macro languages
        self.heli_score.clear();
        let mut winners = Vec::with_capacity(k);
        for lang in Lang::iter() {
            let ord_score = OrderedFloat(self.lang_points.get(lang));
            if let Some(langs) = self.heli_score.get_mut(&ord_score) {
                langs.push(lang);
            } else {
                self.heli_score.insert(
                    ord_score,
                    Vec::from([lang])
                );
            }
        }
        // Extract the topk from the tree
        'outer: for _ in 0..k {
            if let Some((score, langs)) = self.heli_score.pop_first() {
                for lang in langs {
                    winners.push((lang, score.into_inner()));
                    // There can be ties, indeed all langs that haven't been scored will be 7.0
                    // and a heli_score.pop will return more than one
                    // so we stop filling the array if k elements have been added
                    if winners.len() >= k {
                        break 'outer;
                    }
                }
            }
        }
        winners
    }

    /// Update scores according to current ngram probability if found
    fn score_gram(&mut self, gram: &str, dic_id: usize) -> bool {
        if let Some(kiepro) = self.model[dic_id].dic.get(gram) {
            // found the word in language model
            // update scores according to each lang that has the word
            // use penalty value for langs that don't have the word
            debug!("word scored '{gram}'");
            debug!("{:?}", kiepro);
            self.lang_scored.reset();
            let mut score;
            // Score the langs that have probabilities for this ngram
            for (lang, prob) in kiepro {
                score = self.word_scores.get(*lang);
                self.word_scores.insert(lang.clone(), score + *prob);
                self.lang_scored.set(lang, true);
            }
            // Penalize all the languages that do not have probabilities for this ngram
            for i in 0..Lang::COUNT {
                // instead of excluding scored langs with an if
                // sum them all, multiplying by the negation of the bitmap
                // which results in adding a 0 if it's scored
                // this is faster, because of easier autovectorization?
                self.word_scores.add_index(
                    i,
                    Self::PENALTY_VALUE * !self.lang_scored[i] as usize as f32
                );
            }
            return true;
        }
        false
    }

    /// Read the text and obtain language scores based on found ngrams.
    fn score_langs(&mut self, text: &str) -> bool {
        // lowercase and remove non-alphabetic characters
        //TODO is it really remove all non alpha? because I found words with punctuation in
        //langmodel entries
        debug!("Input text: '{}'", text);
        let lowercased = text.to_lowercase();
        let replaced = RE_NON_ALPHA.replace_all(&lowercased, " ");
        self.heli_score.clear();

        let mut last_was_cjk = false;
        let mut last_was_space = false;
        let mut cjk_num_chars = 0_usize;
        let mut mystery_text = String::with_capacity(replaced.len());
        let mut mystery_length = 0;

        for mystery_char in replaced.chars() {
            // Original HeLI checks only CJK_*, which is only the common background
            // chars of CJK and does not include Hana or Hangul.
            // I do not know if this was intentional, but as a side effect, it separates
            // the groups of CJK unfied (commonly known chinese chars) from the hana and hangul
            // with a space. It seems to give better japanese identification in some cases.
            let is_cjk = if let Ok(is) = is_cjk_block(mystery_char) {
                is
            } else {
                warn!("Could not find unicode block for '{}'", mystery_char);
                return false;
            };

            if is_cjk {
                if !last_was_cjk && !last_was_space {
                    mystery_text.push(' ');
                }
                last_was_cjk = true;
                last_was_space = false;
                cjk_num_chars += 1;
            } else {
                if last_was_cjk && mystery_char != ' ' {
                    mystery_text.push(' ');
                }
                last_was_space = mystery_char == ' ';
                last_was_cjk = false;
            }
            if !last_was_space {
                mystery_length += 1;
            }
            mystery_text.push(mystery_char);
        }

        debug!("Mystery text: '{}'", mystery_text);
        //debug!("Words: [{:?}]", mystery_text.split_whitespace().format(", "));

        // We don't need to remove repeated spaces
        // split_whitespace ignores them
        let mut words = mystery_text.split_whitespace().peekable();


        if words.peek().is_none() {
            return false;
        }

        self.lang_points.reset();

        let mut word_scored;
        let mut num_words = 0;
        for word in words {
            debug!("Scoring '{}'", word);
            num_words += 1;
            self.word_scores.reset();
            word_scored = self.score_gram(word, 0);

            // Go from highest order ngram to lowest until one of the orders is found in any
            // language
            //TODO does it make sense to explore ngrams longer than the current word?
            if !word_scored {
                debug!("Word has not been found");
                let wordspace = format!(" {word} ");
                for t in (1..Self::MAX_NGRAM+1).rev() {
                    if word_scored {
                        break;
                    }

                    let mut grammaara = 0;
                    // Iterate over all possible ngrams of order t, over the current word
                    for gram in wordspace.as_shingles(t) {
                        let cur_scored = self.score_gram(gram, t);
                        grammaara += cur_scored as usize; // sum+1 if score returns true
                        if !word_scored && cur_scored {
                            word_scored = true;
                        }
                    }

                    if word_scored {
                        // Normalize wordscores by the number of ngrams found in ngram models
                        debug!("Word scores: {:?}", self.word_scores);
                        self.word_scores.norm(grammaara as f32);
                    }
                }
            }

            // accumulate wordscores for the current word in the global lang points
            self.lang_points.add(&self.word_scores);
            debug!("Word scores: {:?}", self.word_scores);
            debug!("Lang points: {:?}", self.lang_points);
        }

        debug!("Finished scoring");
        // Choose the winner
        // the original code adds "und" but seems to not take it into consideration
        //self.lang_points.insert("und".to_string(), Self::PENALTY_VALUE + 1.0);
        debug!("Lang points: {:?}", self.lang_points);

        // Normalize lang points and apply penalties if more than 50% is CJK
        //TODO try to simplify this
        // the CJK fix could just finish early?
        let cjk_pct;
        if mystery_length == 0 {
            cjk_pct = 0.0;
        } else {
            cjk_pct =  cjk_num_chars as f32 / mystery_length as f32;
        }
        debug!("CJK amount: {cjk_num_chars} ({cjk_pct:.2}%) mystery_text size: {mystery_length}");
        for lang in Lang::iter() {
            let lang_score_norm = self.lang_points.get(lang) / num_words as f32;
            self.lang_points.insert(lang, lang_score_norm);

            if cjk_pct > 0.5 && !lang.is_cjk() {
                self.lang_points.insert(lang, Self::PENALTY_VALUE + 1.0);
            }
        }
        debug!("Normalized lang points: {:?}", self.lang_points);

        true
    }

    /// Identify the most probable language of a given text.
    ///
    /// Returns the language and score of the highest scoring language.
    /// If there are no alphabetical characters or language can not be determined
    /// it will return unk.
    pub fn identify(&mut self, text: &str) -> (Lang, f32) {
        if self.score_langs(text) {
            self.pick_winner()
        } else {
            (Lang::und, Self::PENALTY_VALUE)
        }
    }

    /// Identify the top k most probable languages of a given text.
    ///
    /// Return the list of top k most probable languages and their scores.
    /// If there are no alphabetical characters or language can not be determined
    /// it will return unk.
    pub fn identify_topk(&mut self, text: &str, k: usize) -> Vec<(Lang, f32)> {
        if self.score_langs(text) {
            self.rank_langs(k)
        } else {
            Vec::from([(Lang::und, Self::PENALTY_VALUE)])
        }
    }

    /// Parallel version of [`Self::identify`]
    ///
    /// Takes an iterator of text instances and returns a [`Vec`] with the results
    pub fn par_identify<I>(&self, texts: I) -> Vec<(Lang, f32)>
        where I: IntoParallelIterator<Item = String>
    {
        // Each thread initializes with its own copy to the identifier object
        thread_local! {
            static IDENTIFIER_LOCAL: Mutex<Option<Identifier>> = Mutex::new(None);
        }

        // Parallelize identification by the number of texts
        texts
            .into_par_iter()
            .map(|text| {
                IDENTIFIER_LOCAL.with(|identifier| {
                    // Only initialize the identifier once
                    let mut identifier = identifier.lock().unwrap();
                    if identifier.is_none() {
                        *identifier = Some(self.clone());
                    }
                    identifier.as_mut().unwrap().identify(&text)
                })
            })
            .collect()
    }

}

#[cfg(test)]
mod tests {
    use crate::identifier::Identifier;
    use heliport_model::lang::Lang;
    use crate::python;
    use pyo3;

    const INPUT_SENTS: [&str;13] = [
        "L'aigua clara",
        "Hola, ¿qué tal?",
        "Korvausinvestoinnit on otettu huomioon liiketoimintasuunnitelmassa rahoituskuluina ja poistoina.",
        "而目前各方都在追问到底谁应该为这场大疫情在中国的扩散承担责任。",
        "Pēc nejaušās izvēles izraudzītas sešas vistas no vielas saņemšanas grupas un sešas vistas no nesēja kontroles grupas, un trīs vistas no pozitīvās kontroles grupas (ja šo grupu pēta paralēli) jānogalina dažas dienas pēc dozēšanas, un galvas smadzenes un muguras smadzenes jāsagatavo un jāanalizē, lai noteiktu ar neiropātiju saistītās esterāzes kavēšanas aktivitāti.",
        "وتؤكد رومانيا على التزامها بمواصلة تنفيذ أحكام جدول أعمال الموئل والمشاركة في التعاون الدولي في هذا المجال الدينامي ، وبالتالي زيادة الاستفادة من الدعم والمساعدة المقدمة في تنفيذ برامجها الوطنية.",
        "Namoota duʼaa kaafaman keessaa hedduun isaanii \"jalʼoota,\" jechuunis namoota dhugaa waaʼee Waaqa keenya Yihowaa fi Ilma isaa dubbatu utuu hin baratin duʼani dha.",
        "DOKUMENT INFORMACYJNY NR [...]",
        "In afwijking van de verplichting van sectie IX, hoofdstuk II, punt III.1.a), van bijlage III van Verordening (EG) nr. 853 / 2004 is het maximale kiemgetal voor rauwe koemelk slechts van toepassing indien deze melk warmtebehandeld moet worden en niet zodanig behandeld is binnen de termijn voor aanvaarding die bepaald is in de door de exploitanten van levensmiddelenbedrijven ingevoerde, op HACCP gebaseerde procedures.",
        "Batangiye gushyiraho imihati myinshi no kumara igihe kinini bakurikirana inyungu z'iby'umwuka, ari na ko bakora uko bashoboye ngo begere Yehova.",
        "The Encyclopedia of Religion gir flere opplysninger: \"Dens visjon av en menneskehet som hadde behov for Kristi evangelium, talte for igangsettelse og rask utvidelse av misjonsvirksomheten, både utenlands og innenlands.\"",
        "Kui lõike 5 alusel vastu võetud tehnilistest rakendusmeetmetest ei tulene teisiti, võivad pädevad riigiasutused võtta vastu suuniseid ja vajaduse korral anda juhiseid selle kohta, millistel asjaoludel peab teenuseosutaja teatama isikuandmetega seotud rikkumisest ning millises vormis ja mil viisil seda tuleb teha.",
        "\u{0aae}\u{0a9c}\u{0abe}\u{0a95} \u{0aa4}\u{0ab0}\u{0ac0}\u{0a95}\u{0ac7} @K.",
    ];
    // Expected predictions from original HeLI
    const EXPECTED_PREDS: [(Lang, f32);13] = [
        (Lang::cat, 1.5613),
        (Lang::spa, 0.2340),
        (Lang::fin, 1.8580),
        (Lang::cmn, 2.5705),
        (Lang::lav, 2.2733),
        (Lang::ara, 2.6973),
        (Lang::gaz, 3.3978),
        (Lang::pol, 0.3492),
        (Lang::nld, 0.7148),
        (Lang::tso, 0.2414),
        (Lang::nob, 0.9093),
        (Lang::est, 2.6729),
        (Lang::und, 0.6115), // In heli this is guj 3.2886949 because of an @ in the text
    ];

    #[test_log::test]
    fn test_output_langs() {
        pyo3::prepare_freethreaded_python();
        let mut identifier = Identifier::load(
            &python::module_path().expect("Python module needs to be installed"),
            None,
        ).expect("Could not load model, please run 'heliport bianrize' if you haven't");

        let pred = identifier.identify(&String::from("Hola, ¿qué tal?"));
        assert_eq!(pred.0, Lang::spa);

        for (text, expected) in INPUT_SENTS.iter().zip(EXPECTED_PREDS) {
            let pred = identifier.identify(&text.to_string());
            assert_eq!(pred.0, expected.0);
        }
    }

    #[test_log::test]
    fn test_output_probs() {
        pyo3::prepare_freethreaded_python();
        let mut identifier = Identifier::load(
            &python::module_path().expect("Python module needs to be installed"),
            None,
        ).expect("Could not load model, please run 'heliport bianrize' if you haven't");

        for (text, expected) in INPUT_SENTS.iter().zip(EXPECTED_PREDS) {
            let pred = identifier.identify(&text.to_string());
            let pred_score = format!("{:.4}", pred.1);
            let expected_score = format!("{:.4}", expected.1);
            assert!(pred_score == expected_score,
                "expected  = {:?}\npredict = {:?}", pred, expected);
        }
    }

    #[test_log::test]
    fn test_confidence() {
        pyo3::prepare_freethreaded_python();
        let mut identifier = Identifier::load(
            &python::module_path().expect("Python module needs to be installed"),
            None,
        ).expect("Could not load model, please run 'heliport bianrize' if you haven't");
        identifier.disable_confidence();

        let pred = identifier.identify("hello");
        assert!(pred.0 == Lang::sah);
    }

}
