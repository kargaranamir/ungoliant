//! Language classification/identification utilities.
//!
//! Enables language identification on sentences, using [fasttext](https://fasttext.cc) for now.
use fasttext::{FastText, Prediction};

/// Minimum sentence length, used by [valid_len].
const MIN_SENTENCE_LEN: usize = 100;

/// Clean the prediction label field from `__label__xx` into `xx`.
///
/// Be aware that the function only skips 9 chars without doing any parsing,
/// so it may silently fail if `prediction.label.chars().count() > 9`
/// but not of a `__label__xx` form.
///
/// # Errors
/// Returns an error if provided prediction is too short to be cleaned.
fn clean_prediction(prediction: &Prediction) -> Result<Prediction, String> {
    if prediction.label.chars().count() < 9 {
        return Err(format!(
            "Label is too short to be cleaned: {}",
            prediction.label
        ));
    }
    Ok(Prediction {
        prob: prediction.prob,
        label: prediction.label.chars().skip(9).collect(),
    })
}

/// ensure that sentences meet valid requirements
/// to be sent to fasttext:
/// - valid utf8: currently handled upper in the chain because strings can't be invalid utf8
/// - > [MIN_SENTENCE_LEN] > [char]
pub fn valid_len(sentence: &str) -> bool {
    sentence.chars().count() > MIN_SENTENCE_LEN
}

/// Holds a [fasttext::FastText] instance and its parameters.
/// - [Classifier::k], number of predicted languages on a sentence
/// - [Classifier::threshold], prediction threshold
pub struct Classifier {
    predictor: FastText,
    pub k: i32,
    pub threshold: f32,
}

impl Classifier {
    /// Create a new fasttext classifier allowing to identify
    /// language of strings.
    ///
    /// - [Self::k] is set to 1
    /// - [Self::threshold] is set to .8
    ///
    /// **Having `lid.176.bin` at `.` is mandatory**
    ///
    /// # Errors
    /// Propagates [fasttext::FastText] errors.
    pub fn new_lid() -> Result<Self, String> {
        Self::new("lid.176.bin", 1, 0.8)
    }

    /// Create a new fasttext classifier.
    ///
    /// filename has to be a path to a `bin` file.
    ///
    /// See [fasttext::FastText::predict] for other parameters explanation
    pub fn new(filename: &str, k: i32, threshold: f32) -> Result<Self, String> {
        let mut predictor = FastText::new();
        predictor.load_model(filename)?;
        Ok(Classifier {
            predictor,
            k,
            threshold,
        })
    }

    /// predict for supplied sentence.
    /// returns Ok(None) if no reliable identification has been done.
    pub fn predict(&self, sentence: &str) -> Result<Option<Vec<Prediction>>, String> {
        let predictions = self.predictor.predict(&sentence, self.k, self.threshold)?;

        if predictions.is_empty() {
            Ok(None)
        } else {
            // attempt to clean labels before returning
            Ok(Some(
                predictions
                    .into_iter()
                    .map(|p| clean_prediction(&p).unwrap_or(p))
                    .collect(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ambiguous/multilingual sentence that shouldn't yield a single lang with a high confidence
    #[test]
    fn test_no_id() {
        let classifier = Classifier::new_lid().expect("could not instantiate a classifier");
        let short_sentence = "Bonjour Hello";
        let id = classifier
            .predict(short_sentence)
            .expect("could not predict sentence");
        println!("{:?}", id);
        assert!(id.is_none());
    }

    // unilingual longish sentence that should yield a single lang with a high confidence
    #[test]
    fn test_id_en() {
        let classifier = Classifier::new_lid().expect("could not instantiate a classifier");
        let sentence = "a perfectly, innocent, quite lengthy sentence. How lengthy and normal this sentence is, oh my! Lengthy lengthy.".escape_default().to_string();
        let pred = classifier
            .predict(&sentence)
            .expect("could not launch prediction")
            .unwrap();
        assert_eq!(pred.len(), 1);
        let pred = &pred[0];
        assert_eq!(pred.label, "en");
    }
    // test that garbage unicode from CC does not procees to crash the underlying C++ code.
    // when escaped with C++ friendly escape_default() method.
    #[test]
    fn test_garbage() {
        use std::fs;
        let garbage_default = fs::read_to_string("tests/res/garbage.txt")
            .expect("could not find test file")
            .escape_default()
            .to_string();
        let classifier = Classifier::new_lid().expect("could not instantiate a classifier");
        classifier
            .predict(&garbage_default)
            .expect("could not predict sentence");
    }

    // ensures that any null character in string
    // does not crash classifier.
    #[test]
    fn test_null_terminated() {
        let classifier = Classifier::new_lid().expect("could not instantiate a classifier");
        let nullstring = String::from(char::from(0));
        let mut nullstring2 = String::from("hello");
        nullstring2.push(char::from(0));
        nullstring2.push_str(" world!");

        let cls1 = classifier.predict(&nullstring);

        let cls2 = classifier.predict(&nullstring);

        assert!(cls1.is_err());
        assert!(cls2.is_err());
    }
}
