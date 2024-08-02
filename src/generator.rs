use std::{collections::BTreeSet, future::Future, pin::Pin, sync::Arc, task::{Context, Poll}};

use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{sync::{mpsc::{self, Receiver, Sender}, Mutex}, task};
use tokio_stream::Stream;
use itertools::Itertools;

use crate::{crossword::{Crossword, CrosswordSettings, WordCompatibilitySettings}, traits::{CrosswordChar, CrosswordString}, word::Word};

const MAX_CONCURRENT_TASK_COUNT: usize = 10;

/// Represents all settings for a [generator](CrosswordGenerator).
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize, Hash)]
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
///     let mut generator = CrosswordGenerator::<u8, String>::default();
///     generator.settings = CrosswordGeneratorSettings::default();
///     generator.words = vec!["Hello", "world"].into_iter().map(|s| Word::new(s.to_lowercase(), None)).collect();
///      
///     let str = generator.crossword_stream(|w| String::from_utf8(w.to_owned()).unwrap());
///     str.request_crossword(CrosswordGenerationRequest::Count(2)).await;
///     str.request_crossword(CrosswordGenerationRequest::Stop).await;
///     let crosswords: Vec<Crossword<u8, String>> = str.collect().await;
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
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize, Hash)]
pub struct CrosswordGenerator<CharT: CrosswordChar, StrT: CrosswordString<CharT>>
{
    pub words: BTreeSet<Word<CharT, StrT>>,
    pub settings: CrosswordGeneratorSettings,
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> CrosswordGenerator<CharT, StrT>
{
    /// Takes a function to convert from &\[CharT\] to StrT, because the generator generates crosswords with words with type &\[CharT\] to prevent unnecessary copying
    /// Slow, but crosswords are pretty much random.
    /// If you need fast generation, check [crossword_stream_sorted](CrosswordGenerator::crossword_stream_sorted).

    pub fn crossword_stream_randomized<F>(&self, convert_f: F) -> CrosswordStream<CharT, StrT> where
        F: Fn(&[CharT]) -> StrT,
        F: Clone + Send + Sync + 'static
    {  

        let gen = self.clone();
        
        let gen_func = move |rr: Receiver<CrosswordGenerationRequest>, cs: Sender<Crossword<CharT, StrT>>| async move
        {
            // creating separate tasks for each word permutation
            let rr = Arc::new(Mutex::new(rr));
            let current_request = Arc::new(Mutex::new(CrosswordGenerationRequest::Count(0)));
            let created_crosswords = Arc::<Mutex<BTreeSet<_>>>::new(Mutex::new(BTreeSet::new()));

            let mut tasks = FuturesUnordered::new();
            
            for mut ws in gen.words.iter().enumerate().permutations(gen.words.len())
            {
                //for some randomness
                ws.rotate_right(2);

                //maintaining the number of currently running tasks under MAX_CONCURRENT_TASK_COUNT
                if tasks.len() >= MAX_CONCURRENT_TASK_COUNT
                {
                    tasks.next().await;
                }
                
                let settings = gen.settings.clone();
                let receiver = rr.clone(); 
                let cs = cs.clone();
                let cr = current_request.clone();
                let ws = ws.into_iter().map(|(_, w)| w.clone()).collect::<Vec<_>>();
                let ccs = created_crosswords.clone();
                let cfr = convert_f.clone();

                //creating and spawning the task
                tasks.push(tokio::spawn(async move 
                {
                    let mut cc = Crossword::new(settings.word_compatibility_settings.clone());
                    let ws = ws.iter().map(|w| Word::<CharT, Arc<[CharT]>>::new(w.value.as_ref().to_owned().into(), w.dir.clone())).collect::<Vec<_>>();
                    CrosswordGenerator::<CharT, StrT>::randomized_generator_impl(&settings, receiver, &cs, cr, &mut cc, &ws, &mut 0, ccs, &cfr).await; 
                }));

                if let CrosswordGenerationRequest::Stop = *current_request.lock().await { break; }
            };

            while let Some(_) = tasks.next().await {}       
        };

        CrosswordStream::new(gen_func)
    }

    #[async_recursion]
    async fn randomized_generator_impl<F>(gen_settings: &CrosswordGeneratorSettings, rr: Arc<Mutex<Receiver<CrosswordGenerationRequest>>>, cs: &Sender<Crossword<CharT, StrT>>, current_request: Arc<Mutex<CrosswordGenerationRequest>>, current_crossword: &mut Crossword<CharT, Arc<[CharT]>>, words: &Vec<Word<CharT, Arc<[CharT]>>>, current_word_ind: &mut usize, created_crosswords: Arc<Mutex<BTreeSet<Crossword<CharT, Arc<[CharT]>>>>>, convert_f: &F) where  
        F: Fn(&[CharT]) -> StrT,
        F: Send + Sync + 'static
    {
        if !gen_settings.crossword_settings.check_nonrecoverables_constraints(current_crossword) 
        {
            return; 
        }
        
        if *current_word_ind == words.len()
        {
            if gen_settings.crossword_settings.check_recoverable_constraints(current_crossword) 
            {
                if created_crosswords.lock().await.insert(current_crossword.clone())
                {
                    let mut current_request = current_request.lock().await;
                    while let CrosswordGenerationRequest::Count(0) = *current_request
                    {
                        match rr.lock().await.recv().await
                        {
                            None => { *current_request = CrosswordGenerationRequest::Stop; },
                            Some(req) => *current_request = req
                        }
                    }
        
                    if let CrosswordGenerationRequest::Stop = *current_request { return; }

                    cs.send(current_crossword.clone().convert_to(|w| convert_f(w.as_ref()))).await.unwrap();
                    if let CrosswordGenerationRequest::Count(count) = *current_request { *current_request = CrosswordGenerationRequest::Count(count - 1) }
                }
            }
            return;
        }
        let current_word = &words[*current_word_ind];

        *current_word_ind += 1;

        for step in current_crossword.calculate_possible_ways_to_add_word(current_word).iter()
        {
            current_crossword.add_word(step.clone()).unwrap();

            CrosswordGenerator::randomized_generator_impl(gen_settings, rr.clone(), cs, current_request.clone(), current_crossword, words, current_word_ind, created_crosswords.clone(), convert_f).await;

            if let CrosswordGenerationRequest::Stop = *current_request.lock().await { return; }
            
            //let to_remove: Vec<Crossword<CharT, &[CharT]>> = full_created_crossword_bases.iter().filter_map(|cw| cw.contains_crossword(current_crossword).then_some(cw.clone())).collect();
            //to_remove.into_iter().for_each(|cw| {full_created_crossword_bases.remove(&cw);});
            
            //full_created_crossword_bases.insert(current_crossword.clone());

            current_crossword.remove_word(&step.value);

        }
        
        *current_word_ind -= 1;

    }


    /// Takes a function to convert from &\[CharT\] to StrT, because the generator generates crosswords with words with type &\[CharT\] to prevent unnecessary copying
    /// Fast, but crosswords in a non random order, consecutive crosswords are pretty similar.
    /// If you need randomized results, check [crossword_stream_randomized](CrosswordGenerator::crossword_stream_randomized).
    pub fn crossword_stream_sorted<F>(&self, convert_f: F) -> CrosswordStream<CharT, StrT> where
        F: Fn(&[CharT]) -> StrT,
        F: Send + Sync + 'static
    {  
        let gen = self.clone();
        
        let gen_func = move |mut rr: Receiver<CrosswordGenerationRequest>, cs: Sender<Crossword<CharT, StrT>>| async move
        {

            let mut current_request = CrosswordGenerationRequest::Count(0);
            let mut current_crossword = Crossword::new(gen.settings.word_compatibility_settings.clone());
            let mut full_created_crossword_bases = BTreeSet::new();
            let remaine_words = gen.words.iter().map(|w| Word::<CharT, &[CharT]>::new(w.value.as_ref(), w.dir.clone())).collect();
            CrosswordGenerator::<CharT, StrT>::sorted_generator_impl(&gen.settings, &mut rr, &cs, &mut current_request, &mut current_crossword, &remaine_words, &mut full_created_crossword_bases, &convert_f).await
               
        };

        CrosswordStream::new(gen_func)
    }

    #[async_recursion]
    async fn sorted_generator_impl<'a, F>(gen_settings: &CrosswordGeneratorSettings, rr: &mut Receiver<CrosswordGenerationRequest>, cs: &Sender<Crossword<CharT, StrT>>, current_request: &mut CrosswordGenerationRequest, current_crossword: &mut Crossword<CharT, &'a [CharT]>, remained_words: &BTreeSet<Word<CharT, &'a [CharT]>>, full_created_crossword_bases: &mut BTreeSet<Crossword<CharT, &'a [CharT]>>, convert_f: &F) where  
        F: Fn(&'a [CharT]) -> StrT,
        F: Send + Sync + 'static
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

                cs.send(current_crossword.clone().convert_to(|w| convert_f(w))).await.unwrap();
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

                CrosswordGenerator::sorted_generator_impl(gen_settings, rr, cs, current_request, current_crossword, &new_remained_words, full_created_crossword_bases, convert_f).await;

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
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize, Hash)]
pub enum CrosswordGenerationRequest
{
    /// Request to stop the crossword generation.
    #[default]
    Stop,
    /// Request for some count of crosswords to generate.
    Count(usize),
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