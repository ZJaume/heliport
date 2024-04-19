use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::thread;

use ordered_float::OrderedFloat;
use itertools::Itertools;
use shingles::AsShingles;
use unicode_blocks;
use regex::Regex;
use log::{debug,warn};

use crate::Model;


pub struct Identifier {
    charmodel: Model,
    pub wordmodel: Model,
    regex_non_alpha: Regex,
    _regex_spaces: Regex,
    use_confidence: bool,
    number_top_langs: u16,
    lang_points: HashMap<String, f32>,
    lang_points_final: HashMap<String, f32>,
    word_scores: HashMap<String, f32>,
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

        // both models must have the same languages
        assert!(wordmodel.language_list == charmodel.language_list);
        let mut lang_points: HashMap<String, f32> = HashMap::with_capacity(wordmodel.language_list.len());
        let mut word_scores: HashMap<String, f32> = HashMap::with_capacity(wordmodel.language_list.len());
        for lang in &wordmodel.language_list {
            lang_points.insert(lang.clone(), 0.0);
            word_scores.insert(lang.clone(), 0.0);
        }
        word_scores.shrink_to_fit();
        lang_points.shrink_to_fit();

        Self {
            charmodel: charmodel,
            wordmodel: wordmodel,
            regex_non_alpha: Regex::new(r#"\P{L}"#).expect("Error compiling non-alpha regex for Idenfifier"),
            _regex_spaces: Regex::new("  *").expect("Error compiling repeated spaces regex for Identifier"),
            use_confidence: false,
            number_top_langs: 1,
            lang_points_final: HashMap::with_capacity(lang_points.capacity()),
            lang_points: lang_points,
            word_scores: word_scores,
        }
    }

    // Normalize word scores dividing by a given value
    fn norm_word_scores(&mut self, y: f32) {
        for (_, x) in self.word_scores.iter_mut() {
            *x = *x / y;
        }
    }

    // Reset all word scores to 0
    fn reset_word_scores(&mut self) {
        for (_, val) in self.word_scores.iter_mut() {
            *val = 0.0;
        }
    }

    // Reset all lang points to 0
    fn reset_lang_points(&mut self) {
        for (_, val) in self.lang_points.iter_mut() {
            *val = 0.0;
        }
    }

    // Update lang points by ading current word scores
    fn update_lang_points(&mut self) {
        for (lang, val) in self.lang_points.iter_mut() {
            *val += self.word_scores.get(lang).expect("Lang keys should be the same on both maps!");
        }
    }

