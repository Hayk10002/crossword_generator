use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use crossword_generator::{generator::{CrosswordGenerationRequest, CrosswordGenerator, CrosswordGeneratorSettings}, word::Word};
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("crossword");

    group.bench_function(BenchmarkId::new("Iterative", ""),
        |b| b.iter(||
        {
            let mut generator = CrosswordGenerator::<u8, String>::default();
            generator.settings = CrosswordGeneratorSettings::default();
            generator.words = vec!["Hello", "world", "asdf", "myname", "sesame", "yeeee", "nouyt"].into_iter().map(|s| Word::new(s.to_lowercase(), None)).collect();
            
            let rt = Runtime::new().unwrap();

            rt.block_on(async move
            {
                let mut str = generator.crossword_stream();
                str.request_crossword(CrosswordGenerationRequest::Endless).await;
                while let Some(_) = str.next().await {}
            });
        }));

    group.finish();

}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

// Stream, without &StrT optimistion      188.33 ms