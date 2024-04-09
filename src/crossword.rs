use std::{collections::BTreeSet, default};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::{placed_word::PlacedWord, utils::{CrosswordChar, CrosswordString}, word::{Direction, Position, Word}};

#[derive(Error, Debug)]
pub enum CrosswordError
{
    #[error("Cannot add the word to the crossword.")]
    CantAddWord,
    #[error("The word is already in the crossword.")]
    WordAlreadyExists
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum CrosswordConstraint
{
    None
}

impl CrosswordConstraint
{
    fn check<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, crossword: &Crossword<CharT, StrT>) -> bool
    {
        match self
        {
            &CrosswordConstraint::None => true
        }
    }
    fn recoverable(&self) -> bool
    {
        match self
        {
            &CrosswordConstraint::None => false
        }
    }
}
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct CrosswordSettings
{
    pub constraints: Vec<CrosswordConstraint>
}

impl CrosswordSettings
{
    pub fn check_recoverables<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, crossword: &Crossword<CharT, StrT>) -> bool
    {
        self.constraints.iter().filter(|constr| constr.recoverable()).all(|constr| constr.check(crossword))
    }

    pub fn check_nonrecoverables<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, crossword: &Crossword<CharT, StrT>) -> bool
    {
        self.constraints.iter().filter(|constr| !constr.recoverable()).all(|constr| constr.check(crossword))
    }
}

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
    /// Checks if two [words](Word) are compatible
    pub fn are_words_compatible<CharT: CrosswordChar, StrT: CrosswordString<CharT>>(&self, first: &PlacedWord<CharT, StrT>, second: &PlacedWord<CharT, StrT>) -> bool
    {
        if first.corners_touch(&second) && !self.corner_by_corner { return false; }

        if first.direction == second.direction
        {
            if first.head_touches_head(&second) && !self.head_by_head { return false; }
            if first.side_touches_side(&second) && !self.side_by_side { return false; }
            if first.intersects(&second) { return false; }

            true
        }
        else
        {
            if first.side_touches_head(&second) && !self.side_by_head { return false; }
            if first.intersects(&second)
            {
                let (first_ind, second_ind) = first.get_intersection_indices(&second).unwrap();
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
        return WordCompatibilitySettings 
        {
            side_by_side: false,
            head_by_head: false,
            side_by_head: false,
            corner_by_corner: true
        }    
    }
}
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

    pub fn new(word_compatibility_settings: WordCompatibilitySettings) -> Crossword<CharT, StrT>
    {
        Crossword{ word_compatibility_settings: word_compatibility_settings, ..Default::default() }
    }

    pub fn can_word_be_added(&self, word: &PlacedWord<CharT, StrT>) -> bool
    {
        self.words.iter().all(|w| self.word_compatibility_settings.are_words_compatible(w, word))
    }

    pub fn find_word(&self, word: &StrT) -> Option<&PlacedWord<CharT, StrT>>
    {
        self.words.iter().filter(|w| w.value == *word).next()
    }

    pub fn add_word(&mut self, word: PlacedWord<CharT, StrT>) -> Result<(), CrosswordError>
    {
        if let Some(_) = self.find_word(&word.value) { Err(CrosswordError::WordAlreadyExists) }
        else if !self.can_word_be_added(&word) { Err(CrosswordError::CantAddWord) }
        else { self.words.insert(word); self.normalize(); Ok(()) } 
    }  

    pub fn remove_word(&mut self, word: &StrT) -> bool
    {
        if let Some(word) = self.find_word(word).and_then(|w| Some(w.clone()))
        {
            self.words.remove(&word);

            self.normalize();

            true
        }
        else { false }
    }

    pub fn contains_crossword(&self, other: &Crossword<CharT, StrT>) -> bool 
    {
        if other.words.len() > self.words.len() { return false; }
        let mut offset: Option<(i16, i16)> = None;
        for other_word in other.words.iter()
        {
            let cur_word = self.find_word(&other_word.value);
            if let None = cur_word
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

    pub fn iter(&self) -> impl Iterator<Item = &PlacedWord<CharT, StrT>>
    {
        self.words.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = PlacedWord<CharT, StrT>>
    {
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
                side_by_side: true,
                side_by_head: true,
                ..Default::default()
            }
        );   
        cw.add_word(PlacedWord::<u8, &str>::new( "hello", Position { x: 0, y: 0 }, Direction::Right)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "local", Position { x: 2, y: 0 }, Direction::Down)).unwrap();
        cw.add_word(PlacedWord::<u8, &str>::new( "tac", Position { x: 0, y: 2 }, Direction::Right)).unwrap();

        let new_word = Word::new("hatlo", None, None);

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
