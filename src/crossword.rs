use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::{placed_word::PlacedWord, utils::{CrosswordChar, CrosswordString}, word::{Direction, Position, Word}};

/// Error type for possible errors when working with crosswords
#[derive(Error, Debug)]
pub enum CrosswordError
{
    #[error("Cannot add the word to the crossword.")]
    CantAddWord,
    #[error("The word is already in the crossword.")]
    WordAlreadyExists
}

/// Represents a constraint on a [crossword](Crossword)
/// ```text
/// //MaxArea(46)        MaxLength(7) 
/// // satisfied         unsatisfied
/// //                
/// //                        8
/// //                 < - - - - - - >
/// //                 ---------------
/// //MaxHeight(6)  ^ |h e l l o      |
/// // satisfied    | |      i        |
/// //            6 | |      k        |
/// //              | |      e n t e r|
/// //              | |      l        |
/// //              v |      y        |
/// //                 ---------------
/// ```
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum CrosswordConstraint
{
    None,
    MaxLength(u16),
    MaxHeight(u16),
    MaxArea(u32)
}

impl CrosswordConstraint
{
    fn check<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, crossword: &Crossword<CharT, StrT>) -> bool
    {
        match *self
        {
            CrosswordConstraint::None => true,
            CrosswordConstraint::MaxLength(length) => 
            {
                let size = crossword.get_size();
                size.0 <= length
            }
            CrosswordConstraint::MaxHeight(height) => 
            {
                let size = crossword.get_size();
                size.1 <= height
            }
            CrosswordConstraint::MaxArea(area) => 
            {
                let size = crossword.get_size();
                size.0 as u32 * size.1 as u32 <= area
            }
        }
    }

    /// A constraint is recoverable if adding a new word to a crossword that doesn't meet the requirement can make the crossword to meet the requirement
    /// 
    /// For example a requirement on minimum word count is recoverable
    fn recoverable(&self) -> bool
    {
        match *self
        {
            CrosswordConstraint::None => false,
            CrosswordConstraint::MaxLength(_) => false,
            CrosswordConstraint::MaxHeight(_) => false,
            CrosswordConstraint::MaxArea(_) => false,
        }
    }
}

/// Represents all settigns for a [crossword](Crossword)
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct CrosswordSettings
{
    pub constraints: Vec<CrosswordConstraint>
}

impl CrosswordSettings
{
    pub fn check_recoverable_constraints<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, crossword: &Crossword<CharT, StrT>) -> bool
    {
        self.constraints.iter().filter(|constr| constr.recoverable()).all(|constr| constr.check(crossword))
    }

    pub fn check_nonrecoverables_constraints<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, crossword: &Crossword<CharT, StrT>) -> bool
    {
        self.constraints.iter().filter(|constr| !constr.recoverable()).all(|constr| constr.check(crossword))
    }
}

/// Represents settings that dictate how two [words](PlacedWord) are allowed to be relatively positioned in a [crossword](Crossword) when not intersecting
/// 
/// 
/// # Examples
/// 
/// ```text
///                   -------------
/// side_by_side <-> |h e l l o    |
///                  |    w o r l d|
///                   -------------
/// 
///                   -------------------
/// head_by_head <-> |h e l l o w o r l d|
///                   ------------------- 
/// 
///                   ---------
/// side_by_head <-> |h e l l o|
///                  |    w    |
///                  |    o    |
///                  |    r    |
///                  |    l    |
///                  |    d    |
///                   ---------
/// 
/// 
///                       -----------
/// corner_by_corner <-> |  h e l l o|
///                      |w          |
///                      |o          |
///                      |r          |
///                      |l          |
///                      |d          |
///                       -----------
/// 
/// true == allowed
/// false == not allowed
/// ```
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct WordCompatibilitySettings
{
    pub side_by_side: bool,
    pub head_by_head: bool,
    pub side_by_head: bool,
    pub corner_by_corner: bool
}

