use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::utils::{CrosswordChar, CrosswordString};

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct Position
{
    pub x: i16,
    pub y: i16,
}

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
        match self
        {
            &Direction::Right => Direction::Down,
            &Direction::Down => Direction::Right,
        }
    }

    pub fn get_dir_num_values(&self) -> (u16, u16)
    {
        match self
        {
            &Direction::Right => (1, 0),
            &Direction::Down => (0, 1),
        }
    }
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct Word<CharT: CrosswordChar, StrT: CrosswordString<CharT>>
{
    pub value: StrT,
    pub dir: Option<Direction>,
    character_type: PhantomData<CharT>
} 

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> Word<CharT, StrT>
{
    pub fn new(val: StrT, pos: Option<Position>, dir: Option<Direction>) -> Word<CharT, StrT>
    {
        Word { value: val, dir: dir, character_type: PhantomData }
    } 
}




fn test()
{
    let x = Word::<u8, String>::new("Helloworld".to_owned(), None, None);
}