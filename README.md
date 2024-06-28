# Crossword_Generator

Crossword_generator is a library for creating crosswords from provided words. It determines the positions and directions of the words, but does not generate a finished blank crossword puzzle to solve. 

Works in an async runtime.

```rust
use crossword_generator::{crossword::Crossword, generator::{CrosswordGenerationRequest, CrosswordGenerator, CrosswordGeneratorSettings}, word::Word};
use tokio_stream::StreamExt;

// A quick function to print the crossword to the console
fn print_crossword(cw: &Crossword<u8, String>)
{
    let table = cw.generate_char_table();
    println!(" {} ", vec!['-'; table[0].len() * 2 - 1].into_iter().collect::<String>());
    for i in 0..table.len()
    {
        print!("|");
        for j in 0..table[0].len()
        {
            print!("{}", (if table[i][j] != 0 {table[i][j]} else {32}) as char);
            if j != table[0].len() - 1 { print!(" "); } 
        }
        println!("|");
    }
    println!(" {} ", vec!['-'; table[0].len() * 2 - 1].into_iter().collect::<String>());
}

#[tokio::main]
async fn main()
{
    // Create a generator.
    let mut generator = CrosswordGenerator::<u8, String>::default();

    // Set some settings.
    generator.settings = CrosswordGeneratorSettings::default();

    // Specify the words crosswords will be consisted from.
    generator.words = vec!["hello", "world", "foo", "raw"].into_iter().map(|s| Word::new(s.to_lowercase(), None)).collect();
    
    // Create the crossword stream, this will generate crosswords and return them to you. If you wait long enough, you will get every possible crossword that satisfies the settings.
    let mut str = generator.crossword_stream(|s| String::from_utf8(s.to_owned()).expect("The word is not in proper utf8 format"));

    // You can request a concrete number of crosswords, or all of them.
    str.request_crossword(CrosswordGenerationRequest::All).await;
    while let Some(cw) = str.next().await 
    {
        print_crossword(&cw);
        println!("");
    }
}
```