impl WordCompatibilitySettings 
{
    /// Checks if two [words](PlacedWord) are compatible
    pub fn are_words_compatible<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, first: &PlacedWord<CharT, StrT>, second: &PlacedWord<CharT, StrT>) -> bool
    {
        if first.corners_touch(second) && !self.corner_by_corner { return false; }

        if first.direction == second.direction
        {
            if first.head_touches_head(second) && !self.head_by_head { return false; }
            if first.side_touches_side(second) && !self.side_by_side { return false; }
            if first.intersects(second) { return false; }

            true
        }
        else
        {
            if first.side_touches_head(second) && !self.side_by_head { return false; }
            if first.intersects(second)
            {
                let (first_ind, second_ind) = first.get_intersection_indices(second).unwrap();
                let first_char = first.value.as_ref().iter().nth(first_ind as usize);
                let second_char = second.value.as_ref().iter().nth(second_ind as usize);
        
                return first_char.is_some() && second_char.is_some() && (first_char == second_char);
            }

            true
        }
    }
}

impl Default for WordCompatibilitySettings 
{
    fn default() -> Self 
    {
        WordCompatibilitySettings 
        {
            side_by_side: false,
            head_by_head: false,
            side_by_head: false,
            corner_by_corner: true
        }    
    }
}

/// # Represents a crossword
/// 
/// A crossword can't have two [words](PlacedWord) with the same string value in it.
/// 
/// A crossword is always normalized, meaning all possible coordinates of words are positive, and the minimums are 0
/// 
/// Normalization means shifting coordinates of all words in a way, that ensures that the minimum x and y values in all words will be 0s
/// # Example
/// 
/// ```
/// # use crossword_generator::word::{Direction, Position};
/// # use crossword_generator::placed_word::PlacedWord;
/// # use crossword_generator::crossword::{Crossword, WordCompatibilitySettings};  
/// 
/// let mut cw1 = Crossword::default();
/// let mut cw2 = Crossword::default();
/// 
/// // add_words normalizes the crossword only after adding all words
/// cw1.add_words([PlacedWord::new("hello".to_owned(), Position{ x: 0, y: 3 }, Direction::Right),
///                PlacedWord::new("world".to_owned(), Position{ x: 2, y: 0 }, Direction::Down)].into_iter()).unwrap();
/// 
/// // add_word normalizes the crossword after adding the word
/// cw2.add_word(PlacedWord::new("hello".to_owned(), Position{ x: 0, y: 3 }, Direction::Right)).unwrap();
/// 
/// // so adding a horizontal word at position (0, 3) creates redundant rows (indexes 0, 1, 2), because of that normalization shifts words 3 rows up
/// // effectively adding the word at position (0, 0) 
/// // And the next word will need to be added at (2, -3)
/// cw2.add_word(PlacedWord::new("world".to_owned(), Position{ x: 2, y: -3 }, Direction::Down)).unwrap();
/// 
/// assert_eq!(cw1, cw2)
/// ```
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct Crossword<CharT: CrosswordChar, StrT: CrosswordString<CharT>>
{
    words: BTreeSet<PlacedWord<CharT, StrT>>,
    #[serde(skip)]
    pub word_compatibility_settings: WordCompatibilitySettings
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> Crossword<CharT, StrT>
{
    fn normalize(&mut self)
    {
        let mut min_corner = (i16::MAX, i16::MAX);
        let mut new_set = BTreeSet::new();

        for word in self.words.iter()
        {
            min_corner.0 = min_corner.0.min(word.position.x);
            min_corner.1 = min_corner.1.min(word.position.y);
        }

        for word in self.words.iter()
        {
            let mut new_word = word.clone();
            new_word.position = Position { x: word.position.x - min_corner.0, y: word.position.y - min_corner.1};
            new_set.insert(new_word);
        }

        self.words = new_set;
    }

    /// Creates a new empty crossword with provided [settings](WordCompatibilitySettings)
    pub fn new(word_compatibility_settings: WordCompatibilitySettings) -> Crossword<CharT, StrT>
    {
        Crossword{ word_compatibility_settings, ..Default::default() }
    }

    /// Checks if a [word](PlacedWord) can be added to the [crossword](Crossword) 
    /// 
    /// # Example
    /// 
    /// ```
    /// # use crossword_generator::word::{Direction, Position};
    /// # use crossword_generator::placed_word::PlacedWord;
    /// # use crossword_generator::crossword::{Crossword, WordCompatibilitySettings};                                         
    /// let mut cw = Crossword::new(WordCompatibilitySettings::default());                                  //     ---------
    ///                                                                                                     //    |h e l l o|
    /// cw.add_word(PlacedWord::<u8, &str>::new("hello", Position{x: 0, y: 0}, Direction::Right));          //    |    o    |
    /// cw.add_word(PlacedWord::<u8, &str>::new("local", Position{x: 2, y: 0}, Direction::Down));           //    |    c    |
    ///                                                                                                     //    |    a    |
    ///                                                                                                     //    |    l    |
    ///                                                                                                     //     ---------
    ///                                                                                             
    /// assert!(cw.can_word_be_added(&PlacedWord::new("halo", Position { x: 0, y: 0 }, Direction::Down)));
    /// ```
    /// 
    /// Note that for example word halo on position (3, -2) and direction down is not allowed by a setting in word compatibility settings that forbids two words with same direction to be side to side
    pub fn can_word_be_added(&self, word: &PlacedWord<CharT, StrT>) -> bool
    {
        self.words.iter().all(|w| self.word_compatibility_settings.are_words_compatible(w, word))
    }

    /// Finds the [word](PlacedWord) given its string value.
    pub fn find_word(&self, word: &StrT) -> Option<&PlacedWord<CharT, StrT>>
    {
        self.words.iter().find(|w| w.value == *word)
    }

    fn add_word_unnormalized(&mut self, word: PlacedWord<CharT, StrT>) -> Result<(), CrosswordError>
    {
        if self.find_word(&word.value).is_some() { Err(CrosswordError::WordAlreadyExists) }
        else if !self.can_word_be_added(&word) { Err(CrosswordError::CantAddWord) }
        else { self.words.insert(word); Ok(()) }
    }

    /// Adds the [word](PlacedWord) to the [crossword](Crossword)
    /// Normalizes the crossword after adding the word
    /// 
    /// # Errors
    /// 
    /// [CrosswordError::CantAddWord] - Word can't be added because it's violates the [word compatilibity settings](WordCompatibilitySettings) or has conflict with some other word
    /// [CrosswordError::WordAlreadyExists] - A word with same value already exists in the crossword (yeah, this is not allowed)
    pub fn add_word(&mut self, word: PlacedWord<CharT, StrT>) -> Result<(), CrosswordError>
    {
        self.add_word_unnormalized(word)?;
        self.normalize();
        Ok(())
    }  

    /// Adds the [words](PlacedWord) to the [crossword](Crossword)
    /// Only normalizes the crossword after adding all the words. 
    /// Note, that it's different from calling [Crossword::add_word] in a loop
    /// 
    /// 
    /// # Errors
    /// 
    /// [CrosswordError::CantAddWord] - Word can't be added because it's violates the [word compatilibity settings](WordCompatibilitySettings) or has conflict with some other word
    /// [CrosswordError::WordAlreadyExists] - A word with same value already exists in the crossword (yeah, this is not allowed)
    pub fn add_words(&mut self, mut words: impl Iterator<Item = PlacedWord<CharT, StrT>>) -> Result<(), CrosswordError>
    {
        let res = words.try_for_each(|w| self.add_word_unnormalized(w));
        self.normalize();
        res
    }

    /// Removes the [word](PlacedWord) from the [crossword](Crossword) if finded
    /// 
    /// returns true if the word was succesfully removed
    /// returns false if a word with provaded value was not found
    /// 
    /// (normalizes the crossword after removing the word)
    pub fn remove_word(&mut self, word: &StrT) -> bool
    {
        if let Some(word) = self.find_word(word).cloned()
        {
            self.words.remove(&word);

            self.normalize();

            true
        }
        else { false }
    }

    /// Checks if another [crossword](Crossword) is found inside this crossword.
    /// 
    /// # Example
    /// 
    /// ```
    /// # use crossword_generator::word::{Direction, Position};
    /// # use crossword_generator::placed_word::PlacedWord;
    /// # use crossword_generator::crossword::{Crossword, WordCompatibilitySettings};  
    /// 
    /// // allowing two words to be side by side
    /// let wcs = WordCompatibilitySettings { side_by_side: true, ..Default::default() };
    ///                                                     
    /// let mut cw1 = Crossword::<u8, &str>::new(wcs.clone());                                               //     ---------
    ///                                                                                                      //    |h e l l o|
    /// cw1.add_word(PlacedWord::new("hello", Position { x: 0, y: 0 }, Direction::Right));                   //    |    o    |
    /// cw1.add_word(PlacedWord::new("local", Position { x: 2, y: 0 }, Direction::Down));                    //    |    c a t|
    /// cw1.add_word(PlacedWord::new("cat", Position { x: 2, y: 2 }, Direction::Right));                     //    |    a n o|
    /// cw1.add_word(PlacedWord::new("and", Position { x: 3, y: 2 }, Direction::Down));                      //    |    l d y|
    /// cw1.add_word(PlacedWord::new("toy", Position { x: 4, y: 2 }, Direction::Down));                      //     ---------
    ///                                                                                                      
    ///                                                                                         
    ///
    /// let mut cw2 = Crossword::new(wcs.clone());                                                           //     -----                 
    ///                                                                                                      //    |c a t|
    /// cw2.add_word(PlacedWord::new("cat", Position { x: 0, y: 0 }, Direction::Right));                     //    |  n o|
    /// cw2.add_word(PlacedWord::new("and", Position { x: 1, y: 0 }, Direction::Down));                      //    |  d y|
    /// cw2.add_word(PlacedWord::new("toy", Position { x: 2, y: 0 }, Direction::Down));                      //     -----
    ///     
    /// assert!(cw1.contains_crossword(&cw2));
    /// ```
    pub fn contains_crossword(&self, other: &Crossword<CharT, StrT>) -> bool 
    {
        if other.words.len() > self.words.len() { return false; }
        let mut offset: Option<(i16, i16)> = None;
        for other_word in other.words.iter()
        {
            let cur_word = self.find_word(&other_word.value);
            if cur_word.is_none()
            {
                return false;
            }
            let cur_word = cur_word.unwrap();
            if cur_word.direction != other_word.direction
            {
                return false;
            }

            match &offset
            {
                None => offset = Some((cur_word.position.x - other_word.position.x, cur_word.position.y - other_word.position.y)),
                Some(offset) => 
                {
                    if *offset != (cur_word.position.x - other_word.position.x, cur_word.position.y - other_word.position.y)
                    {
                        return false;
                    }
                }
            }

        }
        true
    }

    /// Returns all possible ways to add a [word](Word) into the [crossword](Crossword)
    /// 
    /// # Example
    /// 
    /// ```
    /// # use crossword_generator::word::{Word, Direction, Position};
    /// # use crossword_generator::placed_word::PlacedWord;
    /// # use crossword_generator::crossword::{Crossword, WordCompatibilitySettings};         
    /// # use std::collections::BTreeSet;   
    ///
    ///                                           
    /// let mut cw = Crossword::default();                                                                  //     ---------
    ///                                                                                                     //    |h e l l o|
    /// cw.add_word(PlacedWord::<u8, &str>::new("hello", Position{x: 0, y: 0}, Direction::Right));          //    |    o    |
    /// cw.add_word(PlacedWord::<u8, &str>::new("local", Position{x: 2, y: 0}, Direction::Down));           //    |    c    |
    ///                                                                                                     //    |    a    |
    ///                                                                                                     //    |    l    |
    ///                                                                                                     //     ---------
    ///                                                                                         
    /// assert_eq!(cw.calculate_possible_ways_to_add_word(&Word::new("halo", None)), 
    ///             BTreeSet::from([
    ///     PlacedWord::new("halo", Position { x: 0, y: 0 }, Direction::Down),
    ///     PlacedWord::new("halo", Position { x: 4, y: -3 }, Direction::Down),
    ///     PlacedWord::new("halo", Position { x: 0, y: 4 }, Direction::Right),
    ///     PlacedWord::new("halo", Position { x: 1, y: 3 }, Direction::Right),
    /// ]));
    /// ```
    /// 
    /// 
    /// 
    /// Note that for example word halo on position 3 -2 and direction down is not allowed by a setting in word compatibility settings that forbids two words with same direction to be side to side
    pub fn calculate_possible_ways_to_add_word(&self, word: &Word<CharT, StrT>) -> BTreeSet<PlacedWord<CharT, StrT>>
    {
        if self.words.is_empty()
        {
            return vec![PlacedWord::new(word.value.clone(), Position::default(), Direction::default())].into_iter().collect()
        }

        self.words.iter()
            .flat_map(|cur_word: &PlacedWord<_, _>  | cur_word.calculate_possible_ways_to_add_word(word))
            .filter(|w: &PlacedWord<_, _>| self.can_word_be_added(w))
            .collect()
    }

    /// Returns the size of the minimum rectangle that can contain the [crossword](Crossword)
    /// 
    /// # Example
    /// 
    /// ```
    /// # use crossword_generator::word::{Direction, Position};
    /// # use crossword_generator::placed_word::PlacedWord;
    /// # use crossword_generator::crossword::Crossword;                                         
    /// let mut cw = Crossword::default();                                                                  //     ---------
    ///                                                                                                     //    |h e l l o|
    /// cw.add_word(PlacedWord::<u8, &str>::new("hello", Position{x: 0, y: 0}, Direction::Right));          //    |    o    |
    /// cw.add_word(PlacedWord::<u8, &str>::new("local", Position{x: 2, y: 0}, Direction::Down));           //    |    c    |
    ///                                                                                                     //    |    a    |
    ///                                                                                                     //    |    l    |
    ///                                                                                                     //     ---------
    /// assert_eq!(cw.get_size(), (5, 5));
    pub fn get_size(&self) -> (u16, u16)
    {
        let mut max_corner = (0i16, 0i16);
    
        for word in self.words.iter()
        {
            max_corner.0 = max_corner.0.max(word.position.x + 1);
            max_corner.1 = max_corner.1.max(word.position.y + 1);
            match word.direction
            {
                Direction::Right => max_corner.0 = max_corner.0.max(word.position.x + word.value.as_ref().iter().count() as i16),
                Direction::Down => max_corner.1 = max_corner.1.max(word.position.y + word.value.as_ref().iter().count() as i16), 
            }
        }
    
        (max_corner.0 as u16, max_corner.1 as u16)
    }

    /// Returns a matrix of characters that represent the [crossword](Crossword)
    /// 
    /// # Example
    /// 
    /// ```
    /// # use crossword_generator::word::{Direction, Position};
    /// # use crossword_generator::placed_word::PlacedWord;
    /// # use crossword_generator::crossword::Crossword;                                         
    /// let mut cw = Crossword::default();                                                                  //     ---------
    ///                                                                                                     //    |h e l l o|
    /// cw.add_word(PlacedWord::<u8, &str>::new("hello", Position{x: 0, y: 0}, Direction::Right));          //    |    o    |
    /// cw.add_word(PlacedWord::<u8, &str>::new("local", Position{x: 2, y: 0}, Direction::Down));           //    |    c    |
    ///                                                                                                     //    |    a    |
    ///                                                                                                     //    |    l    |
    ///                                                                                                     //     ---------
    /// assert_eq!(cw.generate_char_table(), vec!
    /// [
    ///     vec![ b'h',  b'e', b'l',  b'l',  b'o'],    
    ///     vec![b'\0', b'\0', b'o', b'\0', b'\0'],
    ///     vec![b'\0', b'\0', b'c', b'\0', b'\0'],
    ///     vec![b'\0', b'\0', b'a', b'\0', b'\0'],
    ///     vec![b'\0', b'\0', b'l', b'\0', b'\0']
    /// ]);   
    /// 
    /// // uses the default value for the empty cells                                              
    /// ```

    pub fn generate_char_table(&self) ->Vec<Vec<CharT>>
    {
        let size = self.get_size();
        let mut table = vec![vec![CharT::default(); size.0 as usize]; size.1 as usize];
        for word in self.words.iter()
        {
            for (index, char) in word.value.as_ref().iter().enumerate()
            {
                match word.direction
                {
                    Direction::Right => table[word.position.y as usize][word.position.x as usize + index] = char.clone(),
                    Direction::Down => table[word.position.y as usize + index][word.position.x as usize] = char.clone(),
                }
            }
        }
    
        table
    }

    pub fn convert_to<StrT2: CrosswordString<CharT>>(self, f: impl Fn(StrT) -> StrT2) -> Crossword<CharT, StrT2>
    {
        let mut res = Crossword::default();

        res.add_words(self
            .into_iter()
            .map(|w| 
                PlacedWord::new(f(w.value), w.position, w.direction)
            )).unwrap();
    
        res
    }
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> IntoIterator for Crossword<CharT, StrT>
{
    type Item = PlacedWord<CharT, StrT>;
    type IntoIter = <BTreeSet<PlacedWord<CharT, StrT>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.words.into_iter()
    }
}


#[cfg(test)]
mod tests {
    

    use super::*;

    #[test]
    fn test_crossword_contains_crossword() {
        let mut cw = Crossword::new(
            WordCompatibilitySettings
            {
                side_by_side: true,
                ..Default::default()
            }
        );   
        cw.add_word(PlacedWord::<u8, &str>::new( "hello", Position { x: 0, y: 0 }, Direction::Right)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "local", Position { x: 2, y: 0 }, Direction::Down)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "cat", Position { x: 2, y: 2 }, Direction::Right)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "and", Position { x: 3, y: 2 }, Direction::Down)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "toy", Position { x: 4, y: 2 }, Direction::Down)).unwrap();

        
        let mut cw1 = Crossword::new(
            WordCompatibilitySettings
            {
                side_by_side: true,
                ..Default::default()
            }
        );
        cw1.add_word(PlacedWord::<u8, &str>::new( "hello", Position { x: 0, y: 0 }, Direction::Right)).unwrap();
        cw1.add_word(PlacedWord::<u8, &str>::new( "local", Position { x: 2, y: 0 }, Direction::Down)).unwrap();
        cw1.add_word(PlacedWord::<u8, &str>::new( "cat", Position { x: 2, y: 2 }, Direction::Right)).unwrap();
        cw1.add_word(PlacedWord::<u8, &str>::new( "and", Position { x: 3, y: 2 }, Direction::Down)).unwrap();
        cw1.add_word(PlacedWord::<u8, &str>::new( "toy", Position { x: 4, y: 2 }, Direction::Down)).unwrap();

        let mut cw2 = Crossword::new(
            WordCompatibilitySettings
            {
                side_by_side: true,
                ..Default::default()
            }
        );   
        cw2.add_word(PlacedWord::<u8, &str>::new( "cat", Position { x: 0, y: 0 }, Direction::Right)).unwrap();
        cw2.add_word(PlacedWord::<u8, &str>::new( "and", Position { x: 1, y: 0 }, Direction::Down)).unwrap();
        cw2.add_word(PlacedWord::<u8, &str>::new( "toy", Position { x: 2, y: 0 }, Direction::Down)).unwrap();

        let mut cw3 = Crossword::new(
            WordCompatibilitySettings
            {
                side_by_side: true,
                ..Default::default()
            }
        );   
        cw3.add_word(PlacedWord::<u8, &str>::new( "and", Position { x: 0, y: 0 }, Direction::Down)).unwrap();
        cw3.add_word(PlacedWord::<u8, &str>::new( "toy", Position { x: 1, y: -1 }, Direction::Down)).unwrap();

        assert_eq!([cw.contains_crossword(&cw1), cw.contains_crossword(&cw2), cw.contains_crossword(&cw3)], [true, true, false]);
    }

    #[test]
    fn test_crossword_remove_word() {
        let mut cw = Crossword::new(
            WordCompatibilitySettings
            {
                side_by_side: true,
                ..Default::default()
            }
        );   
        cw.add_word(PlacedWord::<u8, &str>::new( "hello", Position { x: 0, y: 0 }, Direction::Right)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "local", Position { x: 2, y: 0 }, Direction::Down)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "cat", Position { x: 2, y: 2 }, Direction::Right)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "and", Position { x: 3, y: 2 }, Direction::Down)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "toy", Position { x: 4, y: 2 }, Direction::Down)).unwrap();
        
        cw.remove_word(&"toy");

        let mut cw_rm = Crossword::new(
            WordCompatibilitySettings
            {
                side_by_side: true,
                ..Default::default()
            }
        );   
        cw_rm.add_word(PlacedWord::<u8, &str>::new( "hello", Position { x: 0, y: 0 }, Direction::Right)).unwrap();
        cw_rm.add_word(PlacedWord::<u8, &str>::new( "local", Position { x: 2, y: 0 }, Direction::Down)).unwrap();
        cw_rm.add_word(PlacedWord::<u8, &str>::new( "cat", Position { x: 2, y: 2 }, Direction::Right)).unwrap();
        cw_rm.add_word(PlacedWord::<u8, &str>::new( "and", Position { x: 3, y: 2 }, Direction::Down)).unwrap();

        assert_eq!(cw, cw_rm);
    }

    #[test]
    fn test_crossword_calculate_possible_ways_to_add_word() {
        let mut cw = Crossword::new(
            WordCompatibilitySettings
            {
                side_by_side: true, // |-
                side_by_head: true, // ||
                ..Default::default()
            }
        );   
        cw.add_word(PlacedWord::<u8, &str>::new( "hello", Position { x: 0, y: 0 }, Direction::Right)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "local", Position { x: 2, y: 0 }, Direction::Down)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "tac", Position { x: 0, y: 2 }, Direction::Right)).unwrap();

        let new_word = Word::new("hatlo", None);

        assert_eq!(cw.calculate_possible_ways_to_add_word(&new_word), vec![
            PlacedWord::new(new_word.value, Position { x: 0, y: 0 }, Direction::Down),
            PlacedWord::new(new_word.value, Position { x: 1, y: 1 }, Direction::Down),   //|-
            PlacedWord::new(new_word.value, Position { x: 1, y: 3 }, Direction::Right),  //||
            PlacedWord::new(new_word.value, Position { x: 3, y: -3 }, Direction::Down),  //||
            PlacedWord::new(new_word.value, Position { x: -1, y: 4 }, Direction::Right),
            PlacedWord::new(new_word.value, Position { x: -2, y: 1 }, Direction::Right), //||
            PlacedWord::new(new_word.value, Position { x: 4, y: -4 }, Direction::Down),
            ].into_iter().collect());
    }



}
