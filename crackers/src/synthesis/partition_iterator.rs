use std::cmp::max;

pub trait Partition<T> {
    fn partitions(&self) -> PartitionIterator<'_, T>;
}

impl<T> Partition<T> for &[T] {
    fn partitions(&self) -> PartitionIterator<'_, T> {
        PartitionIterator::new(self)
    }
}

impl<T> Partition<T> for Vec<T> {
    fn partitions(&self) -> PartitionIterator<'_, T> {
        PartitionIterator::new(self.as_slice())
    }
}

pub struct PartitionIterator<'a, T> {
    source: &'a [T],
    child: Option<Box<PartitionIterator<'a, T>>>,
    pivot: usize,
}

impl<'a, T> PartitionIterator<'a, T> {
    pub(crate) fn new(source: &'a [T]) -> PartitionIterator<'a, T> {
        PartitionIterator {
            source,
            pivot: source.len(),
            child: None,
        }
    }
}

impl<'a, T> Iterator for PartitionIterator<'a, T> {
    type Item = Vec<&'a [T]>;

    fn next(&mut self) -> Option<Self::Item> {
        // base case
        if self.pivot == 0 {
            return None;
        }
        if let Some(child) = &mut self.child {
            if let Some(next) = child.next() {
                // get the next child value for the current pivot
                let mut result = Vec::with_capacity(next.len() + 1);
                result.extend_from_slice(&next);
                result.push(&self.source[self.pivot..self.source.len()]);
                Some(result)
            } else {
                // if pivot is 0 then this iterator is done
                let pivot = self.pivot - 1;
                self.pivot = pivot;
                self.child = Some(Box::new(PartitionIterator::new(&self.source[0..pivot])));
                self.next()
            }
        } else {
            // base case
            let pivot = max(self.pivot.saturating_sub(1), 0);
            self.pivot = pivot;
            self.child = Some(Box::new(PartitionIterator::new(&self.source[0..pivot])));
            Some(vec![self.source])
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::synthesis::partition_iterator::Partition;

    #[test]
    fn test_empty() {
        let a: Vec<usize> = vec![];
        assert_eq!(a.as_slice().partitions().next(), None)
    }

    #[test]
    fn test_one() {
        let a: Vec<usize> = vec![1];
        let partitions: Vec<Vec<&[usize]>> = a.partitions().collect();
        let expected: Vec<Vec<&[usize]>> = vec![vec![&[1]]];
        assert_eq!(partitions, expected);
    }
    #[test]
    fn test_two() {
        let a: Vec<usize> = vec![1, 2];
        let partitions: Vec<Vec<&[usize]>> = a.partitions().collect();
        let expected: Vec<Vec<&[usize]>> = vec![vec![&[1, 2]], vec![&[1], &[2]]];
        assert_eq!(partitions, expected);
    }

    #[test]
    fn test_three() {
        let a: Vec<usize> = vec![1, 2, 3];
        let partitions: Vec<Vec<&[usize]>> = a.partitions().collect();
        let expected: Vec<Vec<&[usize]>> = vec![
            vec![&[1, 2, 3]],
            vec![&[1, 2], &[3]],
            vec![&[1], &[2], &[3]],
            vec![&[1], &[2, 3]],
        ];
        assert_eq!(partitions, expected);
    }
}