    pub fn identify(&mut self, text: &String) -> (String, Option<f32>) {
        // lowercase and remove non-alphabetic characters
        //TODO is it really remove all non alpha? because I found words with punctuation in
        //langmodel entries
        let lowercased = text.to_lowercase();
        let replaced = self.regex_non_alpha.replace_all(&lowercased, " ");
        self.lang_points_final.clear();

        let mut last_was_cjk = false;
        let mut last_was_space = false;
        let mut cjk_num_chars = 0_usize;
        let mut mystery_text = String::with_capacity(replaced.len());

        for mystery_char in replaced.chars() {
            let charset = match unicode_blocks::find_unicode_block(mystery_char) {
                Some(charset) => charset,
                None => {
                    warn!("Could not find unicode block for '{}'", mystery_char);
                    return (String::from("und"), Some(Self::PENALTY_VALUE));
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
                return ("und".to_string(), Some(Self::PENALTY_VALUE));
            } else {
                return ("und".to_string(), None);
            }
        }

        //TODO can this speed up if are class member variables and allocated only once?
        //will that be possible if we want to do multiple parallel calls to this function?
        //let lang_scores = vec![0.0; self.wordmodel.language_list.len()];
        self.reset_lang_points();

        let mut word_scored;
        let mut num_words = 0;
        let mut mystery_length = 0;
        let mut word;
        for iword in words {
            //TODO the original implementation inserts spaces at the beginning and end of the current word
            //if the word is the last, it only adds at the beginning
            //I found this space thing completely useless, the matches will be the same without the
            //spacing. Also, the original omits space at the end for the last word could be a bug?
            //that way last words are never taken into account because the wordmodel is loading all
            //the words with space at the begining and at the end. is this intended?
            //TODO let's try with the spaces and see what happens
            word = format!(" {iword} ");

            debug!("Scoring '{}'", word);
            word_scored = false;
            num_words += 1;
            mystery_length += word.len();
            self.reset_word_scores();

            //TODO this condition seems useless, the constant never changes, maybe for debug?
            if Model::MAX_USED < 1.0 {
                if self.wordmodel.dic.contains_key(&word) {
                    // found the word in language model
                    // update scores according to each lang that has the word
                    // use penalty value for langs that don't have the word
                    word_scored = true;
                    debug!("word scored");
                    let kiepro = &self.wordmodel.dic[&word];
                    for lang in &self.wordmodel.language_list {
                        if kiepro.contains_key(lang) {
                            let prob = kiepro[lang];
                            // debug!("{lang}: {prob}");
                            self.word_scores.insert(lang.to_string(), kiepro[lang]);
                        } else {
                            self.word_scores.insert(lang.to_string(), Self::PENALTY_VALUE);
                        }
                    }
                }
            }

            //TODO is this really needed? if word is not found it is not scored in the code above
            //so it is still at 0 because it was reset at the beginning of the iteration
            if !word_scored {
                debug!("Word has not been found");
                self.reset_word_scores();
            }

            // Go from highest order ngram to lowest until one of the orders is found in any
            // language
            //TODO does it make sense to explore ngrams longer than the current word?
            let mut score;
            let mut prob;
            for t in (1..Self::MAX_NGRAM+1).rev() {
                if word_scored {
                    break;
                }

                let mut grammaara = 0;
                // Iterate over all possible ngrams of order t, over the current word
                // shingles manages ngram extraction automatically
                // if word has less chars than current ngram size, it won't do nothing
                for gram in word.as_shingles(t) {
                    if self.charmodel.dic.contains_key(gram) {
                        debug!("Word scored in ngram '{gram}'");
                        grammaara += 1;
                        word_scored = true;
                        let kiepro = &self.charmodel.dic[gram];
                        for lang in &self.charmodel.language_list {
                            score = self.word_scores.get(&lang.to_string())
                                .expect("All the langs should be already in the map!");
                            if kiepro.contains_key(lang) {
                                prob = kiepro[lang];
                                debug!("{lang}: {score} {prob}");
                                self.word_scores.insert(lang.to_string(), score + kiepro[lang]);
                            } else {
                                self.word_scores.insert(lang.to_string(), score + Self::PENALTY_VALUE);
                            }
                        }
                    }
                }

                if word_scored {
                    // Normalize wordscores by the number of ngrams found in charmodel
                    debug!("Word scores: {:?}", self.word_scores);
                    self.norm_word_scores(grammaara as f32);
                }
            }

            // accumulate wordscores for the current word in the global lang points
            self.update_lang_points();
            debug!("Word scores: {:?}", self.word_scores);
            debug!("Lang points: {:?}", self.lang_points);
        }

        debug!("Finished scoring");
        // Choose the winner
        let mut lang_score;
        // the original code adds "und" but seems to not take it into consideration
        //self.lang_points.insert("und".to_string(), Self::PENALTY_VALUE + 1.0);
        debug!("Lang points: {:?}", self.lang_points);

        //TODO try to simplify this
        // the CJK fix could just finish early?
        // keep two maps of scores to handle all that 3/6 letter codes seems unefficient
        // maybe can be done in a different way?
        for lang in &self.wordmodel.language_list {
            self.lang_points.insert(
                lang.clone(),
                self.lang_points[lang] / num_words as f32,
            );
            if (100 / mystery_length * cjk_num_chars) > 50 {
                if lang != "jpn" && lang != "kor" && lang != "cmn" {
                    self.lang_points_final.insert(lang.clone(), Self::PENALTY_VALUE + 1.0);
                }
            }

            // we store only 3-letter codes in lang_points_final
            // keep the lowest (best) score between all the subfamiles of a 3-letter code
            lang_score = *self.lang_points.get(lang).expect("Should have all langs!");
            if self.lang_points_final.contains_key(&lang[0..3]) {
                if lang_score < self.lang_points_final[&lang[0..3]] {
                    self.lang_points_final.insert(String::from(&lang[0..3]), lang_score);
                }
            } else {
                self.lang_points_final.insert(String::from(&lang[0..3]), lang_score);
            }
        }
        debug!("Normalized lang points: {:?}", self.lang_points_final);

        // Rank languages
        let mut heli_score = BTreeMap::<OrderedFloat<f32>, Vec<String>>::new();
        let mut score: OrderedFloat<f32>;
        for lang in &self.wordmodel.language_list {
            score = OrderedFloat(self.lang_points_final[&lang[0..3]]);
            heli_score.entry(score)
                .and_modify(|langs| langs.push(lang.clone()))
                .or_insert(vec![lang.clone()]);
        }
        debug!("Ranking: {:?}", heli_score);

        // return winner for top k = 1
        // do not choose at random if there is tie, unlike the original code does
        // I do not want undeterministic output
        if self.number_top_langs == 1 {
            if let Some(winners) = heli_score.first_key_value() {
                if winners.1.len() == 0 {
                    panic!("winners should not be empty!");
                }
                return (winners.1[0].clone(), Some(winners.0.into_inner()));
            } else {
                panic!("heli_score should not be empty!");
            }
        }

        ("und".to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use crate::identifier::Identifier;

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
    const EXPECTED_PREDS: [(&str, Option<f32>);12] = [
        ("cat", Some(3.9435382)),
        ("cat", Some(4.047136 )),
        ("fin", Some(3.4751024)),
        ("cmn", Some(4.429534 )),
        ("lav", Some(3.6547067)),
        ("ara", Some(3.468608 )),
        ("fin", Some(6.273052 )),
        ("pol", Some(3.8661745)),
        ("nld", Some(3.5002592)),
        ("tso", Some(5.6970944)),
        ("nob", Some(3.548138 )),
        ("est", Some(3.4789875)),
    ];

    #[test_log::test]
    fn test_output_langs() {
        let mut identifier = Identifier::new(String::from("gramdict.ser"),
                                         String::from("wordict.ser"));

        let pred = identifier.identify(&String::from("Hola, qué tal?"));
        assert_eq!(pred.0, "cat");

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
        assert_eq!(pred, ("cat".to_string(), Some(4.047136_f32)));

        for (text, expected) in INPUT_SENTS.iter().zip(EXPECTED_PREDS) {
            let pred = identifier.identify(&text.to_string());
            let pred_score = format!("{:.3}", pred.1.expect("Shouldn't be a none"));
            let expected_score = format!("{:.3}", expected.1.expect("Shouldn't be a none"));
            assert!(pred_score == expected_score,
                "expected  = {:?}\npredict = {:?}", pred, expected);
        }
    }
}
