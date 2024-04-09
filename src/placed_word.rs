use std::{collections::BTreeSet, marker::PhantomData};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::{utils::{CrosswordChar, CrosswordString}, word::{Direction, Position, Word}};



#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
struct WordBoundingBox
{
    x: i16,
    y: i16,
    w: u16, 
    h: u16
}

impl WordBoundingBox
{
    fn intersects(&self, other: &WordBoundingBox) -> bool 
    {
        (self.x < other.x + other.w as i16 && self.x + self.w as i16 > other.x) &&
        (self.y < other.y + other.h as i16 && self.y + self.h as i16 > other.y)
    }

    fn sides_touch(&self, other: &WordBoundingBox) -> bool
    {
        ((self.x + self.w as i16 > other.x && self.x < other.x + other.w as i16) && (self.y + self.h as i16 == other.y || other.y + other.h as i16 == self.y)) || 
        ((self.y + self.h as i16 > other.y && self.y < other.y + other.h as i16) && (self.x + self.w as i16 == other.x || other.x + other.w as i16 == self.x))
    }

    fn corners_touch(&self, other: &WordBoundingBox) -> bool
    {
        (self.x == other.x + other.w as i16 && self.y == other.y + other.h as i16) ||
        (self.x + self.w as i16 == other.x && self.y == other.y + other.h as i16) ||
        (self.x + self.w as i16 == other.x && self.y + self.h as i16 == other.y) ||
        (self.x == other.x + other.w as i16 && self.y + self.h as i16 == other.y)
    }

}


