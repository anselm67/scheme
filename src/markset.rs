
pub struct MarkSet {
    bits: Vec<u64>,
    capacity: usize
}

impl MarkSet {

    pub fn new (capacity: usize) -> Self {
        let size = (capacity + 63) / 64;
        Self { 
            capacity: capacity,
            bits: vec![0; size], 
        }
    }

    pub fn len(&self) -> usize {
        self.capacity
    }

    pub fn mark(&mut self, id: usize) -> bool {
        let block = id / 64;
        let mask = 1 << (id % 64);
        let prev = self.bits[block] & mask;

        self.bits[block] |= mask;
        prev == 0
    }

    pub fn is_marked(&self, id: usize) -> bool {
        let block = id / 64;
        let mask = 1 << (id % 64);
        (self.bits[block] & mask) != 0
    }

    pub fn count(&self) -> usize {
        self.bits.iter().map(|block| block.count_ones() as usize).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markset_boundaries() {
        let mut marks = MarkSet::new(128);
        let boundaries = [0, 63, 64, 127];

        for &idx in &boundaries {
            assert!(!marks.is_marked(idx), "Index {} should be unmarked initially", idx);
            marks.mark(idx);
            assert!(marks.is_marked(idx), "Index {} should be marked after calling mark()", idx);
        }        
    }

    #[test]
    fn test_mark() {
        let mut marks = MarkSet::new(128);
        for i in 0..128 {
            assert!( ! marks.is_marked(i) );
        }
        assert!( ! marks.is_marked(1), "All marks are off at init.");
        assert!(marks.mark(1), "First mark returns true.");
        assert!( ! marks.mark(1), "Second mark returns false.");
    }
}