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

fn main() {
    let src = signal(1);
    for _ in 0..1000 {
        let mut last = SignalOrComputed::Signal(src);
        for _ in 0..1000 {
            let prev = last;
            last = SignalOrComputed::Computed(memo(move || prev.get() + 1));
        }
        effect(move || {
            let _ = last.get();
        });
    }
    for _ in 0..1000 {
        src.set(src.get() + 1);
    }
}
