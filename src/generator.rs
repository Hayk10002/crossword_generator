use std::{collections::BTreeSet, future::Future, pin::Pin, task::{Context, Poll}};

use async_recursion::async_recursion;
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc::{self, Receiver, Sender}, task};
use tokio_stream::Stream;

use crate::{crossword::{Crossword, CrosswordSettings, WordCompatibilitySettings}, traits::{CrosswordChar, CrosswordString}, word::Word};

/// Represents all settings for a [generator](CrosswordGenerator).
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct CrosswordGeneratorSettings
{
    pub crossword_settings: CrosswordSettings,
    pub word_compatibility_settings: WordCompatibilitySettings
}

/// Represents a crossword generator, runs in an async runtime.
/// 
/// # Example
/// ```
/// use crossword_generator::generator::{CrosswordGenerator, CrosswordGeneratorSettings, CrosswordGenerationRequest};
/// use crossword_generator::crossword::Crossword;
/// use crossword_generator::placed_word::PlacedWord;
/// use crossword_generator::word::{Direction, Position, Word};
/// 
/// use tokio_stream::StreamExt;
/// 
/// #[tokio::main]
/// async fn main() 
/// {
/// 
///     let mut generator = CrosswordGenerator::<u8, Vec<u8>>::default();
///     generator.settings = CrosswordGeneratorSettings::default();
///     generator.words = vec!["Hello", "world"].into_iter().map(|s| Word::new(<String as AsRef<[u8]>>::as_ref(&s.to_lowercase()).to_owned(), None)).collect();
///      
///     let str = generator.crossword_stream();
///     str.request_crossword(CrosswordGenerationRequest::Count(2)).await;
///     str.request_crossword(CrosswordGenerationRequest::Stop).await;
///     let crosswords: Vec<Crossword<u8, String>> = str.map(|cw| cw.convert_to(|w| String::from_utf8(w).unwrap())).collect().await;
///     
///     let mut cw1 = Crossword::default();
///     let mut cw2 = Crossword::default();
/// 
///     cw1.add_words([PlacedWord::new("hello".to_owned(), Position{ x: 0, y: 3 }, Direction::Right),
///                    PlacedWord::new("world".to_owned(), Position{ x: 2, y: 0 }, Direction::Down)].into_iter()).unwrap();
///     
///     cw2.add_words([PlacedWord::new("hello".to_owned(), Position{ x: 0, y: 3 }, Direction::Right),
///                    PlacedWord::new("world".to_owned(), Position{ x: 3, y: 0 }, Direction::Down)].into_iter()).unwrap();
/// 
///     assert_eq!(crosswords, vec![cw1, cw2])
/// }
/// ```
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct CrosswordGenerator<CharT: CrosswordChar, StrT: CrosswordString<CharT>>
{
    pub words: BTreeSet<Word<CharT, StrT>>,
    pub settings: CrosswordGeneratorSettings,
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT> + FromIterator<CharT>> CrosswordGenerator<CharT, StrT>
{
    pub fn crossword_stream(&self) -> CrosswordStream<CharT, StrT>
    {  
        let gen = self.clone();
        
        let gen_func = move |mut rr: Receiver<CrosswordGenerationRequest>, cs: Sender<Crossword<CharT, StrT>>| async move
        {

            let mut current_request = CrosswordGenerationRequest::Count(0);
            let mut current_crossword = Crossword::new(gen.settings.word_compatibility_settings.clone());
            let mut full_created_crossword_bases = BTreeSet::new();
            let remaine_words = gen.words.iter().map(|w| Word::<CharT, &[CharT]>::new(w.value.as_ref(), w.dir.clone())).collect();
            CrosswordGenerator::<CharT, StrT>::generator_impl(&gen.settings, &mut rr, &cs, &mut current_request, &mut current_crossword, &remaine_words, &mut full_created_crossword_bases).await
               
        };

        CrosswordStream::new(gen_func)
    }

    #[async_recursion]
    async fn generator_impl<'a>(gen_settings: &CrosswordGeneratorSettings, rr: &mut Receiver<CrosswordGenerationRequest>, cs: &Sender<Crossword<CharT, StrT>>, current_request: &mut CrosswordGenerationRequest, current_crossword: &mut Crossword<CharT, &'a [CharT]>, remained_words: &BTreeSet<Word<CharT, &'a [CharT]>>, full_created_crossword_bases: &mut BTreeSet<Crossword<CharT, &'a [CharT]>>)  
    {
        if !gen_settings.crossword_settings.check_nonrecoverables_constraints(current_crossword) 
        {
            return; 
        }

        if full_created_crossword_bases.iter().any(|cw| current_crossword.contains_crossword(cw))
        {
            return;
        }
        
        if remained_words.is_empty()
        {
            if gen_settings.crossword_settings.check_recoverable_constraints(current_crossword) 
            {
                while let CrosswordGenerationRequest::Count(0) = current_request
                {
                    match rr.recv().await
                    {
                        None | Some(CrosswordGenerationRequest::Stop) => { *current_request = CrosswordGenerationRequest::Stop; return },
                        Some(req) => *current_request = req
                    }
                }

                cs.send(current_crossword.clone().convert_to(|w| w.iter().cloned().collect())).await.unwrap();
                if let CrosswordGenerationRequest::Count(count) = *current_request { *current_request = CrosswordGenerationRequest::Count(count - 1) }
            }
            return;
        }
        for current_word in remained_words.iter()
        {
            let mut new_remained_words = remained_words.clone();
            new_remained_words.remove(current_word);
            for step in current_crossword.calculate_possible_ways_to_add_word(current_word).iter()
            {
                current_crossword.add_word(step.clone()).unwrap();

                CrosswordGenerator::generator_impl(gen_settings, rr, cs, current_request, current_crossword, &new_remained_words, full_created_crossword_bases).await;

                if let CrosswordGenerationRequest::Stop = current_request { return; }
                
                let to_remove: Vec<Crossword<CharT, &[CharT]>> = full_created_crossword_bases.iter().filter_map(|cw| cw.contains_crossword(current_crossword).then_some(cw.clone())).collect();
                to_remove.into_iter().for_each(|cw| {full_created_crossword_bases.remove(&cw);});
                
                full_created_crossword_bases.insert(current_crossword.clone());

                current_crossword.remove_word(&step.value);
            }
        }

 
    }

}


