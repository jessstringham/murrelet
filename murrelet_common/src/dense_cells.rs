use std::ops::Add;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellId(pub usize);

impl CellId {
    pub fn i(&self) -> usize {
        self.0
    }
}

impl From<usize> for CellId {
    fn from(value: usize) -> Self {
        CellId(value)
    }
}

impl From<&usize> for CellId {
    fn from(value: &usize) -> Self {
        CellId(*value)
    }
}

impl From<&CellId> for CellId {
    fn from(value: &CellId) -> Self {
        *value
    }
}

#[derive(Debug, Clone)]
pub struct CellMapperDense<T> {
    v: Vec<Option<T>>,
    max_count: usize,
}
impl<T: Clone> CellMapperDense<T> {
    pub fn new(max_count: usize) -> Self {
        assert!(max_count > 0);
        Self {
            v: vec![None; max_count],
            max_count,
        }
    }

    pub fn from_vec(s: Vec<(CellId, T)>) -> Self {
        let mut c = Self::new(s.len());
        for (cell_id, source) in &s {
            c.insert(cell_id, source.clone());
        }
        c
    }

    pub fn resize(&mut self, max_count: usize) {
        if max_count < self.max_count {
            // do nothing
        } else {
            let additional = max_count - self.max_count;
            self.v.extend(std::iter::repeat_n(None, additional));
            self.max_count = max_count;
        }
    }

    fn double_size(&mut self) {
        self.resize(self.max_count * 2);
    }

    pub fn insert<C>(&mut self, cell_id: C, source: T)
    where
        C: Into<CellId>,
    {
        let cell_id: CellId = cell_id.into();
        // assert!(
        //     cell_id.i() >= self.max_count,
        //     "tried to insert something with id {} into {}",
        //     cell_id.i(),
        //     self.max_count
        // );

        while cell_id.i() >= self.max_count {
            // println!("wrong size");
            self.double_size();
        }

        self.v[cell_id.i()] = Some(source);
    }

    pub fn add<C, U>(&mut self, id: C, amount: U, zero: T)
    where
        C: Into<CellId>,
        U: Add<T, Output = T>,
    {
        let cell_id: CellId = id.into();

        while cell_id.i() >= self.max_count {
            // println!("wrong size");

            self.double_size();
        }

        if let Some(ref mut a) = self.v[cell_id.i()] {
            *a = amount.add(a.clone());
        } else {
            self.v[cell_id.i()] = Some(amount + zero)
        }
    }

    // same as before, but quietly fail if no zero
    pub fn add_silent<U>(&mut self, cell_id: &CellId, amount: U)
    where
        U: Add<T, Output = T>,
    {
        while cell_id.i() >= self.max_count {
            // println!("wrong size");

            self.double_size();
        }

        if let Some(ref mut a) = self.v[cell_id.i()] {
            *a = amount.add(a.clone());
        }
    }

    pub fn into_iter(&self) -> impl Iterator<Item = (CellId, &T)> {
        self.v
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_ref().map(|t| (CellId(i), t)))
    }

    pub fn iter(&self) -> impl Iterator<Item = (CellId, &T)> {
        self.v
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_ref().map(|t| (CellId(i), t)))
    }

    pub fn get<C: Into<CellId>>(&self, id: C) -> Option<&T> {
        let cell_id: CellId = id.into();
        self.v.get(cell_id.i()).and_then(|opt| opt.as_ref())
    }

    pub fn get_mut<C: Into<CellId>>(&mut self, id: C) -> Option<&mut T> {
        let cell_id: CellId = id.into();
        self.v.get_mut(cell_id.i()).and_then(|opt| opt.as_mut())
    }

    pub fn get_mut_or_default<C: Into<CellId>>(&mut self, id: C, default: impl Fn() -> T) -> &mut T {
        let cell_id: CellId = id.into();
        let i = cell_id.i();

        if i >= self.v.len() {
            self.v.resize_with(i + 1, || None);
        }

        if self.v[i].is_none() {
            self.v[i] = Some(default());
        }
        self.v.get_mut(cell_id.i()).and_then(|opt| opt.as_mut()).unwrap()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (CellId, &mut T)> {
        self.v
            .iter_mut()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_mut().map(|t| (CellId(i), t)))
    }

    pub fn length(&self) -> usize {
        self.v
            .iter()
            .rposition(|opt| opt.is_some())
            .map(|idx| idx + 1)
            .unwrap_or(0)
    }

    pub fn clear(&mut self) {
        for v in self.v.iter_mut() {
            *v = None;
        }
    }

    pub fn contains<C: Into<CellId>>(&self, cell_id: C) -> bool {
        let cell_id: CellId = cell_id.into();
        self.v.get(cell_id.i()).and_then(|opt| opt.as_ref()).is_some()
    }

    pub fn max_count(&self) -> usize {
        self.max_count
    }

    pub fn remove(&mut self, i: usize) -> Option<T> {
        self.v.get_mut(i).and_then(Option::take)
    }
}
