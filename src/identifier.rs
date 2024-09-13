use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use ordered_float::OrderedFloat;
use strum::{IntoEnumIterator, EnumCount};
use shingles::AsShingles;
use unicode_blocks;
use regex::Regex;
use anyhow::Result;
use log::{debug,warn};
use lazy_static::lazy_static;
use rayon::prelude::*;

use crate::languagemodel::Model;
use crate::lang::{Lang, LangScores, LangBitmap};

lazy_static! {
    static ref RE_NON_ALPHA: Regex = Regex::new(r#"[^#gc\p{L}\p{M}′'’´ʹािीुूृेैोौंँः् া ি ী ু ূ ৃ ে ৈ ো ৌ।্্্я̄\u07A6\u07A7\u07A8\u07A9\u07AA\u07AB\u07AC\u07AD\u07AE\u07AF\u07B0\u0A81\u0A82\u0A83\u0ABC\u0ABD\u0ABE\u0ABF\u0AC0\u0AC1\u0AC2\u0AC3\u0AC4\u0AC5\u0AC6\u0AC7\u0AC8\u0AC9\u0ACA\u0ACB\u0ACC\u0ACD\u0AD0\u0AE0\u0AE1\u0AE2\u0AE3\u0AE4\u0AE5\u0AE6\u0AE7\u0AE8\u0AE9\u0AEA\u0AEB\u0AEC\u0AED\u0AEE\u0AEF\u0AF0\u0AF1]"#)
            .expect("Error compiling non-alpha regex for Idenfifier");
}

pub struct Identifier {
    model: Arc<Model>,
    lang_scored: LangBitmap,
    lang_points: LangScores,
    word_scores: LangScores,
    heli_score: BTreeMap<OrderedFloat<f32>, Vec<Lang>>,
}


impl Identifier {
    const PENALTY_VALUE : f32 = 7.0;
    const MAX_NGRAM : usize = 6;

    pub fn load(modelpath: &str) -> Result<Self> {
        Ok(Self::new(Arc::new(Model::load(modelpath)?)))
    }

    pub fn new(model: Arc<Model>) -> Self {
        Self {
            model: model,
            lang_scored: LangBitmap::new(),
            lang_points: LangScores::new(),
            word_scores: LangScores::new(),
            heli_score: BTreeMap::new(),
        }
    }

    /// Get the most probable language according to the current language scores
    fn pick_winner(&mut self) -> (Lang, Option<f32>) {
        // if only one lang is requested, just search for the minimum score (winner)
        let mut min = Self::PENALTY_VALUE + 1.0;
        let mut winner_lang = Lang::unk;

        for lang in Lang::iter() {
            let points = self.lang_points.get(lang);
            if points <= min {
                min = points;
                winner_lang = lang;
            }
        }

        (winner_lang, Some(min))
    }

    /// Build a ranking of the top k scoring languages,
    /// according to the current language scores
    fn rank_langs(&mut self, k: usize) -> Vec<(Lang, Option<f32>)> {
        //TODO do the actual ranking here, maybe btree is still the fastest way
        // maybe a heap of tuples is faster
        // we also do not need a btree<lang, vec>, if there are ties is fine if its
        // deterministic
        // unimplemented!("Top k larger than 1 is not implemented");
        self.heli_score.clear();
        let mut winners = Vec::new();
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
        for _ in 0..k {
            if let Some((score, langs)) = self.heli_score.pop_first() {
                for lang in langs {
                    winners.push((lang, Some(score.into_inner())));
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
        let lowercased = text.to_lowercase();
        let replaced = RE_NON_ALPHA.replace_all(&lowercased, " ");
        self.heli_score.clear();

        let mut last_was_cjk = false;
        let mut last_was_space = false;
        let mut cjk_num_chars = 0_usize;
        let mut mystery_text = String::with_capacity(replaced.len());
        let mut mystery_length = 0;

        for mystery_char in replaced.chars() {
            let charset = match unicode_blocks::find_unicode_block(mystery_char) {
                Some(charset) => charset,
                None => {
                    warn!("Could not find unicode block for '{}'", mystery_char);
                    return false
                }
            };

            if unicode_blocks::is_cjk_block(charset) {
                if !last_was_cjk && !last_was_space {
                    mystery_text.push(' ');
                }
                last_was_cjk = true;
                last_was_space = false;
                cjk_num_chars += 1;
            } else {
                if last_was_cjk && mystery_char != ' ' {
                    mystery_text.push(mystery_char);
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
            cjk_pct = 0;
        } else {
            cjk_pct = 100 / mystery_length * cjk_num_chars
        }
        for lang in Lang::iter() {
            let lang_score_norm = self.lang_points.get(lang) / num_words as f32;
            self.lang_points.insert(lang, lang_score_norm);

            if cjk_pct > 50 && !lang.is_cjk() {
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
    pub fn identify(&mut self, text: &str) -> (Lang, Option<f32>) {
        if self.score_langs(text) {
            self.pick_winner()
        } else {
            (Lang::unk, Some(Self::PENALTY_VALUE))
        }
    }

    /// Identify the top k most probable languages of a given text.
    ///
    /// Return the list of top k most probable languages and their scores.
    /// If there are no alphabetical characters or language can not be determined
    /// it will return unk.
    pub fn identify_top_k(&mut self, text: &str, k: usize) -> Vec<(Lang, Option<f32>)> {
        if self.score_langs(text) {
            self.rank_langs(k)
        } else {
            Vec::from([(Lang::unk, Some(Self::PENALTY_VALUE))])
        }
    }

    /// Parallel version of [`Self::identify`]
    ///
    /// Takes an iterator of text instances and returns a [`Vec`] with the results
    pub fn par_identify<I>(&self, texts: I) -> Vec<(Lang, Option<f32>)>
        where I: IntoParallelIterator<Item = String>
    {
        // Each thread initializes with its own reference to the identifier object
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
                        *identifier = Some(Identifier::new(self.model.clone()));
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
    use crate::lang::Lang;

    const INPUT_SENTS: [&str;12] = [
        "L'aigua clara",
        "Hola, qué tal?",
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
    ];
    // Expected predictions from original HeLI
    const EXPECTED_PREDS: [(Lang, Option<f32>);12] = [
        (Lang::Cat, Some(3.9435382)),
        (Lang::Cat, Some(4.047136 )),
        (Lang::Fin, Some(3.4751024)),
        (Lang::Cmn, Some(4.429534 )),
        (Lang::Lav, Some(3.6547067)),
        (Lang::Ara, Some(3.468608 )),
        (Lang::Fin, Some(6.273052 )),
        (Lang::Pol, Some(3.8661745)),
        (Lang::Nld, Some(3.5002592)),
        (Lang::Tso, Some(5.6970944)),
        (Lang::Nob, Some(3.548138 )),
        (Lang::Est, Some(3.4789875)),
    ];

    #[test_log::test]
    fn test_output_langs() {
        let mut identifier = Identifier::new(String::from("gramdict.ser"),
                                         String::from("wordict.ser"));

        let pred = identifier.identify(&String::from("Hola, qué tal?"));
        assert_eq!(pred.0, Lang::Cat);

        for (text, expected) in INPUT_SENTS.iter().zip(EXPECTED_PREDS) {
            let pred = identifier.identify(&text.to_string());
            assert_eq!(pred.0, expected.0);
        }
    }

    #[ignore]
    #[test_log::test]
    fn test_output_probs() {
        let mut identifier = Identifier::new(String::from("gramdict.ser"),
                                         String::from("wordict.ser"));

        let pred = identifier.identify(&String::from("Hola, qué tal?"));
        assert_eq!(pred, (Lang::Cat, Some(4.047136_f32)));

        for (text, expected) in INPUT_SENTS.iter().zip(EXPECTED_PREDS) {
            let pred = identifier.identify(&text.to_string());
            let pred_score = format!("{:.3}", pred.1.expect("Shouldn't be a none"));
            let expected_score = format!("{:.3}", expected.1.expect("Shouldn't be a none"));
            assert!(pred_score == expected_score,
                "expected  = {:?}\npredict = {:?}", pred, expected);
        }
    }
}