/// Represents a request to [CrosswordStream] for generating crosswords.
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub enum CrosswordGenerationRequest
{
    /// Request to stop the crossword generation.
    #[default]
    Stop,
    /// Request for some count of crosswords to generate.
    Count(u32),
    /// Request for generating all possible crosswords.
    All
}

pub struct CrosswordStream<CharT: CrosswordChar + 'static, StrT: CrosswordString<CharT> + 'static>
{
    request_sender: Sender<CrosswordGenerationRequest>,
    crossword_reciever: Receiver<Crossword<CharT, StrT>>
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> CrosswordStream<CharT, StrT>
{
    
    pub fn new<F,Fut>(gen_func: F) -> CrosswordStream<CharT, StrT>
    where
        F: FnOnce(Receiver<CrosswordGenerationRequest>, Sender<Crossword<CharT, StrT>>) -> Fut,
        Fut: Future<Output=()> + Send + 'static
    {
        let (rs, rr) = mpsc::channel(100);
        let (cs, cr) = mpsc::channel(100);

        task::spawn(gen_func(rr, cs));
        
        CrosswordStream { request_sender: rs, crossword_reciever: cr }
    }

    /// Requests crosswords to generate with function like next or take.
    /// 
    /// After requesting some count of crosswords (with [CrosswordGenerationRequest::Count]) and generating the crosswords the stream will start to wait for other requests, so if you want to only generate for example 10 crosswords, you need to request that, and then request a [CrosswordGenerationRequest::Stop] to stop the generator.
    pub async fn request_crossword(&self, req: CrosswordGenerationRequest)
    {
        self.request_sender.send(req).await.unwrap();
    }
}  

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> Stream for CrosswordStream<CharT, StrT>
{
    type Item = Crossword<CharT, StrT>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>
    {
        self.crossword_reciever.poll_recv(cx)
    }
}