pub mod utils;
pub mod word;
pub mod placed_word;
pub mod crossword;
pub mod generator;


pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use tokio_stream::StreamExt;

    use self::{generator::{CrosswordGenerationRequest, CrosswordGenerator, CrosswordGeneratorSettings}, word::Word};

    use super::*;

    #[tokio::test]
    async fn it_works() {
        let gen = CrosswordGenerator::<u8, Vec<u8>>
        {
            words: ["a",
                    "accb",
                    "b"].into_iter().map(|s| Word::<u8, Vec<u8>>::new(<String as AsRef<[u8]>>::as_ref(&s.to_lowercase()).to_owned(), None)).collect(),
            settings: CrosswordGeneratorSettings::default()
        };

        let mut str = gen.crossword_stream();
        str.request_crossword(CrosswordGenerationRequest::Count(10)).await;
        str.request_crossword(CrosswordGenerationRequest::Stop).await;


        let mut crosswords = vec![];
        while let Some(cw) = str.next().await
        {
            crosswords.push(cw.convert_to::<String>(|w| String::from_utf8(w).unwrap()));
        }
        
        println!("{}", serde_json::to_string_pretty(&crosswords).unwrap());
    }
}
