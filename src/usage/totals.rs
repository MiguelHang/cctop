#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Totals {
    pub(crate) memory: Option<u64>,
    pub(crate) memory_partial: bool,
    pub(crate) cpu: Option<f64>,
}

impl Totals {
    pub(super) fn of(memory: Option<u64>, cpu: Option<f64>) -> Self {
        Self { memory, memory_partial: memory.is_none(), cpu }
    }

    pub(super) fn add(self, other: Self) -> Self {
        Self {
            memory: add(self.memory, other.memory, u64::saturating_add),
            memory_partial: self.memory_partial || other.memory_partial,
            cpu: add(self.cpu, other.cpu, |a, b| a + b),
        }
    }

    pub(super) fn sum(totals: impl IntoIterator<Item = Self>) -> Self {
        totals.into_iter().fold(Self::default(), Self::add)
    }
}

fn add<T>(a: Option<T>, b: Option<T>, plus: impl FnOnce(T, T) -> T) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(plus(a, b)),
        (a, b) => a.or(b),
    }
}
