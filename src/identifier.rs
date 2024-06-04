use std::collections::BTreeMap;
use std::path::Path;
use std::thread;

use ordered_float::OrderedFloat;
use shingles::AsShingles;
use unicode_blocks;
use regex::Regex;
use log::{debug,warn};

use crate::languagemodel::{Model, ModelType};
use crate::lang::{Lang,LangScores};


pub struct Identifier {
    charmodel: Model,
    wordmodel: Model,
    regex_non_alpha: Regex,
    _regex_spaces: Regex,
    use_confidence: bool,
    number_top_langs: u16,
    lang_points: LangScores,
    word_scores: LangScores,
    heli_score: BTreeMap<OrderedFloat<f32>, Vec<Lang>>,
}


impl Identifier {
    const PENALTY_VALUE : f32 = 7.0;
    const MAX_NGRAM : usize = 6;

    pub fn new(grampath: String, wordpath: String) -> Self {
        let char_handle = thread::spawn(move || {
            let path = Path::new(&grampath);
            Model::from_bin(path)
        });

        let word_handle = thread::spawn(move || {
            let path = Path::new(&wordpath);
            Model::from_bin(path)
        });

        let wordmodel = word_handle.join().unwrap();
        let charmodel = char_handle.join().unwrap();
        assert!(wordmodel.model_type == ModelType::Word);
        assert!(charmodel.model_type == ModelType::Char);
        let regex_non_alpha = Regex::new(r#"[^#gc\p{L}\p{M}′'’´ʹािीुूृेैोौंँः् া ি ী ু ূ ৃ ে ৈ ো ৌ।্্্я̄\u07A6\u07A7\u07A8\u07A9\u07AA\u07AB\u07AC\u07AD\u07AE\u07AF\u07B0\u0A81\u0A82\u0A83\u0ABC\u0ABD\u0ABE\u0ABF\u0AC0\u0AC1\u0AC2\u0AC3\u0AC4\u0AC5\u0AC6\u0AC7\u0AC8\u0AC9\u0ACA\u0ACB\u0ACC\u0ACD\u0AD0\u0AE0\u0AE1\u0AE2\u0AE3\u0AE4\u0AE5\u0AE6\u0AE7\u0AE8\u0AE9\u0AEA\u0AEB\u0AEC\u0AED\u0AEE\u0AEF\u0AF0\u0AF1]"#)
            .expect("Error compiling non-alpha regex for Idenfifier");


        Self {
            charmodel: charmodel,
            wordmodel: wordmodel,
            regex_non_alpha: regex_non_alpha,
            _regex_spaces: Regex::new("  *").expect("Error compiling repeated spaces regex for Identifier"),
            use_confidence: false,
            number_top_langs: 1,
            lang_points: LangScores::new(),
            word_scores: LangScores::new(),
            heli_score: BTreeMap::new(),
        }
    }

    // Compute the ranking of languages
    fn rank_langs(&mut self) -> (Lang, Option<f32>) {
        let mut winner_tuple;
        // if only one lang is requested, just search for the minimum score (winner)
        if self.number_top_langs == 1 {
            let mut min = Self::PENALTY_VALUE + 1.0;
            let mut winner_lang = Lang::Und;

            for lang in Lang::iter() {
                let points = self.lang_points.get(lang);
                if points <= min {
                    min = points;
                    winner_lang = *lang;
                }
            }

            winner_tuple =  (winner_lang, Some(min));
        }
        else {
            //TODO do the actual ranking here, maybe btree is still the fastest way
            // maybe a heap of tuples is faster
            // we also do not need a btree<lang, vec>, if there are ties is fine if its
            // deterministic
            unimplemented!("Top k larger than 1 is not implemented");
        }

        // return macrolang (aka return finnish instead of variants)
        winner_tuple.0 = winner_tuple.0.macrolang();
        winner_tuple
    }

    pub fn identify(&mut self, text: &str) -> (Lang, Option<f32>) {
        // lowercase and remove non-alphabetic characters
        //TODO is it really remove all non alpha? because I found words with punctuation in
        //langmodel entries
        let lowercased = text.to_lowercase();
        let replaced = self.regex_non_alpha.replace_all(&lowercased, " ");
        self.heli_score.clear();

        let mut last_was_cjk = false;
        let mut last_was_space = false;
        let mut cjk_num_chars = 0_usize;
        let mut mystery_text = String::with_capacity(replaced.len());

        for mystery_char in replaced.chars() {
            let charset = match unicode_blocks::find_unicode_block(mystery_char) {
                Some(charset) => charset,
                None => {
                    warn!("Could not find unicode block for '{}'", mystery_char);
                    return (Lang::Und, Some(Self::PENALTY_VALUE));
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
            mystery_text.push(mystery_char);
        }

        debug!("Mystery text: '{}'", mystery_text);
        //debug!("Words: [{:?}]", mystery_text.split_whitespace().format(", "));

        // We don't need to remove repeated spaces
        // split_whitespace ignores them
        let mut words = mystery_text.split_whitespace().peekable();


        if words.peek().is_none() {
            if self.use_confidence && self.number_top_langs == 1 {
                return (Lang::Und, Some(Self::PENALTY_VALUE));
            } else {
                return (Lang::Und, None);
            }
        }

        self.lang_points.reset();

        let mut word_scored;
        let mut num_words = 0;
        let mut mystery_length = 0;
        for word in words {
            debug!("Scoring '{}'", word);
            word_scored = false;
            num_words += 1;
            mystery_length += word.chars().count(); //TODO move this to the cjk count above? .chars() iterator is expensive
            self.word_scores.reset();

            //TODO this condition seems useless, the constant never changes, maybe for debug?
            if Model::MAX_USED < 1.0 {
                if self.wordmodel.dic.contains_key(word) {
                    // found the word in language model
                    // update scores according to each lang that has the word
                    // use penalty value for langs that don't have the word
                    word_scored = true;
                    debug!("word scored");
                    let kiepro = &self.wordmodel.dic[word];
                    debug!("{:?}", kiepro);
                    for lang in Lang::iter() {
                        if kiepro.contains_key(lang) {
                            self.word_scores.insert(lang.clone(), kiepro[lang]);
                        } else {
                            self.word_scores.insert(lang.clone(), Self::PENALTY_VALUE);
                        }
                    }
                }
            }

            //TODO is this really needed? if word is not found it is not scored in the code above
            //so it is still at 0 because it was reset at the beginning of the iteration
            if !word_scored {
                debug!("Word has not been found");
                self.word_scores.reset();
            }

            // Go from highest order ngram to lowest until one of the orders is found in any
            // language
            //TODO does it make sense to explore ngrams longer than the current word?
            let mut score;
            let wordspace = format!(" {word} ");
            for t in (1..Self::MAX_NGRAM+1).rev() {
                if word_scored {
                    break;
                }

                let mut grammaara = 0;
                // Iterate over all possible ngrams of order t, over the current word
                // shingles manages ngram extraction automatically
                // if word has less chars than current ngram size, it won't do nothing
                for gram in wordspace.as_shingles(t) {
                    if self.charmodel.dic.contains_key(gram) {
                        debug!("Word scored in ngram '{gram}'");
                        grammaara += 1;
                        word_scored = true;
                        let kiepro = &self.charmodel.dic[gram];
                        for lang in Lang::iter() {
                            score = self.word_scores.get(lang);
                            if kiepro.contains_key(lang) {
                                self.word_scores.insert(lang.clone(), score + kiepro[lang]);
                            } else {
                                self.word_scores.insert(lang.clone(), score + Self::PENALTY_VALUE);
                            }
                        }
                    }
                }

                if word_scored {
                    // Normalize wordscores by the number of ngrams found in charmodel
                    debug!("Word scores: {:?}", self.word_scores);
                    self.word_scores.norm(grammaara as f32);
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
        for lang in Lang::iter() {
            let lang_score_norm = self.lang_points.get(lang) / num_words as f32;
            self.lang_points.insert(*lang, lang_score_norm);

            if (100 / mystery_length * cjk_num_chars) > 50 {
                if !lang.is_cjk() {
                    self.lang_points.insert(*lang, Self::PENALTY_VALUE + 1.0);
                }
            }
        }
        debug!("Normalized lang points: {:?}", self.lang_points);

        self.rank_langs()
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
