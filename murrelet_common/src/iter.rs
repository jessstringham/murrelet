struct PrevCurrNextNoLoopIter<'a, T> {
    data: &'a [T], // view of the data
    len: usize,
    index: usize, // where we currently are
}

impl<'a, T> PrevCurrNextNoLoopIter<'a, T> {
    fn new(data: &'a [T]) -> Self {
        Self {
            data,
            index: 0,
            len: data.len(),
        }
    }
}

impl<'a, T> Iterator for PrevCurrNextNoLoopIter<'a, T> {
    type Item = (&'a T, &'a T, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // if we're at the end, return
        if self.index + 2 >= self.len {
            return None;
        }
        let result = (
            &self.data[self.index],
            &self.data[self.index + 1],
            &self.data[self.index + 2],
        );
        self.index += 1;
        Some(result)
    }
}

pub fn prev_curr_next_no_loop_iter<'a, T>(
    v: &'a [T],
) -> Box<dyn Iterator<Item = (&'a T, &'a T, &'a T)> + 'a> {
    let iter = PrevCurrNextNoLoopIter::new(v);
    Box::new(iter)
}

struct PrevCurrNextWithLoopIter<'a, T> {
    data: &'a [T], // view of the data
    len: usize,
    index: usize, // where we currently are
}

impl<'a, T> PrevCurrNextWithLoopIter<'a, T> {
    fn new(data: &'a [T]) -> Self {
        Self {
            data,
            len: data.len(),
            index: 0,
        }
    }
}

impl<'a, T> Iterator for PrevCurrNextWithLoopIter<'a, T> {
    type Item = (&'a T, &'a T, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // if we're at the end, return
        if self.index >= self.len {
            return None;
        }

        if self.index + 2 == self.len {
            let result = (
                &self.data[self.index],
                &self.data[self.index + 1],
                &self.data[0],
            );
            self.index += 1;
            return Some(result);
        }

        if self.index + 1 == self.len {
            let result = (&self.data[self.index], &self.data[0], &self.data[1]);
            self.index += 1;
            return Some(result);
        }

        let result = (
            &self.data[self.index],
            &self.data[self.index + 1],
            &self.data[self.index + 2],
        );
        self.index += 1;
        Some(result)
    }
}

pub fn prev_curr_next_loop_iter<'a, T>(
    v: &'a [T],
) -> Box<dyn Iterator<Item = (&'a T, &'a T, &'a T)> + 'a> {
    let iter = PrevCurrNextWithLoopIter::new(v);
    Box::new(iter)
}

struct CurrNextNoLoopIter<'a, T> {
    data: &'a [T], // view of the data
    len: usize,
    index: usize, // where we currently are
}

impl<'a, T> CurrNextNoLoopIter<'a, T> {
    fn new(data: &'a [T]) -> Self {
        Self {
            data,
            len: data.len(),
            index: 0,
        }
    }
}

impl<'a, T> Iterator for CurrNextNoLoopIter<'a, T> {
    type Item = (&'a T, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // if we're at the end, return
        if self.index + 1 >= self.len {
            return None;
        }
        let result = (&self.data[self.index], &self.data[self.index + 1]);
        self.index += 1;
        Some(result)
    }
}

pub fn curr_next_no_loop_iter<'a, T>(v: &'a [T]) -> Box<dyn Iterator<Item = (&'a T, &'a T)> + 'a> {
    let iter = CurrNextNoLoopIter::new(v);
    Box::new(iter)
}

struct CurrNextWithLoopIter<'a, T> {
    data: &'a [T], // view of the data
    len: usize,
    index: usize, // where we currently are
}

impl<'a, T> CurrNextWithLoopIter<'a, T> {
    fn new(data: &'a [T]) -> Self {
        Self {
            data,
            index: 0,
            len: data.len(),
        }
    }
}

impl<'a, T> Iterator for CurrNextWithLoopIter<'a, T> {
    type Item = (&'a T, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // if we're at the end, return
        if self.index >= self.len {
            return None;
        }

        if self.index + 1 == self.len {
            let result = (&self.data[self.index], &self.data[0]);
            self.index += 1;
            return Some(result);
        }

        let result = (&self.data[self.index], &self.data[self.index + 1]);
        self.index += 1;
        Some(result)
    }
}

impl<'a, T> ExactSizeIterator for CurrNextWithLoopIter<'a, T> {
    fn len(&self) -> usize {
        self.len
    }
}

pub fn curr_next_loop_iter<'a, T>(v: &'a [T]) -> Box<dyn Iterator<Item = (&'a T, &'a T)> + 'a> {
    let iter = CurrNextWithLoopIter::new(v);
    Box::new(iter)
}
