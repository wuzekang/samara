use criterion::{Criterion, criterion_group, criterion_main};
use samara_signals::*;

#[derive(Clone, Copy)]
enum SignalOrComputed {
    Signal(Signal<i32>),
    Computed(Computed<i32>),
}
impl SignalOrComputed {
    fn get(&self) -> i32 {
        match self {
            SignalOrComputed::Signal(s) => s.get(),
            SignalOrComputed::Computed(c) => c.get(),
        }
    }
}
fn criterion_benchmark(c: &mut Criterion) {
    for w in [1, 10, 100, 1000] {
        for h in [1, 10, 100, 1000] {
            c.bench_function(&format!("{w} * {h}"), |b| {
                let mut src = signal(1);
                for _ in 0..w {
                    let mut last = SignalOrComputed::Signal(src);
                    for _ in 0..h {
                        let prev = last;
                        last = SignalOrComputed::Computed(memo(move || prev.get() + 1));
                    }
                    effect(move || {
                        let _ = last.get();
                    });
                }

                b.iter(|| {
                    src += 1
                });

                cleanup();
            });
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
