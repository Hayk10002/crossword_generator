use std::marker::PhantomData;
use serde::{Deserialize, Serialize};
use crate::utils::{CrosswordChar, CrosswordString};

/// Represents the position of the first character of a [word](crate::placed_word::PlacedWord) placed in [crossword](crate::crossword::Crossword)
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct Position
{
    pub x: i16,
    pub y: i16,
}

/// Represents the direction of a [word](crate::placed_word::PlacedWord) placed in [crossword](crate::crossword::Crossword)
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub enum Direction
{
    #[default]
    Right,
    Down,
}

impl Direction
{
    pub fn opposite(&self) -> Direction
    {
        match *self
        {
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Right,
        }
    }
}

/// Represents a word outside of a [crossword](crate::crossword::Crossword), has no particular [position](Position), but can have a specified [direction](Direction) that when generating crosswords, the word will be only in the specified direction
/// 
/// Accepts two template parameters, that specify the type of individual characters in the word and the type of the word itself (for example u8 and &str, or if you want your crossword to consist of numbers, Digit and Vec\<Digit\> (where Digit is a type that accepts only numbers from 0 to 9))  
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct Word<CharT: CrosswordChar, StrT: CrosswordString<CharT>>
{
    pub value: StrT,
    pub dir: Option<Direction>,
    #[serde(skip)]
    character_type: PhantomData<CharT>
} 

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> Word<CharT, StrT>
{
    // you can specify a constraint on direction with Some(direction)
    pub fn new(val: StrT, dir: Option<Direction>) -> Word<CharT, StrT>
    {
        Word { value: val, dir, character_type: PhantomData }
    } 
}