#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct PlacedWord<CharT: CrosswordChar, StrT: CrosswordString<CharT>>
{
    pub position: Position,
    pub direction: Direction,
    pub value: StrT,
    #[serde(skip)]
    character_type: PhantomData<CharT>
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> PlacedWord<CharT, StrT>
{
    pub fn new(val: StrT, pos: Position, dir: Direction) -> PlacedWord<CharT, StrT>
    {
        PlacedWord { value: val, position: pos, direction: dir, character_type: PhantomData }
    } 
    fn value(&self) -> &[CharT]
    {
        self.value.as_ref()
    }
    fn get_bounding_box(&self) -> WordBoundingBox
    {
        match self.direction 
        {
            Direction::Right => WordBoundingBox { x: self.position.x, y: self.position.y, w: self.value().len() as u16, h: 1 },
            Direction::Down => WordBoundingBox { x: self.position.x, y: self.position.y, w: 1, h: self.value().len() as u16 },
        }
    }

    fn get_parallel_coordinate(&self) -> i16
    {
        match self.direction
        {
            Direction::Right => self.position.y,
            Direction::Down => self.position.x,
        }
    }

    #[allow(dead_code)]
    fn get_perpendicular_coordinate(&self) -> i16
    {
        match self.direction
        {
            Direction::Right => self.position.x,
            Direction::Down => self.position.y,
        }
    }

    /// Returns true if two [words](Word) are intersecting 
    pub fn intersects(&self, other: &PlacedWord<CharT, StrT>) -> bool 
    {
        self.get_bounding_box().intersects(&other.get_bounding_box())
    }

    fn sides_touch(&self, other: &PlacedWord<CharT, StrT>) -> bool
    {
        self.get_bounding_box().sides_touch(&other.get_bounding_box())
    }

    /// Returns true if two [words](Word) are corner by corner (check [WordCompatibilitySettings::corner_by_corner])
    pub fn corners_touch(&self, other: &PlacedWord<CharT, StrT>) -> bool
    {
        self.get_bounding_box().corners_touch(&other.get_bounding_box())
    }

    /// Returns true if two [words](Word) are side by side (check [WordCompatibilitySettings::side_by_side])
    pub fn side_touches_side(&self, other: &PlacedWord<CharT, StrT>) -> bool
    {
        self.direction == other.direction &&
        self.sides_touch(other) && 
        self.get_parallel_coordinate() != other.get_parallel_coordinate()
    }

    /// Returns true if two [words](Word) are side by head (check [WordCompatibilitySettings::side_by_head])
    pub fn side_touches_head(&self, other: &PlacedWord<CharT, StrT>) -> bool
    {
        self.direction != other.direction &&
        self.sides_touch(other)
    }

    /// Returns true if two [words](Word) are head by head (check [WordCompatibilitySettings::head_by_head])
    pub fn head_touches_head(&self, other: &PlacedWord<CharT, StrT>) -> bool
    {
        self.direction == other.direction &&
        self.sides_touch(other) && 
        self.get_parallel_coordinate() == other.get_parallel_coordinate()
    }

    /// Returns the indices of the characters in the intersection of the [words](Word) if they are intersecting
    /// 
    /// Returns None otherwise
    /// 
    /// ## Examples
    /// ```
    /// # use crossword_generator::word::{Word, Position, Direction};
    /// let w1 = Word{ position: Position{x: 0, y: 1}, direction: Direction::Right, value: "hello"};
    /// let w2 = Word{ position: Position{x: 4, y: 0}, direction: Direction::Down, value: "world"};
    /// 
    /// //         w
    /// // h e l l o
    /// //         r
    /// //         l
    /// //         d
    /// 
    /// assert_eq!(w1.get_intersection_indices(&w2), Some((4, 1)));
    /// ```
    /// 
    /// Note that this function does not care if the characters on the intersection are not the same, so if the words are dog and cat, 
    /// function can return non None result even though the words dog and cat don't have a common letter.
    pub fn get_intersection_indices(&self, other: &PlacedWord<CharT, StrT>) -> Option<(u16, u16)>
    {
        if !self.intersects(other) { return None; }
        if self.direction == other.direction { return None; }

        match self.direction
        {
            Direction::Right => Some(((other.position.x - self.position.x) as u16, (self.position.y - other.position.y) as u16)),
            Direction::Down => Some(((other.position.y - self.position.y) as u16, (self.position.x - other.position.x) as u16))
        }
    }

    /// Returns all possible ways to add another [word](Word) on top of this 
    /// 
    /// ## Examples
    /// ```
    /// # use crossword_generator::word::{Word, Position, Direction};
    /// # use std::collections::BTreeSet;
    /// let w1 = Word{ position: Position{x: 0, y: 3}, direction: Direction::Right, value: "hello"};
    /// 
    /// 
    /// //     w w 
    /// //     o o 
    /// //     r r w
    /// // h e l l o ---> 3 ways
    /// //     d d r
    /// //         l
    /// //         d
    /// 
    /// assert_eq!(w1.calculate_possible_ways_to_add_word("world"), BTreeSet::from([
    ///     Word{ position: Position{x: 2, y: 0}, direction: Direction::Down, value: "world"},
    ///     Word{ position: Position{x: 3, y: 0}, direction: Direction::Down, value: "world"},
    ///     Word{ position: Position{x: 4, y: 2}, direction: Direction::Down, value: "world"}
    /// ]));
    ///
    /// ```
    pub fn calculate_possible_ways_to_add_word(&self, word: &Word<CharT, StrT>) -> BTreeSet<PlacedWord<CharT, StrT>>
    {
        if let Some(dir) = &word.dir
        {
            if *dir == self.direction { return BTreeSet::default(); }
        }
        let w = word.value.as_ref();
        let mut pos_ways: BTreeSet<PlacedWord<CharT, StrT>> = BTreeSet::new();
        let common_chars = w.iter().filter(|c| self.value.as_ref().contains(*c)).collect::<Vec<&CharT>>();

        for char in common_chars
        {
            for (word_ind, self_ind) in w.iter().enumerate().filter_map(|c| if c.1 == char { Some(c.0) } else { None } ).cartesian_product(self.value.as_ref().iter().enumerate().filter_map(|c| if c.1 == char { Some(c.0) } else { None } ))
            {
                pos_ways.insert(PlacedWord::<CharT, StrT>::new(
                    
                    word.value.clone(),
                    match self.direction
                    {
                        Direction::Right => Position{ x: self.position.x + self_ind as i16, y: self.position.y - word_ind as i16},
                        Direction::Down  => Position{ x: self.position.x - word_ind as i16, y: self.position.y + self_ind as i16},
                    },
                    self.direction.opposite(),
                ));
            }
        }

        pos_ways
    }
}





#[cfg(test)]
mod tests
{
    use itertools::iproduct;
    
    use crate::crossword::WordCompatibilitySettings;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    
    #[test]
    fn test_placed_word_intersects()
    {
        let mut first = PlacedWord::new("hayastan", Position{ x: 0, y: 0 }, Direction::Right);
        let mut second = PlacedWord::new("arcax", Position{ x: 0, y: 0 }, Direction::Right);
        
        let mut comp = vec![];
        for y in -2i16..=2
        {
            for x in -6i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.intersects(&second) as isize);
            }
        }
    
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_hor");
                                
        first.direction = Direction::Down;
        second.direction = Direction::Down;
        comp = vec![];
        for y in -6i16..=9
        {
            for x in -2i16..=2
            {
                second.position = Position {x, y};
                comp.push(first.intersects(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0], "ver_ver");
        
        first.direction = Direction::Right;
        comp = vec![];
        for y in -6i16..=2
        {
            for x in -2i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.intersects(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_ver");
    }
    
    #[test]
    fn test_placed_word_side_touches_side()
    {
        let mut first = PlacedWord::new("hayastan", Position{ x: 0, y: 0 }, Direction::Right);
        let mut second = PlacedWord::new("arcax", Position{ x: 0, y: 0 }, Direction::Right);
        
        let mut comp = vec![];
        for y in -2i16..=2
        {
            for x in -6i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.side_touches_side(&second) as isize);
            }
        }
    
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_hor");
                                
        first.direction = Direction::Down;
        second.direction = Direction::Down;
        comp = vec![];
        for y in -6i16..=9
        {
            for x in -2i16..=2
            {
                second.position = Position {x, y};
                comp.push(first.side_touches_side(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 1, 0, 1, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0], "ver_ver");
        
        first.direction = Direction::Right;
        comp = vec![];
        for y in -6i16..=2
        {
            for x in -2i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.side_touches_side(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_ver");
    }
        
    #[test]
    fn test_placed_word_side_touches_head()
    {
        let mut first = PlacedWord::new("hayastan", Position{ x: 0, y: 0 }, Direction::Right);
        let mut second = PlacedWord::new("arcax", Position{ x: 0, y: 0 }, Direction::Right);
        
        let mut comp = vec![];
        for y in -2i16..=2
        {
            for x in -6i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.side_touches_head(&second) as isize);
            }
        }
    
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_hor");
                                
        first.direction = Direction::Down;
        second.direction = Direction::Down;
        comp = vec![];
        for y in -6i16..=9
        {
            for x in -2i16..=2
            {
                second.position = Position {x, y};
                comp.push(first.side_touches_head(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0], "ver_ver");
        
        first.direction = Direction::Right;
        comp = vec![];
        for y in -6i16..=2
        {
            for x in -2i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.side_touches_head(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_ver");
    }
    
    #[test]
    fn test_placed_word_head_touches_head()
    {
        let mut first = PlacedWord::new("hayastan", Position{ x: 0, y: 0 }, Direction::Right);
        let mut second = PlacedWord::new("arcax", Position{ x: 0, y: 0 }, Direction::Right);
        
        let mut comp = vec![];
        for y in -2i16..=2
        {
            for x in -6i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.head_touches_head(&second) as isize);
            }
        }
    
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_hor");
                                
        first.direction = Direction::Down;
        second.direction = Direction::Down;
        comp = vec![];
        for y in -6i16..=9
        {
            for x in -2i16..=2
            {
                second.position = Position {x, y};
                comp.push(first.head_touches_head(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 1, 0, 0,
                                0, 0, 0, 0, 0], "ver_ver");
        
        first.direction = Direction::Right;
        comp = vec![];
        for y in -6i16..=2
        {
            for x in -2i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.head_touches_head(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_ver");
    }
    
    #[test]
    fn test_placed_word_corners()
    {
        let mut first = PlacedWord::new("hayastan", Position{ x: 0, y: 0 }, Direction::Right);
        let mut second = PlacedWord::new("arcax", Position{ x: 0, y: 0 }, Direction::Right);
        
        let mut comp = vec![];
        for y in -2i16..=2
        {
            for x in -6i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.corners_touch(&second) as isize);
            }
        }
    
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_hor");
                                
        first.direction = Direction::Down;
        second.direction = Direction::Down;
        comp = vec![];
        for y in -6i16..=9
        {
            for x in -2i16..=2
            {
                second.position = Position {x, y};
                comp.push(first.corners_touch(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0,
                                0, 1, 0, 1, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0,
                                0, 1, 0, 1, 0,
                                0, 0, 0, 0, 0], "ver_ver");
        
        first.direction = Direction::Right;
        comp = vec![];
        for y in -6i16..=2
        {
            for x in -2i16..=9
            {
                second.position = Position {x, y};
                comp.push(first.corners_touch(&second) as isize);
            }
        }
        
        assert_eq!(comp, vec![  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "hor_ver");
    }

    #[test]
    fn test_placed_word_get_intersection_indices()
    {
        let mut first = PlacedWord::new("hayastan", Position{ x: 0, y: 0 }, Direction::Right);
        let mut second = PlacedWord::new("arcax", Position{ x: 0, y: 0 }, Direction::Right);

        assert_eq!(first.get_intersection_indices(&second), None);

        first.direction = Direction::Down;
        assert_eq!(first.get_intersection_indices(&second), Some((0, 0)));
        
        second.position = Position {x: -1, y: 2};
        assert_eq!(first.get_intersection_indices(&second), Some((2, 1)));

        second.position.x = 2;
        assert_eq!(first.get_intersection_indices(&second), None);
    }

    
    
    #[test]
    fn test_word_compatibility_settings_are_words_compatible() {

        for (a, b, c, d) in iproduct!((0isize..2), (0isize..2), (0isize..2), (0isize..2))
        {
            let settings = WordCompatibilitySettings { side_by_side: a != 0, head_by_head: b != 0, side_by_head: c != 0, corner_by_corner: d != 0 };

            let mut first = PlacedWord::new("hayastan", Position{ x: 0, y: 0 }, Direction::Right);
            let mut second = PlacedWord::new("arcax", Position{ x: 0, y: 0 }, Direction::Right);
            
            let mut comp = vec![];
            for y in -2i16..=2
            {
                for x in -6i16..=9
                {
                    second.position = Position {x, y};
                    comp.push(settings.are_words_compatible(&first, &second) as isize);
                }
            }
        
            assert_eq!(comp, vec![  1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                                    1, d, a, a, a, a, a, a, a, a, a, a, a, a, d, 1,
                                    1, b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b, 1,
                                    1, d, a, a, a, a, a, a, a, a, a, a, a, a, d, 1,
                                    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], "hor_hor with settings {:?}", settings);
                
            first.direction = Direction::Down;
            second.direction = Direction::Down;
            comp = vec![];
            for y in -6i16..=9
            {
                for x in -2i16..=2
                {
                    second.position = Position {x, y};
                    comp.push(settings.are_words_compatible(&first, &second) as isize);
                }
            }
            
            assert_eq!(comp, vec![  1, 1, 1, 1, 1,
                                    1, d, b, d, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, a, 0, a, 1,
                                    1, d, b, d, 1,
                                    1, 1, 1, 1, 1], "ver_ver with settings {:?}", settings);

            first.direction = Direction::Right;
            comp = vec![];
            for y in -6i16..=2
            {
                for x in -2i16..=9
                {
                    second.position = Position {x, y};
                    comp.push(settings.are_words_compatible(&first, &second) as isize);
                }
            }
            
            assert_eq!(comp, vec![  1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                                    1, d, c, c, c, c, c, c, c, c, d, 1,
                                    1, c, 0, 0, 0, 0, 0, 0, 0, 0, c, 1,
                                    1, c, 0, 1, 0, 1, 0, 0, 1, 0, c, 1,
                                    1, c, 0, 0, 0, 0, 0, 0, 0, 0, c, 1,
                                    1, c, 0, 0, 0, 0, 0, 0, 0, 0, c, 1,
                                    1, c, 0, 1, 0, 1, 0, 0, 1, 0, c, 1,
                                    1, d, c, c, c, c, c, c, c, c, d, 1,
                                    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], "hor_ver with settings {:?}", settings);
}
        
    }
                            
}
